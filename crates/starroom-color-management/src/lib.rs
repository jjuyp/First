//! Color-management boundaries for Starroom.
//! ICC parsing/execution is provided by a pluggable provider (LittleCMS is the intended native
//! implementation). Published chromatic adaptation math lives here so the render graph can keep
//! file, working and display transforms explicit.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Xyz {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

pub const D50: Xyz = Xyz { x: 0.96422, y: 1.0, z: 0.82521 };
pub const D65: Xyz = Xyz { x: 0.95047, y: 1.0, z: 1.08883 };

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix3(pub [[f32; 3]; 3]);

impl Matrix3 {
    pub fn multiply_vec(self, value: Xyz) -> Xyz {
        Xyz {
            x: self.0[0][0] * value.x + self.0[0][1] * value.y + self.0[0][2] * value.z,
            y: self.0[1][0] * value.x + self.0[1][1] * value.y + self.0[1][2] * value.z,
            z: self.0[2][0] * value.x + self.0[2][1] * value.y + self.0[2][2] * value.z,
        }
    }

    pub fn multiply(self, other: Self) -> Self {
        let mut out = [[0.0; 3]; 3];
        for (row, values) in out.iter_mut().enumerate() {
            for (column, value) in values.iter_mut().enumerate() {
                *value = (0..3).map(|index| self.0[row][index] * other.0[index][column]).sum();
            }
        }
        Self(out)
    }

    pub fn inverse(self) -> Option<Self> {
        let m = self.0;
        let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
        if det.abs() < 1.0e-8 || !det.is_finite() {
            return None;
        }
        let d = 1.0 / det;
        Some(Self([
            [
                (m[1][1] * m[2][2] - m[1][2] * m[2][1]) * d,
                (m[0][2] * m[2][1] - m[0][1] * m[2][2]) * d,
                (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * d,
            ],
            [
                (m[1][2] * m[2][0] - m[1][0] * m[2][2]) * d,
                (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * d,
                (m[0][2] * m[1][0] - m[0][0] * m[1][2]) * d,
            ],
            [
                (m[1][0] * m[2][1] - m[1][1] * m[2][0]) * d,
                (m[0][1] * m[2][0] - m[0][0] * m[2][1]) * d,
                (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * d,
            ],
        ]))
    }
}

const BRADFORD: Matrix3 = Matrix3([
    [0.8951, 0.2664, -0.1614],
    [-0.7502, 1.7135, 0.0367],
    [0.0389, -0.0685, 1.0296],
]);

pub fn bradford_adaptation(source_white: Xyz, destination_white: Xyz) -> Matrix3 {
    let source_lms = BRADFORD.multiply_vec(source_white);
    let destination_lms = BRADFORD.multiply_vec(destination_white);
    let scale = Matrix3([
        [destination_lms.x / source_lms.x, 0.0, 0.0],
        [0.0, destination_lms.y / source_lms.y, 0.0],
        [0.0, 0.0, destination_lms.z / source_lms.z],
    ]);
    let inverse = BRADFORD.inverse().expect("Bradford matrix is invertible");
    inverse.multiply(scale).multiply(BRADFORD)
}

pub fn adapt_xyz(value: Xyz, source_white: Xyz, destination_white: Xyz) -> Xyz {
    bradford_adaptation(source_white, destination_white).multiply_vec(value)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RenderingIntent {
    Perceptual,
    RelativeColorimetric,
    Saturation,
    AbsoluteColorimetric,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IccProfileDescriptor {
    pub name: String,
    pub fingerprint: String,
    pub embedded: bool,
}

pub trait IccTransformProvider {
    type Error;
    type Transform;

    fn build_transform(
        &self,
        input_profile: &[u8],
        output_profile: &[u8],
        intent: RenderingIntent,
        black_point_compensation: bool,
    ) -> Result<Self::Transform, Self::Error>;

    fn apply_rgb_f32(
        &self,
        transform: &Self::Transform,
        pixels: &mut [[f32; 3]],
    ) -> Result<(), Self::Error>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WorkingSpace {
    Rec2020LinearD65,
}

impl Default for WorkingSpace {
    fn default() -> Self {
        Self::Rec2020LinearD65
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 2.0e-4
    }

    #[test]
    fn adapting_white_maps_source_white_to_destination_white() {
        let adapted = adapt_xyz(D65, D65, D50);
        assert!(close(adapted.x, D50.x));
        assert!(close(adapted.y, D50.y));
        assert!(close(adapted.z, D50.z));
    }

    #[test]
    fn same_white_adaptation_is_identity_for_sample() {
        let sample = Xyz { x: 0.3, y: 0.4, z: 0.2 };
        let adapted = adapt_xyz(sample, D65, D65);
        assert!(close(adapted.x, sample.x));
        assert!(close(adapted.y, sample.y));
        assert!(close(adapted.z, sample.z));
    }
}
