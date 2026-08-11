//! Deterministic healing-brush reference for Starroom.
//! V1 copies nearby texture while adapting low-frequency color/luminance and feathering the
//! destination. AI/content-aware inpainting remains a replaceable future provider.

use serde::{Deserialize, Serialize};
use starroom_detail::{LinearImage, gaussian_blur};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HealPoint {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HealStroke {
    pub source: HealPoint,
    pub destination: HealPoint,
    /// Radius in full-resolution pixels.
    pub radius: f32,
    /// 0 hard edge, 1 broad feather.
    pub feather: f32,
    /// 0..1 blend strength.
    pub opacity: f32,
}

fn smoothstep(edge0: f32, edge1: f32, value: f32) -> f32 {
    if (edge1 - edge0).abs() < f32::EPSILON {
        return if value < edge0 { 0.0 } else { 1.0 };
    }
    let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn normalized_to_pixel(point: HealPoint, image: &LinearImage) -> (f32, f32) {
    (
        point.x.clamp(0.0, 1.0) * image.width.saturating_sub(1) as f32,
        point.y.clamp(0.0, 1.0) * image.height.saturating_sub(1) as f32,
    )
}

fn sample_bilinear(image: &LinearImage, x: f32, y: f32, channel: usize) -> f32 {
    if image.width == 0 || image.height == 0 {
        return 0.0;
    }
    let x = x.clamp(0.0, image.width.saturating_sub(1) as f32);
    let y = y.clamp(0.0, image.height.saturating_sub(1) as f32);
    let x0 = x.floor() as usize;
    let y0 = y.floor() as usize;
    let x1 = (x0 + 1).min(image.width - 1);
    let y1 = (y0 + 1).min(image.height - 1);
    let tx = x - x0 as f32;
    let ty = y - y0 as f32;
    let read = |px: usize, py: usize| image.data[(py * image.width + px) * 3 + channel];
    let top = read(x0, y0) * (1.0 - tx) + read(x1, y0) * tx;
    let bottom = read(x0, y1) * (1.0 - tx) + read(x1, y1) * tx;
    top * (1.0 - ty) + bottom * ty
}

/// Applies one circular healing stroke. Source high-frequency texture is preserved while the
/// source patch low frequency is shifted toward the destination low frequency.
pub fn apply_heal(image: &LinearImage, stroke: HealStroke) -> LinearImage {
    if image.width == 0 || image.height == 0 || stroke.radius <= 0.0 || stroke.opacity <= 0.0 {
        return image.clone();
    }
    let radius = stroke.radius.clamp(0.5, 512.0);
    let feather = stroke.feather.clamp(0.0, 1.0);
    let opacity = stroke.opacity.clamp(0.0, 1.0);
    let low = gaussian_blur(image, (radius * 0.2).clamp(1.0, 24.0));
    let (source_x, source_y) = normalized_to_pixel(stroke.source, image);
    let (destination_x, destination_y) = normalized_to_pixel(stroke.destination, image);
    let delta_x = source_x - destination_x;
    let delta_y = source_y - destination_y;

    let left = (destination_x - radius).floor().max(0.0) as usize;
    let right = (destination_x + radius)
        .ceil()
        .min(image.width.saturating_sub(1) as f32) as usize;
    let top = (destination_y - radius).floor().max(0.0) as usize;
    let bottom = (destination_y + radius)
        .ceil()
        .min(image.height.saturating_sub(1) as f32) as usize;
    let inner = radius * (1.0 - feather * 0.85);

    let mut output = image.clone();
    for y in top..=bottom {
        for x in left..=right {
            let dx = x as f32 - destination_x;
            let dy = y as f32 - destination_y;
            let distance = (dx * dx + dy * dy).sqrt();
            if distance > radius {
                continue;
            }
            let edge = if radius <= inner + 1.0e-5 {
                1.0
            } else {
                1.0 - smoothstep(inner, radius, distance)
            };
            let blend = edge * opacity;
            let source_sample_x = x as f32 + delta_x;
            let source_sample_y = y as f32 + delta_y;
            for channel in 0..3 {
                let source_value =
                    sample_bilinear(image, source_sample_x, source_sample_y, channel);
                let source_low = sample_bilinear(&low, source_sample_x, source_sample_y, channel);
                let destination_low = low.data[(y * image.width + x) * 3 + channel];
                let adapted = destination_low + (source_value - source_low);
                let index = (y * image.width + x) * 3 + channel;
                output.data[index] = image.data[index] * (1.0 - blend) + adapted * blend;
            }
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgb_index(width: usize, x: usize, y: usize) -> usize {
        (y * width + x) * 3
    }

    fn flat_with_spot() -> LinearImage {
        let mut data = vec![0.2; 7 * 3 * 3];
        let center = rgb_index(7, 3, 1);
        data[center] = 0.9;
        data[center + 1] = 0.1;
        data[center + 2] = 0.1;
        LinearImage::new(7, 3, data).expect("fixture")
    }

    #[test]
    fn zero_opacity_is_identity() {
        let image = flat_with_spot();
        let output = apply_heal(
            &image,
            HealStroke {
                source: HealPoint { x: 0.1, y: 0.5 },
                destination: HealPoint { x: 0.5, y: 0.5 },
                radius: 2.0,
                feather: 0.5,
                opacity: 0.0,
            },
        );
        assert_eq!(output, image);
    }

    #[test]
    fn healing_reduces_isolated_color_spot() {
        let image = flat_with_spot();
        let center = rgb_index(7, 3, 1);
        let output = apply_heal(
            &image,
            HealStroke {
                source: HealPoint { x: 0.0, y: 0.5 },
                destination: HealPoint { x: 0.5, y: 0.5 },
                radius: 1.5,
                feather: 0.4,
                opacity: 1.0,
            },
        );
        assert!(output.data[center] < image.data[center]);
        assert!(output.data[center + 1] > image.data[center + 1]);
    }

    #[test]
    fn healing_stays_finite_at_edges() {
        let image = flat_with_spot();
        let output = apply_heal(
            &image,
            HealStroke {
                source: HealPoint { x: 0.0, y: 0.0 },
                destination: HealPoint { x: 1.0, y: 1.0 },
                radius: 4.0,
                feather: 1.0,
                opacity: 1.0,
            },
        );
        assert!(output.data.iter().all(|value| value.is_finite()));
    }
}
