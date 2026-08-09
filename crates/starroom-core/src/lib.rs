//! Core image-state primitives. Pixel decoding/rendering is added in M1.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("could not read source image {path}: {source}")]
    ReadSource {
        path: PathBuf,
        source: std::io::Error,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceIdentity {
    pub path: PathBuf,
    pub content_hash: String,
    pub byte_length: u64,
}

impl SourceIdentity {
    /// Reads source bytes only to establish identity. The source is never opened for writing.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, CoreError> {
        let path = path.as_ref();
        let bytes = fs::read(path).map_err(|source| CoreError::ReadSource {
            path: path.to_owned(),
            source,
        })?;
        Ok(Self {
            path: path.to_owned(),
            content_hash: format!("{:x}", Sha256::digest(&bytes)),
            byte_length: bytes.len() as u64,
        })
    }

    pub fn verify_unchanged(&self) -> Result<bool, CoreError> {
        Ok(Self::from_path(&self.path)?.content_hash == self.content_hash)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalAdjustments {
    pub exposure_ev: f32,
    pub contrast: f32,
    pub highlights: f32,
    pub shadows: f32,
    pub whites: f32,
    pub blacks: f32,
    pub temperature: f32,
    pub tint: f32,
    pub vibrance: f32,
    pub saturation: f32,
}

impl Default for GlobalAdjustments {
    fn default() -> Self {
        Self {
            exposure_ev: 0.0,
            contrast: 0.0,
            highlights: 0.0,
            shadows: 0.0,
            whites: 0.0,
            blacks: 0.0,
            temperature: 0.0,
            tint: 0.0,
            vibrance: 0.0,
            saturation: 0.0,
        }
    }
}

impl GlobalAdjustments {
    pub fn validate(self) -> bool {
        let values = [
            self.exposure_ev,
            self.contrast,
            self.highlights,
            self.shadows,
            self.whites,
            self.blacks,
            self.temperature,
            self.tint,
            self.vibrance,
            self.saturation,
        ];
        values.into_iter().all(f32::is_finite) && (-5.0..=5.0).contains(&self.exposure_ev)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn source_identity_never_mutates_source() {
        let mut file = tempfile::NamedTempFile::new().expect("fixture");
        file.write_all(b"private fixture pixels")
            .expect("fixture bytes");
        let before = fs::read(file.path()).expect("before bytes");
        let identity = SourceIdentity::from_path(file.path()).expect("identity");
        let after = fs::read(file.path()).expect("after bytes");
        assert_eq!(before, after);
        assert!(identity.verify_unchanged().expect("verification"));
    }

    #[test]
    fn rejects_non_finite_adjustments() {
        let invalid = GlobalAdjustments {
            exposure_ev: f32::NAN,
            ..Default::default()
        };
        assert!(!invalid.validate());
    }
}
