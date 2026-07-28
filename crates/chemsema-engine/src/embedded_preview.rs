use std::io::{Cursor, Read};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use flate2::read::ZlibDecoder;
use image::{DynamicImage, ImageFormat, ImageReader};
use serde::{Deserialize, Serialize};

use crate::ImageResourceData;

pub const MAX_EMBEDDED_PREVIEW_SOURCE_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_EMBEDDED_PREVIEW_PIXELS: u64 = 100_000_000;
pub const MAX_EMBEDDED_PREVIEW_DIMENSION: u32 = 32_768;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EmbeddedPreviewStatus {
    Decoded,
    NoPreview,
    InvalidSignature,
    Oversize,
    DecodeError,
}

impl EmbeddedPreviewStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Decoded => "decoded",
            Self::NoPreview => "no-preview",
            Self::InvalidSignature => "invalid-signature",
            Self::Oversize => "oversize",
            Self::DecodeError => "decode-error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddedObjectResourceData {
    #[serde(default = "embedded_object_schema")]
    pub schema: String,
    pub format: String,
    pub data_base64: String,
    pub preview_status: EmbeddedPreviewStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<ImageResourceData>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview_detail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uncompressed_size: Option<u64>,
}

fn embedded_object_schema() -> String {
    "chemsema.resource.embedded-object.v1".to_string()
}

#[derive(Debug, Clone, PartialEq)]
pub struct EmbeddedPreviewResult {
    pub status: EmbeddedPreviewStatus,
    pub preview: Option<ImageResourceData>,
    pub detail: Option<String>,
    pub uncompressed_size: Option<u64>,
}

pub fn extract_embedded_preview(
    format: &str,
    source: &[u8],
    declared_uncompressed_size: Option<u64>,
) -> EmbeddedPreviewResult {
    if source.is_empty() {
        return result(EmbeddedPreviewStatus::InvalidSignature, "empty payload");
    }
    if source.len() > MAX_EMBEDDED_PREVIEW_SOURCE_BYTES {
        return result(EmbeddedPreviewStatus::Oversize, "source exceeds 64 MiB");
    }
    let (container_format, bytes, uncompressed_size) =
        match decompress_container(format, source, declared_uncompressed_size) {
            Ok(value) => value,
            Err(value) => return value,
        };
    if !valid_container_signature(container_format, &bytes) {
        return result(
            EmbeddedPreviewStatus::InvalidSignature,
            "container signature does not match the declared format",
        );
    }
    let decoded = match container_format {
        "TIFF" => decode_image(&bytes, ImageFormat::Tiff),
        "EnhancedMetafile" | "WindowsMetafile" | "OLEObject" | "PDF" | "MacPICT" => {
            extract_embedded_raster(&bytes)
        }
        _ => Err("unsupported embedded-object format".to_string()),
    };
    match decoded {
        Ok(preview) => EmbeddedPreviewResult {
            status: EmbeddedPreviewStatus::Decoded,
            preview: Some(preview),
            detail: None,
            uncompressed_size,
        },
        Err(detail) if detail == "no embedded raster preview" => EmbeddedPreviewResult {
            status: EmbeddedPreviewStatus::NoPreview,
            preview: None,
            detail: Some(detail),
            uncompressed_size,
        },
        Err(detail) => EmbeddedPreviewResult {
            status: if detail.contains("limit") || detail.contains("exceeds") {
                EmbeddedPreviewStatus::Oversize
            } else {
                EmbeddedPreviewStatus::DecodeError
            },
            preview: None,
            detail: Some(detail),
            uncompressed_size,
        },
    }
}

