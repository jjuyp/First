//! Geometry and perspective primitives for Starroom.
//! Coordinates are normalized to the source frame unless otherwise documented.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point2 {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CropRect {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl Default for CropRect {
    fn default() -> Self {
        Self { left: 0.0, top: 0.0, right: 1.0, bottom: 1.0 }
    }
}

impl CropRect {
    pub fn normalized(self) -> Self {
        let left = self.left.clamp(0.0, 1.0);
        let top = self.top.clamp(0.0, 1.0);
        let right = self.right.clamp(left + 1.0e-5, 1.0);
        let bottom = self.bottom.clamp(top + 1.0e-5, 1.0);
        Self { left, top, right, bottom }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct GeometryParameters {
    pub rotation_degrees: f32,
    pub vertical_keystone: f32,
    pub horizontal_keystone: f32,
    pub scale: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub flip_horizontal: bool,
    pub flip_vertical: bool,
    pub crop: CropRect,
}

impl Default for GeometryParameters {
    fn default() -> Self {
        Self {
            rotation_degrees: 0.0,
            vertical_keystone: 0.0,
            horizontal_keystone: 0.0,
            scale: 1.0,
            offset_x: 0.0,
            offset_y: 0.0,
            flip_horizontal: false,
            flip_vertical: false,
            crop: CropRect::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix3 {
    pub m: [[f32; 3]; 3],
}

impl Matrix3 {
    pub const IDENTITY: Self = Self {
        m: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    };

    pub fn multiply(self, other: Self) -> Self {
        let mut out = [[0.0; 3]; 3];
        for (row, row_values) in out.iter_mut().enumerate() {
            for (column, value) in row_values.iter_mut().enumerate() {
                *value = (0..3).map(|index| self.m[row][index] * other.m[index][column]).sum();
            }
        }
        Self { m: out }
    }

    pub fn transform(self, point: Point2) -> Point2 {
        let x = self.m[0][0] * point.x + self.m[0][1] * point.y + self.m[0][2];
        let y = self.m[1][0] * point.x + self.m[1][1] * point.y + self.m[1][2];
        let w = self.m[2][0] * point.x + self.m[2][1] * point.y + self.m[2][2];
        let safe_w = if w.abs() < 1.0e-8 { 1.0e-8_f32.copysign(w) } else { w };
        Point2 { x: x / safe_w, y: y / safe_w }
    }

    pub fn inverse(self) -> Option<Self> {
        let m = self.m;
        let determinant = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
        if determinant.abs() < 1.0e-8 || !determinant.is_finite() {
            return None;
        }
        let inverse_det = 1.0 / determinant;
        Some(Self {
            m: [
                [
                    (m[1][1] * m[2][2] - m[1][2] * m[2][1]) * inverse_det,
                    (m[0][2] * m[2][1] - m[0][1] * m[2][2]) * inverse_det,
                    (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * inverse_det,
                ],
                [
                    (m[1][2] * m[2][0] - m[1][0] * m[2][2]) * inverse_det,
                    (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * inverse_det,
                    (m[0][2] * m[1][0] - m[0][0] * m[1][2]) * inverse_det,
                ],
                [
                    (m[1][0] * m[2][1] - m[1][1] * m[2][0]) * inverse_det,
                    (m[0][1] * m[2][0] - m[0][0] * m[2][1]) * inverse_det,
                    (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * inverse_det,
                ],
            ],
        })
    }
}

fn translation(x: f32, y: f32) -> Matrix3 {
    Matrix3 { m: [[1.0, 0.0, x], [0.0, 1.0, y], [0.0, 0.0, 1.0]] }
}

fn scale(x: f32, y: f32) -> Matrix3 {
    Matrix3 { m: [[x, 0.0, 0.0], [0.0, y, 0.0], [0.0, 0.0, 1.0]] }
}

fn rotation(degrees: f32) -> Matrix3 {
    let angle = degrees.to_radians();
    let cos = angle.cos();
    let sin = angle.sin();
    Matrix3 { m: [[cos, -sin, 0.0], [sin, cos, 0.0], [0.0, 0.0, 1.0]] }
}

fn keystone(horizontal: f32, vertical: f32) -> Matrix3 {
    Matrix3 {
        m: [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [horizontal.clamp(-1.5, 1.5), vertical.clamp(-1.5, 1.5), 1.0],
        ],
    }
}

/// Builds a forward normalized-coordinate transform around image center.
pub fn build_transform(parameters: GeometryParameters) -> Matrix3 {
    let safe_scale = parameters.scale.clamp(0.05, 20.0);
    let flip_x = if parameters.flip_horizontal { -1.0 } else { 1.0 };
    let flip_y = if parameters.flip_vertical { -1.0 } else { 1.0 };
    let centered = translation(-0.5, -0.5);
    let transform = keystone(parameters.horizontal_keystone, parameters.vertical_keystone)
        .multiply(rotation(parameters.rotation_degrees))
        .multiply(scale(safe_scale * flip_x, safe_scale * flip_y))
        .multiply(centered);
    translation(0.5 + parameters.offset_x, 0.5 + parameters.offset_y).multiply(transform)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1.0e-4
    }

    #[test]
    fn identity_transform_preserves_point() {
        let transform = build_transform(GeometryParameters::default());
        let point = transform.transform(Point2 { x: 0.2, y: 0.8 });
        assert!(close(point.x, 0.2));
        assert!(close(point.y, 0.8));
    }

    #[test]
    fn inverse_round_trip_restores_point() {
        let parameters = GeometryParameters {
            rotation_degrees: 13.0,
            vertical_keystone: 0.12,
            horizontal_keystone: -0.08,
            scale: 1.1,
            offset_x: 0.03,
            offset_y: -0.02,
            ..Default::default()
        };
        let transform = build_transform(parameters);
        let inverse = transform.inverse().expect("invertible");
        let source = Point2 { x: 0.33, y: 0.72 };
        let projected = transform.transform(source);
        let restored = inverse.transform(projected);
        assert!(close(source.x, restored.x));
        assert!(close(source.y, restored.y));
    }

    #[test]
    fn horizontal_flip_mirrors_around_center() {
        let transform = build_transform(GeometryParameters { flip_horizontal: true, ..Default::default() });
        let output = transform.transform(Point2 { x: 0.2, y: 0.5 });
        assert!(close(output.x, 0.8));
        assert!(close(output.y, 0.5));
    }

    #[test]
    fn crop_is_clamped_to_valid_normalized_rectangle() {
        let crop = CropRect { left: -0.2, top: 0.1, right: 1.4, bottom: 0.9 }.normalized();
        assert_eq!(crop.left, 0.0);
        assert_eq!(crop.top, 0.1);
        assert_eq!(crop.right, 1.0);
        assert_eq!(crop.bottom, 0.9);
    }
}
