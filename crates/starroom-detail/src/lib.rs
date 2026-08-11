//! CPU reference detail engine for Starroom.
//! Production preview/export will move these operations to tiled GPU kernels, but the GPU must
//! remain numerically close to these deterministic references.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq)]
pub struct LinearImage {
    pub width: usize,
    pub height: usize,
    /// Interleaved RGB linear-light pixels.
    pub data: Vec<f32>,
}

impl LinearImage {
    pub fn new(width: usize, height: usize, data: Vec<f32>) -> Result<Self, DetailError> {
        if data.len() != width.saturating_mul(height).saturating_mul(3) {
            return Err(DetailError::InvalidBufferLength);
        }
        if data.iter().any(|value| !value.is_finite()) {
            return Err(DetailError::NonFiniteInput);
        }
        Ok(Self {
            width,
            height,
            data,
        })
    }

    fn sample(&self, x: isize, y: isize, channel: usize) -> f32 {
        let safe_x = x.clamp(0, self.width.saturating_sub(1) as isize) as usize;
        let safe_y = y.clamp(0, self.height.saturating_sub(1) as isize) as usize;
        self.data[(safe_y * self.width + safe_x) * 3 + channel]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SharpenParameters {
    /// 0..1
    pub amount: f32,
    /// Gaussian radius in pixels at full resolution.
    pub radius: f32,
    /// Minimum local difference before sharpening, linear-light domain.
    pub threshold: f32,
}

impl Default for SharpenParameters {
    fn default() -> Self {
        Self {
            amount: 0.35,
            radius: 1.0,
            threshold: 0.002,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DenoiseParameters {
    /// 0..1 luminance smoothing strength.
    pub luminance: f32,
    /// 0..1 chroma smoothing strength.
    pub chroma: f32,
    pub radius: f32,
}

impl Default for DenoiseParameters {
    fn default() -> Self {
        Self {
            luminance: 0.0,
            chroma: 0.0,
            radius: 1.25,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailError {
    InvalidBufferLength,
    NonFiniteInput,
}

fn gaussian_kernel(radius: f32) -> Vec<f32> {
    let sigma = radius.max(0.25);
    let half = (sigma * 3.0).ceil().clamp(1.0, 24.0) as isize;
    let mut kernel = Vec::with_capacity((half * 2 + 1) as usize);
    let mut sum = 0.0;
    for offset in -half..=half {
        let x = offset as f32;
        let value = (-0.5 * (x / sigma).powi(2)).exp();
        kernel.push(value);
        sum += value;
    }
    for value in &mut kernel {
        *value /= sum.max(f32::EPSILON);
    }
    kernel
}

pub fn gaussian_blur(image: &LinearImage, radius: f32) -> LinearImage {
    if image.width == 0 || image.height == 0 {
        return image.clone();
    }
    let kernel = gaussian_kernel(radius);
    let half = (kernel.len() / 2) as isize;
    let mut horizontal = vec![0.0; image.data.len()];
    for y in 0..image.height {
        for x in 0..image.width {
            for channel in 0..3 {
                let mut value = 0.0;
                for (index, weight) in kernel.iter().enumerate() {
                    let offset = index as isize - half;
                    value += image.sample(x as isize + offset, y as isize, channel) * weight;
                }
                horizontal[(y * image.width + x) * 3 + channel] = value;
            }
        }
    }

    let horizontal_image = LinearImage {
        width: image.width,
        height: image.height,
        data: horizontal,
    };
    let mut output = vec![0.0; image.data.len()];
    for y in 0..image.height {
        for x in 0..image.width {
            for channel in 0..3 {
                let mut value = 0.0;
                for (index, weight) in kernel.iter().enumerate() {
                    let offset = index as isize - half;
                    value +=
                        horizontal_image.sample(x as isize, y as isize + offset, channel) * weight;
                }
                output[(y * image.width + x) * 3 + channel] = value;
            }
        }
    }
    LinearImage {
        width: image.width,
        height: image.height,
        data: output,
    }
}

pub fn sharpen(image: &LinearImage, parameters: SharpenParameters) -> LinearImage {
    let amount = parameters.amount.clamp(0.0, 2.0);
    if amount <= f32::EPSILON {
        return image.clone();
    }
    let blurred = gaussian_blur(image, parameters.radius);
    let threshold = parameters.threshold.max(0.0);
    let mut output = image.clone();
    for ((destination, source), low) in output.data.iter_mut().zip(&image.data).zip(&blurred.data) {
        let detail = source - low;
        *destination = if detail.abs() >= threshold {
            source + detail * amount
        } else {
            *source
        };
    }
    output
}

fn rgb_to_ycbcr(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let y = 0.2627 * r + 0.6780 * g + 0.0593 * b;
    let cb = b - y;
    let cr = r - y;
    (y, cb, cr)
}

fn ycbcr_to_rgb(y: f32, cb: f32, cr: f32) -> (f32, f32, f32) {
    let r = y + cr;
    let b = y + cb;
    let g = (y - 0.2627 * r - 0.0593 * b) / 0.6780;
    (r, g, b)
}

/// Separates luminance from chroma so chroma noise can be smoothed more strongly without
/// smearing luminance detail. This is a classic deterministic NR path, not AI denoise.
pub fn denoise(image: &LinearImage, parameters: DenoiseParameters) -> LinearImage {
    let luminance_strength = parameters.luminance.clamp(0.0, 1.0);
    let chroma_strength = parameters.chroma.clamp(0.0, 1.0);
    if luminance_strength <= f32::EPSILON && chroma_strength <= f32::EPSILON {
        return image.clone();
    }

    let mut components = Vec::with_capacity(image.data.len());
    for pixel in image.data.chunks_exact(3) {
        let (y, cb, cr) = rgb_to_ycbcr(pixel[0], pixel[1], pixel[2]);
        components.extend_from_slice(&[y, cb, cr]);
    }
    let component_image = LinearImage {
        width: image.width,
        height: image.height,
        data: components,
    };
    let blurred = gaussian_blur(&component_image, parameters.radius);
    let mut output = image.clone();
    for pixel_index in 0..image.width.saturating_mul(image.height) {
        let base = pixel_index * 3;
        let original_y = component_image.data[base];
        let original_cb = component_image.data[base + 1];
        let original_cr = component_image.data[base + 2];
        let y = original_y + (blurred.data[base] - original_y) * luminance_strength * 0.65;
        let cb = original_cb + (blurred.data[base + 1] - original_cb) * chroma_strength;
        let cr = original_cr + (blurred.data[base + 2] - original_cr) * chroma_strength;
        let (r, g, b) = ycbcr_to_rgb(y, cb, cr);
        output.data[base] = r;
        output.data[base + 1] = g;
        output.data[base + 2] = b;
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> LinearImage {
        LinearImage::new(
            5,
            1,
            vec![
                0.1, 0.1, 0.1, 0.1, 0.1, 0.1, 0.8, 0.8, 0.8, 0.1, 0.1, 0.1, 0.1, 0.1, 0.1,
            ],
        )
        .expect("fixture")
    }

    #[test]
    fn zero_sharpen_is_identity() {
        let image = fixture();
        let output = sharpen(
            &image,
            SharpenParameters {
                amount: 0.0,
                ..Default::default()
            },
        );
        assert_eq!(output, image);
    }

    #[test]
    fn sharpen_increases_edge_peak() {
        let image = fixture();
        let output = sharpen(
            &image,
            SharpenParameters {
                amount: 0.8,
                ..Default::default()
            },
        );
        assert!(output.data[6] > image.data[6]);
    }

    #[test]
    fn zero_denoise_is_identity() {
        let image = fixture();
        assert_eq!(denoise(&image, DenoiseParameters::default()), image);
    }

    #[test]
    fn chroma_denoise_reduces_color_spike() {
        let image = LinearImage::new(3, 1, vec![0.2, 0.2, 0.2, 0.8, 0.1, 0.1, 0.2, 0.2, 0.2])
            .expect("fixture");
        let output = denoise(
            &image,
            DenoiseParameters {
                luminance: 0.0,
                chroma: 1.0,
                radius: 1.0,
            },
        );
        let original_chroma = (image.data[3] - image.data[4]).abs();
        let output_chroma = (output.data[3] - output.data[4]).abs();
        assert!(output_chroma < original_chroma);
    }
}
