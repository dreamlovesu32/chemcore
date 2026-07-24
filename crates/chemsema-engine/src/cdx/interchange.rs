use super::*;

pub(super) fn interchange_object_from_cdx(node: &CdxNode) -> InterchangeObject {
    let mut properties = BTreeMap::new();
    for (order, property) in node.properties.iter().enumerate() {
        let (official_name, cdx_type) = official_property_info(property.tag)
            .unwrap_or_else(|| (format!("tag_{:04X}", property.tag), "unknown".to_string()));
        let lexical = if property.tag == 0x0700 {
            node.text.clone().unwrap_or_default()
        } else {
            decode_property(property.tag, &property.data, None)
                .map(|(_, value)| value)
                .or_else(|| decode_official_lexical(&cdx_type, &property.data))
                .or_else(|| node.attrs.get(&official_name).cloned())
                .unwrap_or_default()
        };
        let storage_name = unique_property_storage_name(&properties, &official_name);
        properties.insert(
            storage_name,
            InterchangeProperty {
                name: official_name,
                order,
                value_type: Some(cdx_value_type(&cdx_type).to_string()),
                value: lexical,
                cdx_tag: Some(format!("0x{:04X}", property.tag)),
                cdx_type: Some(cdx_type),
                raw_base64: Some(BASE64.encode(&property.data)),
            },
        );
    }
    InterchangeObject {
        name: node.name.clone(),
        format_tag: Some(format!("0x{:04X}", node.tag)),
        id: Some(node.id.to_string()),
        properties,
        text: node.text.clone().unwrap_or_default(),
        children: node
            .children
            .iter()
            .map(interchange_object_from_cdx)
            .collect(),
    }
}

pub(super) fn unique_property_storage_name(
    properties: &BTreeMap<String, InterchangeProperty>,
    name: &str,
) -> String {
    if !properties.contains_key(name) {
        return name.to_string();
    }
    let mut occurrence = 2usize;
    loop {
        let candidate = format!("{name}#{occurrence}");
        if !properties.contains_key(&candidate) {
            return candidate;
        }
        occurrence += 1;
    }
}

pub(super) fn official_property_info(tag: u16) -> Option<(String, String)> {
    static SCHEMA: OnceLock<serde_json::Value> = OnceLock::new();
    let schema = SCHEMA.get_or_init(|| {
        serde_json::from_str(include_str!("../../schemas/cdx-cdxml-official-v1.json"))
            .expect("embedded official CDX/CDXML schema must be valid JSON")
    });
    schema
        .pointer("/cdx/properties")?
        .as_array()?
        .iter()
        .find(|property| {
            property
                .get("tag")
                .and_then(serde_json::Value::as_str)
                .and_then(|value| u16::from_str_radix(value.trim_start_matches("0x"), 16).ok())
                == Some(tag)
        })
        .map(|property| {
            (
                property
                    .get("cdxmlName")
                    .and_then(serde_json::Value::as_str)
                    .or_else(|| property.get("sdkName").and_then(serde_json::Value::as_str))
                    .unwrap_or("unknown")
                    .trim_start_matches("kCDXProp_")
                    .to_string(),
                property
                    .get("cdxType")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
            )
        })
}

pub(super) fn official_property_tag_and_type(name: &str) -> Option<(u16, String)> {
    static SCHEMA: OnceLock<serde_json::Value> = OnceLock::new();
    let schema = SCHEMA.get_or_init(|| {
        serde_json::from_str(include_str!("../../schemas/cdx-cdxml-official-v1.json"))
            .expect("embedded official CDX/CDXML schema must be valid JSON")
    });
    let property = schema
        .pointer("/cdx/properties")?
        .as_array()?
        .iter()
        .find(|property| {
            property
                .get("cdxmlName")
                .and_then(serde_json::Value::as_str)
                == Some(name)
        })?;
    let tag = property
        .get("tag")
        .and_then(serde_json::Value::as_str)
        .and_then(parse_hex_u16)?;
    let cdx_type = property
        .get("cdxType")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    Some((tag, cdx_type))
}

pub(super) fn cdx_value_type(cdx_type: &str) -> &'static str {
    match cdx_type {
        "CDXBoolean" | "CDXBooleanImplied" => "boolean",
        "INT8" | "UINT8" | "INT16" | "UINT16" | "INT32" | "UINT32" | "FLOAT64"
        | "CDXCoordinate" => "number",
        "CDXPoint2D" | "CDXPoint3D" | "CDXRectangle" | "CDXCurvePoints" | "CDXCurvePoints3D" => {
            "number-list"
        }
        "CDXDate" => "date-time-tuple",
        "CDXElementList" => "element-list",
        "CDXGenericList" => "string-list",
        "CDXObjectID" => "object-reference",
        "CDXObjectIDArray" | "CDXObjectIDArrayWithCounts" => "object-reference-list",
        "CDXRepresentsProperty" => "object-property-reference",
        "CDXString" => "string",
        _ => "binary",
    }
}