pub fn cropped_image_resource(
    image: &ImageResourceData,
    crop: crate::ImageCropRect,
) -> Result<ImageResourceData, String> {
    crop.validate(image.pixel_width, image.pixel_height)?;
    let bytes = BASE64
        .decode(image.data_base64.as_bytes())
        .map_err(|error| error.to_string())?;
    let format = match image.mime_type.as_str() {
        "image/png" => ImageFormat::Png,
        "image/jpeg" => ImageFormat::Jpeg,
        "image/gif" => ImageFormat::Gif,
        "image/bmp" => ImageFormat::Bmp,
        "image/tiff" => ImageFormat::Tiff,
        value => return Err(format!("unsupported crop source MIME type {value}")),
    };
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_EMBEDDED_PREVIEW_DIMENSION);
    limits.max_image_height = Some(MAX_EMBEDDED_PREVIEW_DIMENSION);
    limits.max_alloc = Some(MAX_EMBEDDED_PREVIEW_PIXELS.saturating_mul(8));
    reader.limits(limits);
    let decoded = reader.decode().map_err(|error| error.to_string())?;
    encode_png_preview(decoded.crop_imm(
        crop.x as u32,
        crop.y as u32,
        crop.width as u32,
        crop.height as u32,
    ))
}

pub(crate) fn decompress_container<'a>(
    format: &'a str,
    source: &[u8],
    declared_size: Option<u64>,
) -> Result<(&'a str, Vec<u8>, Option<u64>), EmbeddedPreviewResult> {
    let plain_format = match format {
        "CompressedEnhancedMetafile" => "EnhancedMetafile",
        "CompressedWindowsMetafile" => "WindowsMetafile",
        "CompressedOLEObject" => "OLEObject",
        _ => return Ok((format, source.to_vec(), None)),
    };
    if declared_size.is_some_and(|size| size > MAX_EMBEDDED_PREVIEW_SOURCE_BYTES as u64) {
        return Err(result(
            EmbeddedPreviewStatus::Oversize,
            "declared uncompressed size exceeds 64 MiB",
        ));
    }
    let mut decoder = ZlibDecoder::new(source);
    let mut bytes = Vec::new();
    if decoder
        .by_ref()
        .take((MAX_EMBEDDED_PREVIEW_SOURCE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .is_err()
    {
        return Err(result(
            EmbeddedPreviewStatus::DecodeError,
            "zlib decompression failed",
        ));
    }
    if bytes.len() > MAX_EMBEDDED_PREVIEW_SOURCE_BYTES {
        return Err(result(
            EmbeddedPreviewStatus::Oversize,
            "uncompressed payload exceeds 64 MiB",
        ));
    }
    if let Some(expected) = declared_size {
        if expected != bytes.len() as u64 {
            return Err(result(
                EmbeddedPreviewStatus::DecodeError,
                "declared uncompressed size does not match decoded bytes",
            ));
        }
    }
    let decoded_size = bytes.len() as u64;
    Ok((plain_format, bytes, Some(decoded_size)))
}

fn result(status: EmbeddedPreviewStatus, detail: &str) -> EmbeddedPreviewResult {
    EmbeddedPreviewResult {
        status,
        preview: None,
        detail: Some(detail.to_string()),
        uncompressed_size: None,
    }
}

fn valid_container_signature(format: &str, bytes: &[u8]) -> bool {
    match format {
        "TIFF" => bytes.get(..4) == Some(b"II\x2a\0") || bytes.get(..4) == Some(b"MM\0\x2a"),
        "EnhancedMetafile" => {
            bytes.get(..4) == Some(&1u32.to_le_bytes()) && bytes.get(40..44) == Some(b" EMF")
        }
        "WindowsMetafile" => {
            bytes.get(..4) == Some(&0x9ac6_cdd7u32.to_le_bytes())
                || (matches!(bytes.get(..2), Some([1, 0] | [2, 0]))
                    && bytes.get(2..4) == Some(&9u16.to_le_bytes()))
        }
        "OLEObject" => bytes.get(..8) == Some(b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1"),
        "PDF" => bytes.starts_with(b"%PDF-"),
        "MacPICT" => valid_pict_signature(bytes),
        _ => false,
    }
}

fn valid_pict_signature(bytes: &[u8]) -> bool {
    [0usize, 512].into_iter().any(|offset| {
        bytes.len() >= offset + 14
            && bytes[offset + 10..]
                .windows(4)
                .take(16)
                .any(|value| value == [0x00, 0x11, 0x02, 0xff] || value == [0x11, 0x01, 0x02, 0xff])
    })
}

fn extract_embedded_raster(bytes: &[u8]) -> Result<ImageResourceData, String> {
    let candidates = [
        (b"\x89PNG\r\n\x1a\n".as_slice(), ImageFormat::Png),
        (b"\xff\xd8\xff".as_slice(), ImageFormat::Jpeg),
        (b"II\x2a\0".as_slice(), ImageFormat::Tiff),
        (b"MM\0\x2a".as_slice(), ImageFormat::Tiff),
        (b"GIF87a".as_slice(), ImageFormat::Gif),
        (b"GIF89a".as_slice(), ImageFormat::Gif),
        (b"BM".as_slice(), ImageFormat::Bmp),
    ];
    for (signature, format) in candidates {
        for offset in find_all(bytes, signature) {
            if let Ok(image) = decode_image(&bytes[offset..], format) {
                return Ok(image);
            }
        }
    }
    for offset in 0..bytes.len().saturating_sub(40) {
        if matches!(
            bytes.get(offset..offset + 4),
            Some([40, 0, 0, 0] | [108, 0, 0, 0] | [124, 0, 0, 0])
        ) {
            if let Some(bmp) = dib_to_bmp(&bytes[offset..]) {
                if let Ok(image) = decode_image(&bmp, ImageFormat::Bmp) {
                    return Ok(image);
                }
            }
        }
    }
    Err("no embedded raster preview".to_string())
}

fn find_all(haystack: &[u8], needle: &[u8]) -> Vec<usize> {
    haystack
        .windows(needle.len())
        .enumerate()
        .filter_map(|(index, value)| (value == needle).then_some(index))
        .collect()
}

fn dib_to_bmp(dib: &[u8]) -> Option<Vec<u8>> {
    let header_size = u32::from_le_bytes(dib.get(..4)?.try_into().ok()?) as usize;
    if !matches!(header_size, 40 | 108 | 124) || dib.len() < header_size {
        return None;
    }
    let width = i32::from_le_bytes(dib.get(4..8)?.try_into().ok()?).unsigned_abs();
    let height = i32::from_le_bytes(dib.get(8..12)?.try_into().ok()?).unsigned_abs();
    let planes = u16::from_le_bytes(dib.get(12..14)?.try_into().ok()?);
    let bits = u16::from_le_bytes(dib.get(14..16)?.try_into().ok()?);
    let compression = u32::from_le_bytes(dib.get(16..20)?.try_into().ok()?);
    if width == 0
        || height == 0
        || planes != 1
        || !matches!(bits, 1 | 4 | 8 | 16 | 24 | 32)
        || !matches!(compression, 0 | 3)
    {
        return None;
    }
    let colors = if bits <= 8 { 1usize << bits } else { 0 };
    let pixel_offset = header_size.checked_add(colors.checked_mul(4)?)?;
    let row_bytes = ((u64::from(width) * u64::from(bits) + 31) / 32) * 4;
    let pixel_bytes = row_bytes.checked_mul(u64::from(height))? as usize;
    let dib_len = pixel_offset.checked_add(pixel_bytes)?;
    let dib = dib.get(..dib_len)?;
    let file_size = 14usize.checked_add(dib.len())?;
    let mut bmp = Vec::with_capacity(file_size);
    bmp.extend_from_slice(b"BM");
    bmp.extend_from_slice(&(file_size as u32).to_le_bytes());
    bmp.extend_from_slice(&[0; 4]);
    bmp.extend_from_slice(&((14 + pixel_offset) as u32).to_le_bytes());
    bmp.extend_from_slice(dib);
    Some(bmp)
}

fn decode_image(bytes: &[u8], format: ImageFormat) -> Result<ImageResourceData, String> {
    if bytes.len() > MAX_EMBEDDED_PREVIEW_SOURCE_BYTES {
        return Err("image source exceeds byte limit".to_string());
    }
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_EMBEDDED_PREVIEW_DIMENSION);
    limits.max_image_height = Some(MAX_EMBEDDED_PREVIEW_DIMENSION);
    limits.max_alloc = Some(MAX_EMBEDDED_PREVIEW_PIXELS.saturating_mul(8));
    reader.limits(limits);
    let image = reader.decode().map_err(|error| error.to_string())?;
    encode_png_preview(image)
}

fn encode_png_preview(image: DynamicImage) -> Result<ImageResourceData, String> {
    let width = image.width();
    let height = image.height();
    if width == 0
        || height == 0
        || width > MAX_EMBEDDED_PREVIEW_DIMENSION
        || height > MAX_EMBEDDED_PREVIEW_DIMENSION
        || u64::from(width) * u64::from(height) > MAX_EMBEDDED_PREVIEW_PIXELS
    {
        return Err("decoded image exceeds pixel limit".to_string());
    }
    let mut png = Cursor::new(Vec::new());
    image
        .write_to(&mut png, ImageFormat::Png)
        .map_err(|error| error.to_string())?;
    Ok(ImageResourceData {
        mime_type: "image/png".to_string(),
        data_base64: BASE64.encode(png.into_inner()),
        pixel_width: width,
        pixel_height: height,
        source_name: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{write::ZlibEncoder, Compression};
    use image::{Rgba, RgbaImage};
    use std::io::Write;

    fn png() -> Vec<u8> {
        let mut output = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(RgbaImage::from_pixel(2, 3, Rgba([1, 2, 3, 255])))
            .write_to(&mut output, ImageFormat::Png)
            .unwrap();
        output.into_inner()
    }

    #[test]
    fn all_compound_container_branches_extract_an_embedded_png() {
        let png = png();
        let mut emf = vec![0; 44];
        emf[..4].copy_from_slice(&1u32.to_le_bytes());
        emf[40..44].copy_from_slice(b" EMF");
        emf.extend_from_slice(&png);
        let mut wmf = Vec::from(0x9ac6_cdd7u32.to_le_bytes());
        wmf.extend_from_slice(&png);
        let mut ole = Vec::from(b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1".as_slice());
        ole.extend_from_slice(&png);
        let mut pdf = Vec::from(b"%PDF-1.7\n".as_slice());
        pdf.extend_from_slice(&png);
        let mut pict = vec![0; 10];
        pict.extend_from_slice(&[0, 0x11, 0x02, 0xff]);
        pict.extend_from_slice(&png);
        for (format, bytes) in [
            ("EnhancedMetafile", emf),
            ("WindowsMetafile", wmf),
            ("OLEObject", ole),
            ("PDF", pdf),
            ("MacPICT", pict),
        ] {
            let result = extract_embedded_preview(format, &bytes, None);
            assert_eq!(result.status, EmbeddedPreviewStatus::Decoded, "{format}");
            assert_eq!(
                result
                    .preview
                    .map(|preview| (preview.pixel_width, preview.pixel_height)),
                Some((2, 3))
            );
        }
    }

    #[test]
    fn compressed_branch_checks_declared_size_and_signature() {
        let png = png();
        let mut emf = vec![0; 44];
        emf[..4].copy_from_slice(&1u32.to_le_bytes());
        emf[40..44].copy_from_slice(b" EMF");
        emf.extend_from_slice(&png);
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&emf).unwrap();
        let compressed = encoder.finish().unwrap();
        assert_eq!(
            extract_embedded_preview(
                "CompressedEnhancedMetafile",
                &compressed,
                Some(emf.len() as u64)
            )
            .status,
            EmbeddedPreviewStatus::Decoded
        );
        assert_eq!(
            extract_embedded_preview(
                "CompressedEnhancedMetafile",
                &compressed,
                Some(emf.len() as u64 + 1)
            )
            .status,
            EmbeddedPreviewStatus::DecodeError
        );
    }

    #[test]
    fn invalid_and_previewless_containers_have_distinct_states() {
        assert_eq!(
            extract_embedded_preview("PDF", b"not pdf", None).status,
            EmbeddedPreviewStatus::InvalidSignature
        );
        assert_eq!(
            extract_embedded_preview("PDF", b"%PDF-1.7\n%%EOF", None).status,
            EmbeddedPreviewStatus::NoPreview
        );
    }
}
