//! Render-graph primitives for Starroom.
//! The graph is backend-neutral: CPU reference, wgpu preview and tiled export must share the
//! same logical stage order and invalidation rules.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StageId {
    Decode,
    InputTransform,
    WhiteBalance,
    Exposure,
    Tone,
    Curve,
    ColorMixer,
    ColorGrading,
    Layers,
    Detail,
    Optics,
    Geometry,
    DisplayTransform,
    Export,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageNode {
    pub id: StageId,
    pub dependencies: Vec<StageId>,
    pub halo_pixels: u32,
    pub tile_safe: bool,
    pub cpu_supported: bool,
    pub gpu_supported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderGraph {
    pub stages: Vec<StageNode>,
}

impl Default for RenderGraph {
    fn default() -> Self {
        use StageId::*;
        let linear = [
            Decode,
            InputTransform,
            WhiteBalance,
            Exposure,
            Tone,
            Curve,
            ColorMixer,
            ColorGrading,
            Layers,
            Detail,
            Optics,
            Geometry,
            DisplayTransform,
            Export,
        ];

        let mut stages = Vec::with_capacity(linear.len());
        for (index, id) in linear.into_iter().enumerate() {
            let dependencies = if index == 0 { Vec::new() } else { vec![linear[index - 1]] };
            let (halo_pixels, tile_safe) = match id {
                Detail => (32, true),
                Optics | Geometry => (4, true),
                _ => (0, true),
            };
            stages.push(StageNode {
                id,
                dependencies,
                halo_pixels,
                tile_safe,
                cpu_supported: true,
                gpu_supported: !matches!(id, Decode | Export),
            });
        }
        Self { stages }
    }
}

impl RenderGraph {
    pub fn node(&self, id: StageId) -> Option<&StageNode> {
        self.stages.iter().find(|stage| stage.id == id)
    }

    pub fn validate(&self) -> Result<(), GraphError> {
        let ids: BTreeSet<StageId> = self.stages.iter().map(|stage| stage.id).collect();
        if ids.len() != self.stages.len() {
            return Err(GraphError::DuplicateStage);
        }
        for stage in &self.stages {
            for dependency in &stage.dependencies {
                if !ids.contains(dependency) {
                    return Err(GraphError::MissingDependency {
                        stage: stage.id,
                        dependency: *dependency,
                    });
                }
            }
        }

        let mut indegree: BTreeMap<StageId, usize> = ids.iter().map(|id| (*id, 0)).collect();
        let mut downstream: BTreeMap<StageId, Vec<StageId>> = BTreeMap::new();
        for stage in &self.stages {
            for dependency in &stage.dependencies {
                *indegree.entry(stage.id).or_default() += 1;
                downstream.entry(*dependency).or_default().push(stage.id);
            }
        }
        let mut queue: VecDeque<StageId> = indegree
            .iter()
            .filter_map(|(id, degree)| (*degree == 0).then_some(*id))
            .collect();
        let mut visited = 0usize;
        while let Some(stage) = queue.pop_front() {
            visited += 1;
            for next in downstream.get(&stage).into_iter().flatten() {
                let degree = indegree.get_mut(next).expect("known stage");
                *degree -= 1;
                if *degree == 0 {
                    queue.push_back(*next);
                }
            }
        }
        if visited != self.stages.len() {
            return Err(GraphError::Cycle);
        }
        Ok(())
    }

    /// Returns the changed stage plus every transitively dependent downstream stage.
    pub fn invalidate_from(&self, changed: StageId) -> BTreeSet<StageId> {
        let mut downstream: BTreeMap<StageId, Vec<StageId>> = BTreeMap::new();
        for stage in &self.stages {
            for dependency in &stage.dependencies {
                downstream.entry(*dependency).or_default().push(stage.id);
            }
        }
        let mut invalid = BTreeSet::from([changed]);
        let mut queue = VecDeque::from([changed]);
        while let Some(current) = queue.pop_front() {
            for next in downstream.get(&current).into_iter().flatten() {
                if invalid.insert(*next) {
                    queue.push_back(*next);
                }
            }
        }
        invalid
    }

    pub fn maximum_halo(&self) -> u32 {
        self.stages.iter().map(|stage| stage.halo_pixels).max().unwrap_or(0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphError {
    DuplicateStage,
    MissingDependency { stage: StageId, dependency: StageId },
    Cycle,
}

/// Stable cache key derived from immutable source identity, stage parameters and upstream keys.
pub fn stage_cache_key(
    stage: StageId,
    source_hash: &str,
    parameter_json: &str,
    upstream_keys: &[String],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{stage:?}\0{source_hash}\0{parameter_json}\0"));
    for key in upstream_keys {
        hasher.update(key.as_bytes());
        hasher.update([0]);
    }
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_graph_is_valid_and_acyclic() {
        let graph = RenderGraph::default();
        assert_eq!(graph.validate(), Ok(()));
        assert!(graph.maximum_halo() >= 32);
    }

    #[test]
    fn changing_color_mixer_does_not_invalidate_decode_or_white_balance() {
        let graph = RenderGraph::default();
        let invalid = graph.invalidate_from(StageId::ColorMixer);
        assert!(invalid.contains(&StageId::ColorMixer));
        assert!(invalid.contains(&StageId::DisplayTransform));
        assert!(invalid.contains(&StageId::Export));
        assert!(!invalid.contains(&StageId::Decode));
        assert!(!invalid.contains(&StageId::WhiteBalance));
    }

    #[test]
    fn changing_white_balance_invalidates_all_later_creative_stages() {
        let graph = RenderGraph::default();
        let invalid = graph.invalidate_from(StageId::WhiteBalance);
        assert!(invalid.contains(&StageId::Tone));
        assert!(invalid.contains(&StageId::Layers));
        assert!(invalid.contains(&StageId::Geometry));
        assert!(invalid.contains(&StageId::Export));
        assert!(!invalid.contains(&StageId::Decode));
    }

    #[test]
    fn cache_key_changes_with_parameters_but_is_stable_for_same_inputs() {
        let a = stage_cache_key(StageId::Tone, "source", "{\"shadows\":0.2}", &["upstream".into()]);
        let b = stage_cache_key(StageId::Tone, "source", "{\"shadows\":0.2}", &["upstream".into()]);
        let c = stage_cache_key(StageId::Tone, "source", "{\"shadows\":0.3}", &["upstream".into()]);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
