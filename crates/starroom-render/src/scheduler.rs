//! M13 preview-pyramid, tile scheduling and bounded cache primitives.
//!
//! This module has no image math. It owns request identity, zoom-aware level selection,
//! overlap planning, cancellation and bounded LRU bookkeeping so CPU and GPU renderers can
//! execute the same logical graph without delivering stale frames to the desktop UI.

use crate::{PixelRect, RenderTile, plan_tiles};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};

pub const PREVIEW_LEVEL_EDGES: [u32; 4] = [512, 1024, 2048, 4096];
pub const DEFAULT_TILE_EDGE: u32 = 512;
pub const DEFAULT_RAM_BUDGET_BYTES: usize = 512 * 1024 * 1024;
pub const DEFAULT_VRAM_BUDGET_BYTES: usize = 768 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PreviewLevel {
    Edge512,
    Edge1024,
    Edge2048,
    Edge4096,
}

impl PreviewLevel {
    pub const fn max_edge(self) -> u32 {
        match self {
            Self::Edge512 => 512,
            Self::Edge1024 => 1024,
            Self::Edge2048 => 2048,
            Self::Edge4096 => 4096,
        }
    }

    /// Selects the smallest supported pyramid level that covers the requested on-screen edge.
    /// Values above 4096 deliberately stay at the coarsest bounded preview contract; export
    /// always reopens the immutable full-resolution source outside this scheduler.
    pub fn for_requested_edge(requested_edge: u32) -> Self {
        match requested_edge.max(1) {
            1..=512 => Self::Edge512,
            513..=1024 => Self::Edge1024,
            1025..=2048 => Self::Edge2048,
            _ => Self::Edge4096,
        }
    }

