//! Native rendered-image I/O for Starroom v0.2.
//! RAW remains a separate pipeline. This crate decodes common rendered formats, preserves their
//! encoded sample values, and exposes embedded metadata so color management can happen explicitly.

use image::{
    DynamicImage, ExtendedColorType, ImageDecoder, ImageEncoder, ImageFormat, ImageReader,
};
use serde::{Deserialize, Serialize};
use std::{io::Cursor, path::Path};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ImageIoError {
    #[error("image file operation failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("image codec failed: {0}")]
    Codec(#[from] image::ImageError),
    #[error("image format could not be detected")]
    UnknownFormat,
    #[error("RGB buffer length does not match dimensions")]
    InvalidBufferLength,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RenderedFormat {
    Jpeg,
    Png,
    Tiff,
}

impl TryFrom<ImageFormat> for RenderedFormat {
    type Error = ImageIoError;

    fn try_from(value: ImageFormat) -> Result<Self, Self::Error> {
        match value {
            ImageFormat::Jpeg => Ok(Self::Jpeg),
            ImageFormat::Png => Ok(Self::Png),
            ImageFormat::Tiff => Ok(Self::Tiff),
            _ => Err(ImageIoError::UnknownFormat),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecodedRenderedImage {
    pub width: u32,
    pub height: u32,
    pub format: RenderedFormat,
    /// Encoded RGB(A) samples normalized to 0..1. These are not yet converted to Starroom's
    /// linear Rec.2020 working space; the color-management/input-transform stage owns that step.
    pub rgba: Vec<f32>,
    pub embedded_icc: Option<Vec<u8>>,
    pub exif: Option<Vec<u8>>,
}

fn dynamic_to_rgba_f32(image: DynamicImage) -> Vec<f32> {
    image.into_rgba32f().into_raw()
}

fn decode_rendered_inner(
    path: impl AsRef<Path>,
    max_edge: Option<u32>,
) -> Result<DecodedRenderedImage, ImageIoError> {
    let reader = ImageReader::open(path)?.with_guessed_format()?;
    let format = reader.format().ok_or(ImageIoError::UnknownFormat)?;
    let rendered_format = RenderedFormat::try_from(format)?;
    let mut decoder = reader.into_decoder()?;
    let (width, height) = decoder.dimensions();
    let embedded_icc = decoder.icc_profile()?;
    let exif = decoder.exif_metadata()?;
    let mut image = DynamicImage::from_decoder(decoder)?;
    if let Some(max_edge) = max_edge.filter(|edge| *edge > 0) {
        if width > max_edge || height > max_edge {
            // Lanczos3 is a mature, deterministic image-crate resampler. Resizing is performed
            // before the shared color/tone graph only for interactive preview; export always
            // decodes the full source independently.
            image = image.resize(max_edge, max_edge, image::imageops::FilterType::Lanczos3);
        }
    }
    let (decoded_width, decoded_height) = (image.width(), image.height());
    Ok(DecodedRenderedImage {
        width: decoded_width,
        height: decoded_height,
        format: rendered_format,
        rgba: dynamic_to_rgba_f32(image),
        embedded_icc,
        exif,
    })
}

pub fn decode_rendered(path: impl AsRef<Path>) -> Result<DecodedRenderedImage, ImageIoError> {
    decode_rendered_inner(path, None)
}

/// Decodes a bounded interactive-preview source while retaining metadata/profile ownership.
/// Full-resolution export must call [`decode_rendered`] instead.
pub fn decode_rendered_preview(
    path: impl AsRef<Path>,
    max_edge: u32,
) -> Result<DecodedRenderedImage, ImageIoError> {
    decode_rendered_inner(path, Some(max_edge))
}

/// Encodes an already output-transformed RGB8 buffer. The caller owns gamut mapping and output
/// ICC conversion; this function only performs JPEG compression.
pub fn encode_jpeg_rgb8(
    rgb: &[u8],
    width: u32,
    height: u32,
    quality: u8,
    icc_profile: Option<Vec<u8>>,
) -> Result<Vec<u8>, ImageIoError> {
    let expected = width as usize * height as usize * 3;
    if rgb.len() != expected {
        return Err(ImageIoError::InvalidBufferLength);
    }
    let mut cursor = Cursor::new(Vec::new());
    let mut encoder =
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, quality.clamp(1, 100));
    if let Some(profile) = icc_profile {
        encoder
            .set_icc_profile(profile)
            .map_err(image::ImageError::Unsupported)?;
    }
    encoder.write_image(rgb, width, height, ExtendedColorType::Rgb8)?;
    Ok(cursor.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jpeg_encoder_rejects_wrong_buffer_length() {
        let result = encode_jpeg_rgb8(&[0, 0, 0], 2, 2, 90, None);
        assert!(matches!(result, Err(ImageIoError::InvalidBufferLength)));
    }

    #[test]
    fn jpeg_round_trip_decodes_dimensions() {
        let rgb = [255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255];
        let bytes = encode_jpeg_rgb8(&rgb, 2, 2, 95, None).expect("encode");
        let reader = ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .expect("guess format");
        let decoded = reader.decode().expect("decode");
        assert_eq!(decoded.width(), 2);
        assert_eq!(decoded.height(), 2);
    }
}
