use super::Engine;
use crate::{
    ImageCropRect, ImageResourceData, ObjectPayload, Point, Resource, ResourceData, SceneObject,
    SelectionState, Tool, Transform,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde_json::{json, Value as JsonValue};
use std::collections::BTreeMap;

const MAX_IMAGE_BYTES: usize = 64 * 1024 * 1024;
const MAX_IMAGE_DIMENSION_PX: u32 = 32_768;
const MAX_IMAGE_PIXELS: u64 = 100_000_000;
const MIN_IMAGE_SIZE_PT: f64 = 0.1;

fn normalized_raster_mime_type(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "image/png" => Some("image/png"),
        "image/jpeg" | "image/jpg" => Some("image/jpeg"),
        "image/gif" => Some("image/gif"),
        "image/bmp" | "image/x-ms-bmp" => Some("image/bmp"),
        _ => None,
    }
}

pub(crate) fn validated_image_resource_data(
    mime_type: &str,
    data_base64: &str,
    pixel_width: u32,
    pixel_height: u32,
    source_name: Option<&str>,
) -> Option<ImageResourceData> {
    let mime_type = normalized_raster_mime_type(mime_type)?;
    if pixel_width == 0
        || pixel_height == 0
        || pixel_width > MAX_IMAGE_DIMENSION_PX
        || pixel_height > MAX_IMAGE_DIMENSION_PX
        || u64::from(pixel_width) * u64::from(pixel_height) > MAX_IMAGE_PIXELS
    {
        return None;
    }
    let compact: String = data_base64
        .chars()
        .filter(|character| !character.is_ascii_whitespace())
        .collect();
    let decoded = BASE64.decode(compact.as_bytes()).ok()?;
    if decoded.is_empty()
        || decoded.len() > MAX_IMAGE_BYTES
        || image_pixel_dimensions(mime_type, &decoded) != Some((pixel_width, pixel_height))
    {
        return None;
    }
    Some(ImageResourceData {
        mime_type: mime_type.to_string(),
        data_base64: compact,
        pixel_width,
        pixel_height,
        source_name: source_name
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(ToString::to_string),
    })
}

fn image_pixel_dimensions(mime_type: &str, bytes: &[u8]) -> Option<(u32, u32)> {
    match mime_type {
        "image/png" if bytes.get(..8) == Some(b"\x89PNG\r\n\x1a\n") => Some((
            u32::from_be_bytes(bytes.get(16..20)?.try_into().ok()?),
            u32::from_be_bytes(bytes.get(20..24)?.try_into().ok()?),
        )),
        "image/gif"
            if bytes
                .get(..6)
                .is_some_and(|header| header == b"GIF87a" || header == b"GIF89a") =>
        {
            Some((
                u16::from_le_bytes(bytes.get(6..8)?.try_into().ok()?) as u32,
                u16::from_le_bytes(bytes.get(8..10)?.try_into().ok()?) as u32,
            ))
        }
        "image/bmp" if bytes.get(..2) == Some(b"BM") => Some((
            i32::from_le_bytes(bytes.get(18..22)?.try_into().ok()?).unsigned_abs(),
            i32::from_le_bytes(bytes.get(22..26)?.try_into().ok()?).unsigned_abs(),
        )),
        "image/jpeg" => jpeg_pixel_dimensions(bytes),
        _ => None,
    }
}

fn jpeg_pixel_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.get(..2) != Some(&[0xff, 0xd8]) {
        return None;
    }
    let mut offset = 2usize;
    while offset + 4 <= bytes.len() {
        if bytes[offset] != 0xff {
            offset += 1;
            continue;
        }
        let marker = bytes[offset + 1];
        offset += 2;
        if matches!(marker, 0xd8 | 0xd9) || (0xd0..=0xd7).contains(&marker) {
            continue;
        }
        let length = u16::from_be_bytes(bytes.get(offset..offset + 2)?.try_into().ok()?) as usize;
        if length < 2 || offset + length > bytes.len() {
            return None;
        }
        if matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf) {
            return Some((
                u16::from_be_bytes(bytes.get(offset + 5..offset + 7)?.try_into().ok()?) as u32,
                u16::from_be_bytes(bytes.get(offset + 3..offset + 5)?.try_into().ok()?) as u32,
            ));
        }
        offset += length;
    }
    None
}