pub(super) fn encode_hex_bytes(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len() * 2);
    for byte in data {
        write!(&mut out, "{byte:02X}").expect("writing to a string cannot fail");
    }
    out
}

pub(crate) fn decode_hex_bytes(value: &str) -> Option<Vec<u8>> {
    let compact: String = value
        .chars()
        .filter(|character| !character.is_ascii_whitespace())
        .collect();
    if !compact.len().is_multiple_of(2) {
        return None;
    }
    (0..compact.len())
        .step_by(2)
        .map(|offset| u8::from_str_radix(&compact[offset..offset + 2], 16).ok())
        .collect()
}

pub(super) fn decode_official_lexical(cdx_type: &str, data: &[u8]) -> Option<String> {
    Some(match cdx_type {
        "CDXString" => parse_cdx_string(data, None).text,
        "CDXBoolean" => bool_from_bytes(data),
        "CDXBooleanImplied" => "yes".to_string(),
        "INT8" => read_i8(data)?.to_string(),
        "UINT8" => read_u8(data)?.to_string(),
        "INT16" => read_i16(data)?.to_string(),
        "UINT16" => read_u16(data)?.to_string(),
        "INT32" => read_i32(data)?.to_string(),
        "UINT32" | "CDXObjectID" => read_u32(data)?.to_string(),
        "FLOAT64" => fmt_num(read_f64(data)?),
        "CDXCoordinate" => decode_coordinate(data)?,
        "CDXPoint2D" => decode_point2d(data)?,
        "CDXPoint3D" => decode_point3d(data)?,
        "CDXRectangle" => decode_rectangle(data)?,
        "CDXObjectIDArray" => decode_u32_array(data)?,
        "CDXObjectIDArrayWithCounts" => decode_u32_counted_array(data)?,
        "INT16ListWithCounts" => decode_i16_counted_list(data)?,
        "CDXElementList" => decode_element_list(data)?,
        "CDXCurvePoints" => decode_curve_points(data, 2)?,
        "CDXCurvePoints3D" => decode_curve_points(data, 3)?,
        "CDXDate" => decode_cdx_date(data)?,
        "CDXRepresentsProperty" => decode_represents_property(data)?,
        "CDXGenericList" => decode_generic_list(data)?,
        "Unformatted" => encode_hex_bytes(data),
        _ => return None,
    })
}

pub(super) fn encode_official_lexical(cdx_type: &str, value: &str) -> Option<Vec<u8>> {
    Some(match cdx_type {
        "CDXString" => encode_plain_cdx_string(value),
        "CDXBoolean" => vec![if yes(value) { 1 } else { 0 }],
        "CDXBooleanImplied" if yes(value) => Vec::new(),
        "INT8" => vec![value.parse::<i8>().ok()? as u8],
        "UINT8" => vec![value.parse::<u8>().ok()?],
        "INT16" => value.parse::<i16>().ok()?.to_le_bytes().to_vec(),
        "UINT16" => value.parse::<u16>().ok()?.to_le_bytes().to_vec(),
        "INT32" => value.parse::<i32>().ok()?.to_le_bytes().to_vec(),
        "UINT32" | "CDXObjectID" => value.parse::<u32>().ok()?.to_le_bytes().to_vec(),
        "FLOAT64" => value.parse::<f64>().ok()?.to_le_bytes().to_vec(),
        "CDXCoordinate" => encode_coordinate(value)?,
        "CDXPoint2D" => encode_point2d(value)?,
        "CDXPoint3D" => encode_point3d(value)?,
        "CDXRectangle" => encode_rectangle(value)?,
        "CDXObjectIDArray" => encode_u32_array(value)?,
        "CDXObjectIDArrayWithCounts" => encode_u32_counted_array(value)?,
        "INT16ListWithCounts" => encode_i16_counted_list(value)?,
        "CDXElementList" => encode_element_list(value)?,
        "CDXCurvePoints" => encode_curve_points(value, 2)?,
        "CDXCurvePoints3D" => encode_curve_points(value, 3)?,
        "CDXDate" => encode_cdx_date(value)?,
        "CDXRepresentsProperty" => encode_represents_property(value)?,
        "CDXGenericList" => encode_generic_list(value)?,
        "Unformatted" => decode_hex_bytes(value)?,
        _ => return None,
    })
}
