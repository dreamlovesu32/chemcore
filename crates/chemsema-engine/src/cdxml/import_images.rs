use super::*;

pub(in crate::cdxml) fn append_embedded_image_objects(
    root: &XmlNode,
    objects: &mut Vec<SceneObject>,
    resources: &mut BTreeMap<String, Resource>,
) {
    let mut index = 1usize;
    for node in descendants(root) {
        if !node.is("embeddedobject") || node.attr("SupersededBy").is_some() {
            continue;
        }
        let Some(bounds) = parse_bbox(node.attr("BoundingBox")) else {
            continue;
        };
        let raster = [
            ("PNG", "image/png"),
            ("JPEG", "image/jpeg"),
            ("GIF", "image/gif"),
            ("BMP", "image/bmp"),
        ]
        .into_iter()
        .find_map(|(attribute, mime_type)| {
            let bytes = crate::decode_hex_bytes(node.attr(attribute)?)?;
            let (pixel_width, pixel_height) = raster_pixel_dimensions(mime_type, &bytes)?;
            if bytes.is_empty()
                || bytes.len() > MAX_EMBEDDED_IMAGE_BYTES
                || pixel_width == 0
                || pixel_height == 0
                || pixel_width > MAX_EMBEDDED_IMAGE_DIMENSION_PX
                || pixel_height > MAX_EMBEDDED_IMAGE_DIMENSION_PX
                || u64::from(pixel_width) * u64::from(pixel_height) > MAX_EMBEDDED_IMAGE_PIXELS
            {
                return None;
            }
            Some((attribute, mime_type, bytes, pixel_width, pixel_height))
        });
        let opaque = raster
            .is_none()
            .then(|| {
                [
                    "TIFF",
                    "EnhancedMetafile",
                    "CompressedEnhancedMetafile",
                    "WindowsMetafile",
                    "CompressedWindowsMetafile",
                    "OLEObject",
                    "CompressedOLEObject",
                    "PDF",
                    "MacPICT",
                ]
                .into_iter()
                .find_map(|attribute| {
                    crate::decode_hex_bytes(node.attr(attribute)?)
                        .filter(|bytes| !bytes.is_empty())
                        .map(|bytes| (attribute, bytes))
                })
            })
            .flatten();
        if raster.is_none() && opaque.is_none() {
            continue;
        }
        let width = (bounds[2] - bounds[0]).abs();
        let height = (bounds[3] - bounds[1]).abs();
        if width <= crate::EPSILON || height <= crate::EPSILON {
            continue;
        }
        let attribute = raster
            .as_ref()
            .map(|(attribute, _, _, _, _)| *attribute)
            .or_else(|| opaque.as_ref().map(|(attribute, _)| *attribute))
            .unwrap_or("EmbeddedObject");
        let resource_id = format!("image_cdxml_{index:03}");
        let object_id = format!("obj_image_{index:03}");
        let (resource_type, data) =
            if let Some((_, mime_type, bytes, pixel_width, pixel_height)) = raster {
                let image = crate::ImageResourceData {
                    mime_type: mime_type.to_string(),
                    data_base64: BASE64.encode(&bytes),
                    pixel_width,
                    pixel_height,
                    source_name: None,
                };
                let Ok(data) = serde_json::to_value(image) else {
                    continue;
                };
                ("image", data)
            } else {
                let (_, bytes) = opaque.expect("opaque embedded payload was checked above");
                (
                    "embedded-object",
                    json!({ "format": attribute, "dataBase64": BASE64.encode(bytes) }),
                )
            };
        resources.insert(
            resource_id.clone(),
            Resource {
                resource_type: resource_type.to_string(),
                encoding: "base64".to_string(),
                data: ResourceData::Json(data),
                meta: json!({
                    "import": { "cdxml": { "id": node.attr("id"), "attribute": attribute } }
                }),
            },
        );
        objects.push(SceneObject {
            id: object_id,
            object_type: "image".to_string(),
            name: if resource_type == "image" {
                "embedded image"
            } else {
                "embedded object"
            }
            .to_string(),
            visible: true,
            locked: false,
            z_index: parse_i32(node.attr("Z")).unwrap_or(0),
            transform: Transform {
                translate: [round2(bounds[0]), round2(bounds[1])],
                rotate: parse_f64(node.attr("RotationAngle")).unwrap_or(0.0),
                scale: [1.0, 1.0],
            },
            style_ref: None,
            link_policy: Default::default(),
            meta: json!({
                "kind": "image",
                "import": { "cdxml": { "id": node.attr("id"), "attribute": attribute } }
            }),
            payload: ObjectPayload {
                resource_ref: Some(resource_id),
                bbox: Some([0.0, 0.0, round2(width), round2(height)]),
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
        index += 1;
    }
}

pub(super) fn raster_pixel_dimensions(mime_type: &str, bytes: &[u8]) -> Option<(u32, u32)> {
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
            u32::from_le_bytes(bytes.get(18..22)?.try_into().ok()?),
            i32::from_le_bytes(bytes.get(22..26)?.try_into().ok()?).unsigned_abs(),
        )),
        "image/jpeg" => jpeg_pixel_dimensions(bytes),
        _ => None,
    }
}

pub(super) fn jpeg_pixel_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
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
            let height =
                u16::from_be_bytes(bytes.get(offset + 3..offset + 5)?.try_into().ok()?) as u32;
            let width =
                u16::from_be_bytes(bytes.get(offset + 5..offset + 7)?.try_into().ok()?) as u32;
            return (width > 0 && height > 0).then_some((width, height));
        }
        offset += length;
    }
    None
}