impl Engine {
    pub(super) fn validate_image_crop_candidate(
        &self,
        object_id: &str,
        crop: Option<ImageCropRect>,
    ) -> Result<(), String> {
        let object = self
            .state
            .document
            .find_scene_object(object_id)
            .ok_or_else(|| format!("unknown image object {object_id}"))?;
        if object.object_type != "image" {
            return Err(format!("object {object_id} is not an image"));
        }
        let resource_ref = object
            .payload
            .resource_ref
            .as_deref()
            .ok_or_else(|| format!("image {object_id} has no resourceRef"))?;
        let image = self
            .state
            .document
            .resources
            .get(resource_ref)
            .and_then(Resource::display_image)
            .ok_or_else(|| format!("image {object_id} has no decodable preview to crop"))?;
        if let Some(crop) = crop {
            crop.validate(image.pixel_width, image.pixel_height)?;
        }
        Ok(())
    }

    pub(super) fn set_image_crop_direct(
        &mut self,
        object_id: &str,
        crop: Option<ImageCropRect>,
    ) -> bool {
        let object = self
            .state
            .document
            .find_scene_object_mut(object_id)
            .expect("image object was checked above");
        if object.payload.image_crop().ok().flatten() == crop {
            return false;
        }
        object.payload.set_image_crop(crop);
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_image_direct(
        &mut self,
        mime_type: &str,
        data_base64: &str,
        pixel_width: u32,
        pixel_height: u32,
        center: Point,
        width: f64,
        height: f64,
        source_name: Option<&str>,
    ) -> bool {
        let Some(image) = validated_image_resource_data(
            mime_type,
            data_base64,
            pixel_width,
            pixel_height,
            source_name,
        ) else {
            return false;
        };
        if !center.x.is_finite()
            || !center.y.is_finite()
            || !width.is_finite()
            || !height.is_finite()
            || width.abs() < MIN_IMAGE_SIZE_PT
            || height.abs() < MIN_IMAGE_SIZE_PT
        {
            return false;
        }
        let width = crate::round2(width.abs());
        let height = crate::round2(height.abs());
        let object_id = self.next_id("image");
        let resource_id = format!("res_{object_id}");
        let z_index = self
            .state
            .document
            .scene_objects()
            .into_iter()
            .map(|object| object.z_index)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        let name = image
            .source_name
            .clone()
            .unwrap_or_else(|| "image".to_string());
        let Ok(data) = serde_json::to_value(&image) else {
            return false;
        };
        self.state.document.resources.insert(
            resource_id.clone(),
            Resource {
                resource_type: "image".to_string(),
                encoding: "base64".to_string(),
                data: ResourceData::Json(data),
                meta: JsonValue::Null,
            },
        );
        self.state.document.objects.push(SceneObject {
            id: object_id.clone(),
            object_type: "image".to_string(),
            name,
            visible: true,
            locked: false,
            z_index,
            transform: Transform {
                translate: [
                    crate::round2(center.x - width * 0.5),
                    crate::round2(center.y - height * 0.5),
                ],
                rotate: 0.0,
                scale: [1.0, 1.0],
            },
            style_ref: None,
            link_policy: Default::default(),
            meta: json!({ "kind": "image" }),
            payload: ObjectPayload {
                resource_ref: Some(resource_id),
                bbox: Some([0.0, 0.0, width, height]),
                spectrum: None,
                geometry: None,
                constraint: None,
                table: None,
                stoichiometry_grid: None,
                gel_electrophoresis: None,
                plasmid_map: None,
                bio_shape: None,
                extra: BTreeMap::from([
                    ("fit".to_string(), json!("stretch")),
                    ("opacity".to_string(), json!(1.0)),
                ]),
            },
            children: Vec::new(),
        });
        self.state.selection = SelectionState {
            arrow_objects: vec![object_id],
            ..SelectionState::default()
        };
        self.state.tool.active_tool = Tool::Select;
        self.clear_interaction();
        true
    }
}
