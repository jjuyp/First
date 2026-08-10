//! Serializable, storage-independent project state.

use serde::{Deserialize, Serialize};
use starroom_core::{GlobalAdjustments, SourceIdentity};
use std::{collections::BTreeMap, fs, path::Path};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("project serialization failed: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("project file operation failed: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub schema_version: u32,
    pub engine_version: String,
    pub source: SourceIdentity,
    pub global_adjustments: GlobalAdjustments,
    #[serde(default)]
    pub masks: Vec<MaskNode>,
    #[serde(default)]
    pub layers: Vec<AdjustmentLayer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaskNode {
    pub id: String,
    pub name: String,
    pub enabled: bool,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum BlendMode {
    #[default]
    Normal,
    Luminosity,
    Color,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum MaskDefinition {
    None,
    Radial {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        rotation: f32,
        feather: f32,
        invert: bool,
    },
    Linear {
        start_x: f32,
        start_y: f32,
        end_x: f32,
        end_y: f32,
        feather: f32,
    },
    Brush {
        points: Vec<BrushPoint>,
        radius: f32,
        feather: f32,
        flow: f32,
    },
    Provider {
        provider: String,
        request: String,
        fingerprint: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BrushPoint {
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdjustmentLayer {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub opacity: f32,
    #[serde(default)]
    pub blend_mode: BlendMode,
    pub order: u32,
    pub mask: MaskDefinition,
    /// Storage-independent parameter map. Typed engine parameters are resolved by schema/version.
    #[serde(default)]
    pub adjustments: BTreeMap<String, f32>,
}

impl Project {
    pub fn write_sidecar(&self, path: impl AsRef<Path>) -> Result<(), ProjectError> {
        let json = serde_json::to_vec_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjustment_state_and_layers_round_trip() {
        let mut adjustments = BTreeMap::new();
        adjustments.insert("exposure".into(), 0.35);
        let project = Project {
            schema_version: 2,
            engine_version: "0.2.0".into(),
            source: SourceIdentity {
                path: "photo.jpg".into(),
                content_hash: "abc".into(),
                byte_length: 42,
            },
            global_adjustments: GlobalAdjustments {
                exposure_ev: 0.75,
                ..Default::default()
            },
            masks: vec![],
            layers: vec![AdjustmentLayer {
                id: "portrait-light".into(),
                name: "Portrait Light".into(),
                enabled: true,
                opacity: 1.0,
                blend_mode: BlendMode::Normal,
                order: 0,
                mask: MaskDefinition::Provider {
                    provider: "subject".into(),
                    request: "person".into(),
                    fingerprint: None,
                },
                adjustments,
            }],
        };
        let json = serde_json::to_string(&project).expect("serialize");
        let restored: Project = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.global_adjustments.exposure_ev, 0.75);
        assert_eq!(restored.source.content_hash, "abc");
        assert_eq!(restored.layers.len(), 1);
        assert_eq!(
            restored.layers[0].adjustments.get("exposure"),
            Some(&0.35)
        );
    }

    #[test]
    fn old_projects_without_layers_remain_readable() {
        let json = r#"{"schemaVersion":1,"engineVersion":"0.1.0","source":{"path":"photo.jpg","contentHash":"abc","byteLength":42},"globalAdjustments":{"exposureEv":0.0,"contrast":0.0,"highlights":0.0,"shadows":0.0,"whites":0.0,"blacks":0.0,"temperature":0.0,"tint":0.0,"vibrance":0.0,"saturation":0.0},"masks":[]}"#;
        let restored: Project = serde_json::from_str(json).expect("deserialize old project");
        assert!(restored.layers.is_empty());
    }
}