    pub fn dimensions_for(self, source_width: u32, source_height: u32) -> (u32, u32) {
        if source_width == 0 || source_height == 0 {
            return (0, 0);
        }
        let longest = source_width.max(source_height);
        if longest <= self.max_edge() {
            return (source_width, source_height);
        }
        let scale = self.max_edge() as f64 / f64::from(longest);
        (
            (f64::from(source_width) * scale).round().max(1.0) as u32,
            (f64::from(source_height) * scale).round().max(1.0) as u32,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Viewport {
    pub rect: PixelRect,
}

impl Viewport {
    pub const fn full(width: u32, height: u32) -> Self {
        Self {
            rect: PixelRect {
                x: 0,
                y: 0,
                width,
                height,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TilePriority {
    VisibleViewport,
    ViewportNeighborhood,
    RemainingImage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TileIdentity {
    pub source_identity: String,
    pub graph_identity: String,
    pub level: PreviewLevel,
    pub output: PixelRect,
    pub generation: u64,
}

impl TileIdentity {
    pub fn cache_key(&self) -> String {
        let mut hash = Sha256::new();
        hash.update(self.source_identity.as_bytes());
        hash.update([0]);
        hash.update(self.graph_identity.as_bytes());
        hash.update([0]);
        hash.update(self.level.max_edge().to_le_bytes());
        hash.update(self.output.x.to_le_bytes());
        hash.update(self.output.y.to_le_bytes());
        hash.update(self.output.width.to_le_bytes());
        hash.update(self.output.height.to_le_bytes());
        format!("{:x}", hash.finalize())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduledTile {
    pub identity: TileIdentity,
    pub tile: RenderTile,
    pub priority: TilePriority,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderJob {
    pub generation: u64,
    pub level: PreviewLevel,
    pub preview_size: (u32, u32),
    pub tiles: Vec<ScheduledTile>,
}

impl RenderJob {
    /// Identity for the encoded progressive-preview payload. Tile outputs still use their own
    /// identities; this entry lets the desktop reuse a completed current preview while a future
    /// viewport refinement request is scheduled.
    pub fn full_frame_tile(&self) -> ScheduledTile {
        let output = PixelRect {
            x: 0,
            y: 0,
            width: self.preview_size.0,
            height: self.preview_size.1,
        };
        ScheduledTile {
            identity: TileIdentity {
                source_identity: self
                    .tiles
                    .first()
                    .map(|tile| tile.identity.source_identity.clone())
                    .unwrap_or_default(),
                graph_identity: self
                    .tiles
                    .first()
                    .map(|tile| tile.identity.graph_identity.clone())
                    .unwrap_or_default(),
                level: self.level,
                output,
                generation: self.generation,
            },
            tile: RenderTile {
                output,
                input_with_halo: output,
            },
            priority: TilePriority::VisibleViewport,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryBudget {
    pub ram_bytes: usize,
    pub vram_bytes: usize,
}

impl Default for MemoryBudget {
    fn default() -> Self {
        Self {
            ram_bytes: DEFAULT_RAM_BUDGET_BYTES,
            vram_bytes: DEFAULT_VRAM_BUDGET_BYTES,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchedulerStatus {
    pub active_generation: u64,
    pub cached_entries: usize,
    pub cached_ram_bytes: usize,
    pub cached_vram_bytes: usize,
    pub dropped_stale_jobs: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Completion {
    Stored,
    Stale,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    bytes: Vec<u8>,
    vram_bytes: usize,
}

/// Bounded LRU cache shared by preview frame and tile callers. Stored bytes may be encoded
/// preview frames or GPU upload material; logical tile identity prevents a graph-state change
/// from reusing old pixels.
#[derive(Debug)]
pub struct RenderScheduler {
    budget: MemoryBudget,
    active_generation: u64,
    cache: HashMap<String, CacheEntry>,
    lru: VecDeque<String>,
    cached_ram_bytes: usize,
    cached_vram_bytes: usize,
    dropped_stale_jobs: u64,
}

impl Default for RenderScheduler {
    fn default() -> Self {
        Self::new(MemoryBudget::default())
    }
}

impl RenderScheduler {
    pub fn new(budget: MemoryBudget) -> Self {
        Self {
            budget: MemoryBudget {
                ram_bytes: budget.ram_bytes.max(1),
                vram_bytes: budget.vram_bytes.max(1),
            },
            active_generation: 0,
            cache: HashMap::new(),
            lru: VecDeque::new(),
            cached_ram_bytes: 0,
            cached_vram_bytes: 0,
            dropped_stale_jobs: 0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn schedule_preview(
        &mut self,
        source_identity: impl Into<String>,
        graph_identity: impl Into<String>,
        source_width: u32,
        source_height: u32,
        requested_edge: u32,
        viewport: Viewport,
        tile_edge: u32,
        halo: u32,
    ) -> RenderJob {
        self.active_generation = self.active_generation.wrapping_add(1).max(1);
        let level = PreviewLevel::for_requested_edge(requested_edge);
        let (width, height) = level.dimensions_for(source_width, source_height);
        let level_viewport = scale_viewport(viewport, source_width, source_height, width, height);
        let source_identity = source_identity.into();
        let graph_identity = graph_identity.into();
        let mut tiles: Vec<ScheduledTile> = plan_tiles(width, height, tile_edge, halo)
            .into_iter()
            .map(|tile| ScheduledTile {
                priority: classify_priority(tile.output, level_viewport.rect),
                identity: TileIdentity {
                    source_identity: source_identity.clone(),
                    graph_identity: graph_identity.clone(),
                    level,
                    output: tile.output,
                    generation: self.active_generation,
                },
                tile,
            })
            .collect();
        tiles.sort_by_key(|tile| (tile.priority, tile.tile.output.y, tile.tile.output.x));
        RenderJob {
            generation: self.active_generation,
            level,
            preview_size: (width, height),
            tiles,
        }
    }

    pub const fn active_generation(&self) -> u64 {
        self.active_generation
    }

    pub fn is_current(&self, job: &RenderJob) -> bool {
        job.generation == self.active_generation
    }

    pub fn complete_tile(
        &mut self,
        tile: &ScheduledTile,
        bytes: Vec<u8>,
        vram_bytes: usize,
    ) -> Completion {
        if tile.identity.generation != self.active_generation {
            self.dropped_stale_jobs = self.dropped_stale_jobs.saturating_add(1);
            return Completion::Stale;
        }
        let key = tile.identity.cache_key();
        self.remove_entry(&key);
        self.cached_ram_bytes = self.cached_ram_bytes.saturating_add(bytes.len());
        self.cached_vram_bytes = self.cached_vram_bytes.saturating_add(vram_bytes);
        self.cache
            .insert(key.clone(), CacheEntry { bytes, vram_bytes });
        self.touch(&key);
        self.evict_to_budget();
        Completion::Stored
    }

    pub fn cached_tile(&mut self, identity: &TileIdentity) -> Option<Vec<u8>> {
        let key = identity.cache_key();
        let bytes = self.cache.get(&key)?.bytes.clone();
        self.touch(&key);
        Some(bytes)
    }

    pub fn status(&self) -> SchedulerStatus {
        SchedulerStatus {
            active_generation: self.active_generation,
            cached_entries: self.cache.len(),
            cached_ram_bytes: self.cached_ram_bytes,
            cached_vram_bytes: self.cached_vram_bytes,
            dropped_stale_jobs: self.dropped_stale_jobs,
        }
    }

    fn touch(&mut self, key: &str) {
        self.lru.retain(|entry| entry != key);
        self.lru.push_back(key.to_owned());
    }

    fn remove_entry(&mut self, key: &str) {
        if let Some(entry) = self.cache.remove(key) {
            self.cached_ram_bytes = self.cached_ram_bytes.saturating_sub(entry.bytes.len());
            self.cached_vram_bytes = self.cached_vram_bytes.saturating_sub(entry.vram_bytes);
        }
        self.lru.retain(|entry| entry != key);
    }

    fn evict_to_budget(&mut self) {
        while (self.cached_ram_bytes > self.budget.ram_bytes
            || self.cached_vram_bytes > self.budget.vram_bytes)
            && self.cache.len() > 1
        {
            let Some(oldest) = self.lru.pop_front() else {
                break;
            };
            if let Some(entry) = self.cache.remove(&oldest) {
                self.cached_ram_bytes = self.cached_ram_bytes.saturating_sub(entry.bytes.len());
                self.cached_vram_bytes = self.cached_vram_bytes.saturating_sub(entry.vram_bytes);
            }
        }
    }
}

fn scale_viewport(
    viewport: Viewport,
    source_width: u32,
    source_height: u32,
    preview_width: u32,
    preview_height: u32,
) -> Viewport {
    if source_width == 0 || source_height == 0 {
        return Viewport::full(preview_width, preview_height);
    }
    let scale_x = preview_width as f64 / f64::from(source_width);
    let scale_y = preview_height as f64 / f64::from(source_height);
    let x = (f64::from(viewport.rect.x) * scale_x).floor().max(0.0) as u32;
    let y = (f64::from(viewport.rect.y) * scale_y).floor().max(0.0) as u32;
    let right = (f64::from(viewport.rect.x.saturating_add(viewport.rect.width)) * scale_x)
        .ceil()
        .min(f64::from(preview_width)) as u32;
    let bottom = (f64::from(viewport.rect.y.saturating_add(viewport.rect.height)) * scale_y)
        .ceil()
        .min(f64::from(preview_height)) as u32;
    Viewport {
        rect: PixelRect {
            x: x.min(preview_width),
            y: y.min(preview_height),
            width: right.saturating_sub(x.min(right)),
            height: bottom.saturating_sub(y.min(bottom)),
        },
    }
}

fn classify_priority(tile: PixelRect, viewport: PixelRect) -> TilePriority {
    if intersects(tile, viewport) {
        return TilePriority::VisibleViewport;
    }
    let neighborhood = PixelRect {
        x: viewport.x.saturating_sub(viewport.width),
        y: viewport.y.saturating_sub(viewport.height),
        width: viewport.width.saturating_mul(3),
        height: viewport.height.saturating_mul(3),
    };
    if intersects(tile, neighborhood) {
        TilePriority::ViewportNeighborhood
    } else {
        TilePriority::RemainingImage
    }
}

fn intersects(left: PixelRect, right: PixelRect) -> bool {
    let left_right = left.x.saturating_add(left.width);
    let left_bottom = left.y.saturating_add(left.height);
    let right_right = right.x.saturating_add(right.width);
    let right_bottom = right.y.saturating_add(right.height);
    left.x < right_right && right.x < left_right && left.y < right_bottom && right.y < left_bottom
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scheduler() -> RenderScheduler {
        RenderScheduler::new(MemoryBudget {
            ram_bytes: 12,
            vram_bytes: 12,
        })
    }

    #[test]
    fn pyramid_is_zoom_aware_and_bounded_for_100mp_sources() {
        assert_eq!(PreviewLevel::for_requested_edge(400), PreviewLevel::Edge512);
        assert_eq!(
            PreviewLevel::for_requested_edge(1800),
            PreviewLevel::Edge2048
        );
        assert_eq!(
            PreviewLevel::for_requested_edge(9_000),
            PreviewLevel::Edge4096
        );
        assert_eq!(
            PreviewLevel::Edge4096.dimensions_for(11_547, 8_660),
            (4096, 3072)
        );
    }

    #[test]
    fn tiles_have_halo_and_visible_viewport_runs_first() {
        let mut scheduler = scheduler();
        let job = scheduler.schedule_preview(
            "60mp",
            "graph",
            9_000,
            6_000,
            4096,
            Viewport {
                rect: PixelRect {
                    x: 3_000,
                    y: 2_000,
                    width: 1_000,
                    height: 900,
                },
            },
            512,
            32,
        );
        assert!(job.tiles.len() > 20);
        assert_eq!(
            job.tiles.first().expect("tile").priority,
            TilePriority::VisibleViewport
        );
        assert!(
            job.tiles
                .iter()
                .all(|tile| tile.tile.input_with_halo.width >= tile.tile.output.width)
        );
        assert!(
            job.tiles
                .iter()
                .all(|tile| tile.tile.input_with_halo.height >= tile.tile.output.height)
        );
    }

    #[test]
    fn new_generation_cancels_and_rejects_stale_output() {
        let mut scheduler = scheduler();
        let old = scheduler.schedule_preview(
            "source",
            "first",
            1000,
            1000,
            1024,
            Viewport::full(1000, 1000),
            512,
            0,
        );
        let fresh = scheduler.schedule_preview(
            "source",
            "second",
            1000,
            1000,
            1024,
            Viewport::full(1000, 1000),
            512,
            0,
        );
        assert!(!scheduler.is_current(&old));
        assert!(scheduler.is_current(&fresh));
        assert_eq!(
            scheduler.complete_tile(&old.tiles[0], vec![1, 2, 3], 0),
            Completion::Stale
        );
        assert_eq!(scheduler.status().dropped_stale_jobs, 1);
    }

    #[test]
    fn lru_evicts_old_entries_under_ram_or_vram_budget() {
        let mut scheduler = scheduler();
        let first = scheduler.schedule_preview(
            "source",
            "one",
            512,
            512,
            512,
            Viewport::full(512, 512),
            512,
            0,
        );
        assert_eq!(
            scheduler.complete_tile(&first.tiles[0], vec![1; 8], 8),
            Completion::Stored
        );
        let second = scheduler.schedule_preview(
            "source",
            "two",
            512,
            512,
            512,
            Viewport::full(512, 512),
            512,
            0,
        );
        assert_eq!(
            scheduler.complete_tile(&second.tiles[0], vec![2; 8], 8),
            Completion::Stored
        );
        let status = scheduler.status();
        assert_eq!(status.cached_entries, 1);
        assert!(scheduler.cached_tile(&first.tiles[0].identity).is_none());
        assert_eq!(
            scheduler.cached_tile(&second.tiles[0].identity),
            Some(vec![2; 8])
        );
    }

    #[test]
    fn stable_tile_key_changes_for_graph_but_not_generation() {
        let mut scheduler = scheduler();
        let first = scheduler.schedule_preview(
            "source",
            "graph",
            512,
            512,
            512,
            Viewport::full(512, 512),
            512,
            0,
        );
        let second = scheduler.schedule_preview(
            "source",
            "graph",
            512,
            512,
            512,
            Viewport::full(512, 512),
            512,
            0,
        );
        let third = scheduler.schedule_preview(
            "source",
            "changed",
            512,
            512,
            512,
            Viewport::full(512, 512),
            512,
            0,
        );
        assert_eq!(
            first.tiles[0].identity.cache_key(),
            second.tiles[0].identity.cache_key()
        );
        assert_ne!(
            second.tiles[0].identity.cache_key(),
            third.tiles[0].identity.cache_key()
        );
    }

    #[test]
    fn deterministic_24_to_100mp_plans_report_bounded_scheduler_cost() {
        let cases = [
            ("24mp", 6_000, 4_000),
            ("45mp", 8_256, 5_504),
            ("60mp", 9_500, 6_300),
            ("100mp", 11_547, 8_660),
        ];
        for (name, width, height) in cases {
            let started = std::time::Instant::now();
            let mut scheduler = RenderScheduler::default();
            let job = scheduler.schedule_preview(
                name,
                "benchmark-graph",
                width,
                height,
                4_096,
                Viewport::full(width, height),
                DEFAULT_TILE_EDGE,
                32,
            );
            let milliseconds = started.elapsed().as_millis();
            println!(
                "M13_PLAN_BENCH case={name} tiles={} milliseconds={milliseconds}",
                job.tiles.len()
            );
            assert!(!job.tiles.is_empty());
            // Planning does not inspect source pixels; this protects viewport feedback from a
            // pathological allocation/sort regression without claiming an image-render timing.
            assert!(milliseconds < 250, "{name} tile planning exceeded 250ms");
        }
    }
}
