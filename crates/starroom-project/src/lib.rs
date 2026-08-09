//! Serializable, storage-independent project state.

use serde::{Deserialize, Serialize};
use starroom_core::{GlobalAdjustments, SourceIdentity};
use std::{fs, path::Path};
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
    pub masks: Vec<MaskNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaskNode {
    pub id: String,
    pub name: String,
    pub enabled: bool,
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
    fn adjustment_state_round_trips() {
        let project = Project {
            schema_version: 1,
            engine_version: "0.1.0".into(),
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
        };
        let json = serde_json::to_string(&project).expect("serialize");
        let restored: Project = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored.global_adjustments.exposure_ev, 0.75);
        assert_eq!(restored.source.content_hash, "abc");
    }
}
