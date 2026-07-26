use crate::{
    document_to_cdxml, parse_cdxml_document, ChemSemaDocument, InterchangeDocument,
    InterchangeObject, InterchangeProperty,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::sync::OnceLock;

mod decode;
mod encode;
mod enums;
mod interchange;
mod schema;
mod text_tables;
mod utils;

use decode::*;
use encode::*;
use enums::*;
use interchange::*;
use schema::*;
use text_tables::*;
use utils::*;

pub(crate) use interchange::decode_hex_bytes;

const CDX_HEADER: &[u8; 22] = b"VjCD0100\x04\x03\x02\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
const CDX_COORD_FACTOR: f64 = 65_536.0;

pub fn parse_cdx_document(bytes: &[u8], title: Option<&str>) -> Result<ChemSemaDocument, String> {
    let tree = CdxReader::new(bytes).read()?;
    let cdxml = CdxmlWriter::new().write(&tree);
    let interchange = interchange_object_from_cdx(&tree);
    let mut document = parse_cdxml_document(&cdxml, title)?;
    document.interchange.insert(
        "cdx".to_string(),
        InterchangeDocument {
            format: "cdx".to_string(),
            version: Some("0100".to_string()),
            root: interchange,
        },
    );
    document.document.meta["sourceFormat"] = json!("cdx");
    if let Some(import) = document.document.meta.get_mut("import") {
        import["cdx"] = json!({ "nativeImport": true });
    }
    Ok(document)
}

pub fn document_to_cdx(document: &ChemSemaDocument) -> Result<Vec<u8>, String> {
    validate_cdx_chemical_property_types(document)?;
    if document
        .scene_objects()
        .into_iter()
        .any(|object| object.object_type == "stoichiometry-grid")
    {
        return Err(
            "CDX has no verified StoichiometryGrid object or property tags; save as CCJS or CDXML"
                .to_string(),
        );
    }
    let cdxml = document_to_cdxml(document);
    let mut root = crate::cdxml::parse_xml_tree(&cdxml)?;
    let source = document.interchange.get("cdx").map(|source| &source.root);
    if let Some(source) = source {
        overlay_unmodeled_cdx_values(&mut root, source);
    }
    CdxWriter::new(source).write(&root)
}

fn validate_cdx_chemical_property_types(document: &ChemSemaDocument) -> Result<(), String> {
    for property in &document.chemical_properties {
        if property.property_type.code.is_none() && property.property_type.name.is_some() {
            return Err(format!(
                "chemical property '{}' has a CDXML-only named custom type; CDX requires a numeric custom type code greater than 0x8000",
                property.id
            ));
        }
    }
    Ok(())
}

pub fn cdx_to_cdxml(bytes: &[u8]) -> Result<String, String> {
    let tree = CdxReader::new(bytes).read()?;
    Ok(CdxmlWriter::new().write(&tree))
}

pub fn cdxml_to_cdx(cdxml: &str) -> Result<Vec<u8>, String> {
    let root = crate::cdxml::parse_xml_tree(cdxml)?;
    CdxWriter::new(None).write(&root)
}

#[derive(Debug, Clone)]
struct CdxNode {
    name: String,
    tag: u16,
    id: u32,
    attrs: BTreeMap<String, String>,
    properties: Vec<CdxRawProperty>,
    text_runs: Vec<CdxTextRun>,
    text: Option<String>,
    children: Vec<CdxNode>,
}

#[derive(Debug, Clone)]
struct CdxRawProperty {
    tag: u16,
    data: Vec<u8>,
}

#[derive(Debug, Clone)]
struct CdxTextRun {
    start: usize,
    font: u16,
    face: u16,
    size: f64,
    color: u16,
}

fn decode_plasmid_property(
    object_name: &str,
    tag: u16,
    data: &[u8],
) -> Option<(&'static str, String)> {
    let name = match (object_name, tag) {
        ("plasmidmap", 0x1300) => "NumberBasePairs",
        ("plasmidmap", 0x1307) => "RingRadius",
        ("plasmidregion", 0x1304) => "RegionStart",
        ("plasmidregion", 0x1305) => "RegionEnd",
        ("plasmidregion", 0x1306) => "RegionOffset",
        ("plasmidmarker", 0x1302) => "MarkerOffset",
        ("plasmidmarker", 0x1303) => "MarkerAngle",
        ("plasmidmarker", 0x0D05) => "Value",
        _ => return None,
    };
    let bytes: [u8; 4] = data.try_into().ok()?;
    Some((name, i32::from_le_bytes(bytes).to_string()))
}

fn encode_plasmid_property(
    object_name: &str,
    property_name: &str,
    value: &str,
) -> Option<(u16, Vec<u8>)> {
    let tag = match (object_name, property_name) {
        ("plasmidmap", "NumberBasePairs") => 0x1300,
        ("plasmidmap", "RingRadius") => 0x1307,
        ("plasmidregion", "RegionStart") => 0x1304,
        ("plasmidregion", "RegionEnd") => 0x1305,
        ("plasmidregion", "RegionOffset") => 0x1306,
        ("plasmidmarker", "MarkerOffset") => 0x1302,
        ("plasmidmarker", "MarkerAngle") => 0x1303,
        ("plasmidmarker", "Value") => 0x0D05,
        _ => return None,
    };
    let numeric = value.parse::<i32>().ok()?;
    Some((tag, numeric.to_le_bytes().to_vec()))
}

struct CdxReader<'a> {
    bytes: &'a [u8],
    offset: usize,
    font_table: Option<FontTable>,
    legacy_chemsema_object_tags: bool,
}

impl<'a> CdxReader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            offset: 0,
            font_table: None,
            // ChemDraw 12 and ChemDraw 21 both write the long-established
            // 0x801B..0x802B object registry (Arrow=0x8021).  A previous
            // ChemSema beta followed the shifted static SDK table instead;
            // its explicit marker is handled as a compatibility dialect.
            legacy_chemsema_object_tags: true,
        }
    }

    fn read(mut self) -> Result<CdxNode, String> {
        if self.bytes.len() < CDX_HEADER.len() || &self.bytes[..CDX_HEADER.len()] != CDX_HEADER {
            return Err("invalid CDX header".to_string());
        }
        self.offset = CDX_HEADER.len();
        let root = self.read_object()?;
        self.consume_file_end_marker();
        Ok(root)
    }

    fn read_object(&mut self) -> Result<CdxNode, String> {
        let tag = self.read_u16()?;
        let id = self.read_u32()?;
        let name = if self.legacy_chemsema_object_tags {
            legacy_chemsema_object_name(tag).or_else(|| object_name(tag))
        } else {
            object_name(tag)
        }
        .unwrap_or("unknown")
        .to_string();
        let mut node = CdxNode {
            name,
            tag,
            id,
            attrs: BTreeMap::new(),
            properties: Vec::new(),
            text_runs: Vec::new(),
            text: None,
            children: Vec::new(),
        };
        node.attrs.insert("id".to_string(), id.to_string());

        loop {
            let tag = self.read_u16()?;
            if tag == 0 {
                break;
            }
            if is_object_tag(tag) {
                self.offset -= 2;
                node.children.push(self.read_object()?);
                continue;
            }
            let len = self.read_property_len()?;
            let data = self.read_bytes(len)?;
            node.properties.push(CdxRawProperty {
                tag,
                data: data.to_vec(),
            });
            self.apply_property(&mut node, tag, data)?;
        }
        Ok(node)
    }

    fn apply_property(&mut self, node: &mut CdxNode, tag: u16, data: &[u8]) -> Result<(), String> {
        if let Some((name, value)) = decode_plasmid_property(&node.name, tag, data) {
            node.attrs.insert(name.to_string(), value);
            return Ok(());
        }
        if node.name == "spectrum" && tag == 0x0A86 {
            if data.is_empty() || !data.len().is_multiple_of(8) {
                return Err(format!(
                    "invalid CDX Spectrum_DataPoint payload length {}",
                    data.len()
                ));
            }
            let text = node.text.get_or_insert_with(String::new);
            for chunk in data.chunks_exact(8) {
                let value = f64::from_le_bytes(
                    chunk
                        .try_into()
                        .expect("chunks_exact(8) yields eight-byte slices"),
                );
                if !value.is_finite() {
                    return Err("CDX Spectrum_DataPoint contains a non-finite value".to_string());
                }
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(&fmt_num(value));
            }
            return Ok(());
        }
        if tag == 0x0100 {
            if let Some(table) = parse_font_table(data) {
                self.font_table = Some(table.clone());
                node.children.push(table.into_node());
            }
            return Ok(());
        }
        if tag == 0x0300 {
            if let Some(table) = parse_color_table(data) {
                node.children.push(table.into_node());
            }
            return Ok(());
        }
        if tag == 0x0700 {
            let text = parse_cdx_string(data, self.font_table.as_ref());
            node.text = Some(text.text);
            node.text_runs = text.runs;
            return Ok(());
        }
        if tag == 0x000E {
            if let Some(value) = decode_represents_property(data) {
                let mut parts = value.split_whitespace();
                if let (Some(object), Some(property_tag)) = (parts.next(), parts.next()) {
                    let attribute = parse_hex_u16(property_tag)
                        .and_then(official_property_info)
                        .map(|(name, _)| name)
                        .unwrap_or_else(|| property_tag.to_string());
                    let mut attrs = BTreeMap::new();
                    attrs.insert("object".to_string(), object.to_string());
                    attrs.insert("attribute".to_string(), attribute);
                    node.children.push(CdxNode {
                        name: "represent".to_string(),
                        tag,
                        id: 0,
                        attrs,
                        properties: Vec::new(),
                        text_runs: Vec::new(),
                        text: None,
                        children: Vec::new(),
                    });
                }
            }
            return Ok(());
        }
        if tag == 0x080A || tag == 0x080B {
            if let Some((font, face, size, color)) = decode_font_style(data) {
                let prefix = if tag == 0x080A { "Label" } else { "Caption" };
                if font != u16::MAX {
                    node.attrs.insert(format!("{prefix}Font"), font.to_string());
                }
                if face != u16::MAX {
                    node.attrs.insert(format!("{prefix}Face"), face.to_string());
                }
                if size != u16::MAX as f64 / 20.0 {
                    node.attrs.insert(format!("{prefix}Size"), fmt_num(size));
                }
                if color != u16::MAX {
                    node.attrs
                        .insert(format!("{prefix}Color"), color.to_string());
                }
            }
            return Ok(());
        }
        if let Some((name, value)) = decode_property(tag, data, self.font_table.as_ref()) {
            if tag == 0x0003 && value.trim() == "ChemSema" {
                self.legacy_chemsema_object_tags = true;
            }
            if tag == 0x0006
                && value.starts_with("ChemSema/")
                && value.contains("cdx-tags=official")
            {
                self.legacy_chemsema_object_tags = false;
            }
            if tag == 0x0006 && value.contains("cdx-tags=chemdraw") {
                self.legacy_chemsema_object_tags = true;
            }
            node.attrs.insert(name.to_string(), value);
        } else if let Some((name, cdx_type)) = official_property_info(tag) {
            if let Some(value) = decode_official_lexical(&cdx_type, data) {
                node.attrs.insert(name, value);
            }
        }
        Ok(())
    }

    fn read_u16(&mut self) -> Result<u16, String> {
        let bytes = self.read_bytes(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn read_u32(&mut self) -> Result<u32, String> {
        let bytes = self.read_bytes(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_property_len(&mut self) -> Result<usize, String> {
        let short = self.read_u16()?;
        if short == 0xFFFF {
            Ok(self.read_u32()? as usize)
        } else {
            Ok(short as usize)
        }
    }

    fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], String> {
        if self.offset + len > self.bytes.len() {
            return Err("unexpected end of CDX stream".to_string());
        }
        let start = self.offset;
        self.offset += len;
        Ok(&self.bytes[start..start + len])
    }

    fn consume_file_end_marker(&mut self) {
        if self.offset + 2 <= self.bytes.len() {
            let marker = u16::from_le_bytes([self.bytes[self.offset], self.bytes[self.offset + 1]]);
            if marker == 0 {
                self.offset += 2;
            }
        }
    }
}

struct CdxmlWriter;

impl CdxmlWriter {
    fn new() -> Self {
        Self
    }

    fn write(&self, root: &CdxNode) -> String {
        let mut out = String::new();
        out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n");
        out.push_str("<!DOCTYPE CDXML SYSTEM \"http://www.cambridgesoft.com/xml/cdxml.dtd\" >\n");
        self.write_node(&mut out, root, 0);
        out
    }

    fn write_node(&self, out: &mut String, node: &CdxNode, indent: usize) {
        let tag_name = node.name.as_str();
        if tag_name == "fonttable" || tag_name == "colortable" {
            self.write_table_node(out, node, indent);
            return;
        }
        self.write_indent(out, indent);
        write!(out, "<{tag_name}").expect("write CDXML node");
        let mut attrs: Vec<_> = node.attrs.iter().collect();
        attrs.sort_by(|a, b| {
            attr_order(a.0)
                .cmp(&attr_order(b.0))
                .then_with(|| a.0.cmp(b.0))
        });
        for (key, value) in attrs {
            if key.starts_with("CDXRaw") {
                continue;
            }
            write!(out, " {key}=\"{}\"", escape_attr(value)).expect("write CDXML attr");
        }
        if node.children.is_empty() && node.text.is_none() {
            out.push_str("/>\n");
            return;
        }
        out.push_str(">\n");
        if let Some(text) = &node.text {
            self.write_text_runs(out, text, &node.text_runs, indent + 2);
        }
        for child in &node.children {
            self.write_node(out, child, indent + 2);
        }
        self.write_indent(out, indent);
        writeln!(out, "</{tag_name}>").expect("write CDXML node close");
    }

    fn write_table_node(&self, out: &mut String, node: &CdxNode, indent: usize) {
        self.write_indent(out, indent);
        writeln!(out, "<{}>", node.name).expect("write table open");
        for child in &node.children {
            self.write_node(out, child, indent + 2);
        }
        self.write_indent(out, indent);
        writeln!(out, "</{}>", node.name).expect("write table close");
    }

    fn write_text_runs(&self, out: &mut String, text: &str, runs: &[CdxTextRun], indent: usize) {
        if runs.is_empty() {
            self.write_indent(out, indent);
            writeln!(out, "{}", escape_text(text)).expect("write text");
            return;
        }
        let mut sorted = runs.to_vec();
        sorted.sort_by_key(|run| run.start);
        let char_count = text.chars().count();
        for (index, run) in sorted.iter().enumerate() {
            let start = run.start.min(char_count);
            let end = sorted
                .get(index + 1)
                .map(|next| next.start.min(char_count))
                .unwrap_or(char_count)
                .max(start);
            let slice: String = text.chars().skip(start).take(end - start).collect();
            self.write_indent(out, indent);
            out.push_str("<s");
            // CDXString uses 0xFFFF as an inheritance sentinel for each
            // style component. Emitting it as a literal face (65535) turns
            // every style bit on; omitting the CDXML attribute preserves the
            // containing text/document style, as ChemDraw does.
            if run.font != u16::MAX {
                write!(out, " font=\"{}\"", run.font).expect("write text font");
            }
            if run.size != u16::MAX as f64 / 20.0 {
                write!(out, " size=\"{}\"", fmt_num(run.size)).expect("write text size");
            }
            if run.face != u16::MAX {
                write!(out, " face=\"{}\"", run.face).expect("write text face");
            }
            if run.color != u16::MAX {
                write!(out, " color=\"{}\"", run.color).expect("write text color");
            }
            writeln!(out, ">{}</s>", escape_text(&slice)).expect("write styled text");
        }
    }

    fn write_indent(&self, out: &mut String, indent: usize) {
        for _ in 0..indent {
            out.push(' ');
        }
    }
}

struct CdxWriter<'a> {
    next_id: u32,
    source: Option<&'a InterchangeObject>,
}

impl<'a> CdxWriter<'a> {
    fn new(source: Option<&'a InterchangeObject>) -> Self {
        Self {
            next_id: 5000,
            source,
        }
    }

    fn write(mut self, root: &crate::cdxml::xml::XmlNode) -> Result<Vec<u8>, String> {
        let mut out = Vec::new();
        out.extend_from_slice(CDX_HEADER);
        self.write_object(root, self.source, &mut out)?;
        out.extend_from_slice(&[0, 0, 0, 0]);
        Ok(out)
    }

    fn write_object(
        &mut self,
        node: &crate::cdxml::xml::XmlNode,
        source: Option<&InterchangeObject>,
        out: &mut Vec<u8>,
    ) -> Result<(), String> {
        let Some(tag) = object_tag(&node.name) else {
            return Ok(());
        };
        write_u16(out, tag);
        write_u32(out, self.xml_id(node));
        let mut written_properties = BTreeSet::new();

        for (key, value) in &node.attrs {
            if key == "id" {
                continue;
            }
            if node.name == "spectrum" && matches!(key.as_str(), "YLow" | "YScale") {
                // YLow/YScale are the CDXML storage transform. CDX stores the
                // decoded double array directly, as ChemDraw's own writer does.
                continue;
            }
            let spectrum_enum_value = if node.name == "spectrum" {
                match key.as_str() {
                    "Class" => Some(
                        crate::SpectrumClass::from_cdxml(Some(value))?
                            .cdx_value()
                            .to_string(),
                    ),
                    "XType" => Some(
                        crate::SpectrumXAxisType::from_cdxml(Some(value))?
                            .cdx_value()
                            .to_string(),
                    ),
                    "YType" => Some(
                        crate::SpectrumYAxisType::from_cdxml(Some(value))?
                            .cdx_value()
                            .to_string(),
                    ),
                    _ => None,
                }
            } else {
                None
            };
            if let Some((prop_tag, bytes)) = encode_plasmid_property(
                &node.name,
                key,
                spectrum_enum_value.as_deref().unwrap_or(value),
            )
            .or_else(|| encode_property(key, spectrum_enum_value.as_deref().unwrap_or(value)))
            {
                write_property(out, prop_tag, &bytes);
                written_properties.insert(prop_tag);
            }
        }

        if node.name == "CDXML" {
            if let Some(color_table) = node.direct_children("colortable").next() {
                write_property(out, 0x0300, &encode_color_table(color_table));
                written_properties.insert(0x0300);
            }
            if let Some(font_table) = node.direct_children("fonttable").next() {
                write_property(out, 0x0100, &encode_font_table(font_table));
                written_properties.insert(0x0100);
            }
        }

        if node.name == "t" {
            write_property(out, 0x0700, &encode_cdx_string(node));
            written_properties.insert(0x0700);
        }
        if node.name == "spectrum" {
            let y_low = node
                .attr("YLow")
                .map(str::parse::<f64>)
                .transpose()
                .map_err(|_| "invalid spectrum YLow while writing CDX".to_string())?
                .unwrap_or(0.0);
            let y_scale = node
                .attr("YScale")
                .map(str::parse::<f64>)
                .transpose()
                .map_err(|_| "invalid spectrum YScale while writing CDX".to_string())?
                .unwrap_or(1.0);
            let mut bytes = Vec::new();
            for token in node.full_text().split_whitespace() {
                let raw = token.parse::<f64>().map_err(|_| {
                    format!("invalid spectrum data point '{token}' while writing CDX")
                })?;
                let value = y_low + raw * y_scale;
                if !value.is_finite() {
                    return Err(format!(
                        "non-finite spectrum data point '{token}' while writing CDX"
                    ));
                }
                bytes.extend_from_slice(&value.to_le_bytes());
            }
            if bytes.is_empty() {
                return Err("spectrum has no data points while writing CDX".to_string());
            }
            write_property(out, 0x0A86, &bytes);
            written_properties.insert(0x0A86);
        }
        for represent in node.direct_children("represent") {
            let Some(object) = represent.attr("object") else {
                continue;
            };
            let Some(attribute) = represent.attr("attribute") else {
                continue;
            };
            let Some((property_tag, _)) = official_property_tag_and_type(attribute) else {
                continue;
            };
            let lexical = format!("{object} 0x{property_tag:04X}");
            if let Some(bytes) = encode_represents_property(&lexical) {
                write_property(out, 0x000E, &bytes);
                written_properties.insert(0x000E);
            }
        }

        if let Some(source) = source {
            for property in ordered_interchange_properties(source) {
                let Some(prop_tag) = property.cdx_tag.as_deref().and_then(parse_hex_u16) else {
                    continue;
                };
                if written_properties.contains(&prop_tag) {
                    continue;
                }
                let encoded = (!property.value.is_empty())
                    .then(|| {
                        property
                            .cdx_type
                            .as_deref()
                            .and_then(|kind| encode_official_lexical(kind, &property.value))
                    })
                    .flatten();
                if let Some(bytes) = encoded.or_else(|| {
                    property
                        .raw_base64
                        .as_deref()
                        .and_then(|value| BASE64.decode(value).ok())
                }) {
                    write_property(out, prop_tag, &bytes);
                }
            }
        }

        let generated_children: Vec<_> = node
            .children
            .iter()
            .filter(|child| !is_cdx_helper_name(&child.name))
            .collect();
        let mut used_generated = BTreeSet::new();
        if let Some(source) = source {
            for source_child in source
                .children
                .iter()
                .filter(|child| !is_cdx_helper_name(&child.name))
            {
                let generated_index = generated_children
                    .iter()
                    .enumerate()
                    .find(|(index, child)| {
                        !used_generated.contains(index)
                            && interchange_matches_xml(source_child, child)
                    })
                    .map(|(index, _)| index)
                    .or_else(|| {
                        generated_children
                            .iter()
                            .enumerate()
                            .find(|(index, child)| {
                                !used_generated.contains(index) && source_child.name == child.name
                            })
                            .map(|(index, _)| index)
                    });
                if let Some(index) = generated_index
                    .filter(|index| object_tag(&generated_children[*index].name).is_some())
                {
                    used_generated.insert(index);
                    self.write_object(generated_children[index], Some(source_child), out)?;
                } else {
                    self.write_raw_interchange_object(source_child, out)?;
                }
            }
        }
        for (index, child) in generated_children.into_iter().enumerate() {
            if !used_generated.contains(&index) {
                self.write_object(child, None, out)?;
            }
        }
        write_u16(out, 0);
        Ok(())
    }

    fn write_raw_interchange_object(
        &mut self,
        object: &InterchangeObject,
        out: &mut Vec<u8>,
    ) -> Result<(), String> {
        let tag = object
            .format_tag
            .as_deref()
            .and_then(parse_hex_u16)
            .or_else(|| object_tag(&object.name))
            .ok_or_else(|| format!("CDX object '{}' has no writable object tag", object.name))?;
        write_u16(out, tag);
        let id = object
            .id
            .as_deref()
            .and_then(|value| value.parse().ok())
            .unwrap_or_else(|| {
                let id = self.next_id;
                self.next_id += 1;
                id
            });
        write_u32(out, id);
        for property in ordered_interchange_properties(object) {
            let Some(prop_tag) = property.cdx_tag.as_deref().and_then(parse_hex_u16) else {
                continue;
            };
            let bytes = (!property.value.is_empty())
                .then(|| {
                    property
                        .cdx_type
                        .as_deref()
                        .and_then(|kind| encode_official_lexical(kind, &property.value))
                })
                .flatten()
                .or_else(|| {
                    property
                        .raw_base64
                        .as_deref()
                        .and_then(|value| BASE64.decode(value).ok())
                })
                .ok_or_else(|| {
                    format!(
                        "CDX property {} on object {} has no valid rawBase64 payload",
                        property.cdx_tag.as_deref().unwrap_or("unknown"),
                        object.name
                    )
                })?;
            write_property(out, prop_tag, &bytes);
        }
        for child in &object.children {
            if !matches!(
                child.name.as_str(),
                "s" | "font" | "color" | "fonttable" | "colortable" | "represent"
            ) {
                self.write_raw_interchange_object(child, out)?;
            }
        }
        write_u16(out, 0);
        Ok(())
    }

    fn xml_id(&mut self, node: &crate::cdxml::xml::XmlNode) -> u32 {
        if let Some(id) = node.attr("id").and_then(|value| value.parse::<u32>().ok()) {
            id
        } else {
            let id = self.next_id;
            self.next_id += 1;
            id
        }
    }
}

#[derive(Debug, Clone)]
struct FontRecord {
    id: u16,
    charset: u16,
    name: String,
}

#[derive(Debug, Clone)]
struct FontTable {
    fonts: Vec<FontRecord>,
}

impl FontTable {
    fn into_node(self) -> CdxNode {
        let children = self
            .fonts
            .into_iter()
            .map(|font| {
                let mut attrs = BTreeMap::new();
                attrs.insert("id".to_string(), font.id.to_string());
                attrs.insert(
                    "charset".to_string(),
                    charset_name(font.charset).to_string(),
                );
                attrs.insert("name".to_string(), font.name);
                CdxNode {
                    name: "font".to_string(),
                    tag: 0,
                    id: font.id as u32,
                    attrs,
                    properties: Vec::new(),
                    text_runs: Vec::new(),
                    text: None,
                    children: Vec::new(),
                }
            })
            .collect();
        CdxNode {
            name: "fonttable".to_string(),
            tag: 0x0100,
            id: 0,
            attrs: BTreeMap::new(),
            properties: Vec::new(),
            text_runs: Vec::new(),
            text: None,
            children,
        }
    }
}

struct ColorTable {
    colors: Vec<(u16, u16, u16)>,
}

impl ColorTable {
    fn into_node(self) -> CdxNode {
        let children = self
            .colors
            .into_iter()
            .map(|(r, g, b)| {
                let mut attrs = BTreeMap::new();
                attrs.insert("r".to_string(), fmt_num(r as f64 / 65_535.0));
                attrs.insert("g".to_string(), fmt_num(g as f64 / 65_535.0));
                attrs.insert("b".to_string(), fmt_num(b as f64 / 65_535.0));
                CdxNode {
                    name: "color".to_string(),
                    tag: 0,
                    id: 0,
                    attrs,
                    properties: Vec::new(),
                    text_runs: Vec::new(),
                    text: None,
                    children: Vec::new(),
                }
            })
            .collect();
        CdxNode {
            name: "colortable".to_string(),
            tag: 0x0300,
            id: 0,
            attrs: BTreeMap::new(),
            properties: Vec::new(),
            text_runs: Vec::new(),
            text: None,
            children,
        }
    }
}

struct ParsedText {
    text: String,
    runs: Vec<CdxTextRun>,
}

#[derive(Clone, Copy)]
struct PropertySchema {
    name: &'static str,
    kind: PropertyKind,
}

#[derive(Clone, Copy)]
enum PropertyKind {
    String,
    Binary,
    Point2D,
    Point3D,
    Rectangle,
    Coordinate,
    Int8,
    UInt8,
    Int16,
    UInt16,
    LineHeightInt16,
    LineHeightUInt16,
    Fixed16_16,
    UInt32,
    Float64,
    Boolean,
    BooleanImplied,
    BondOrder,
    BondSpacing,
    AngleTenths,
    FontStyle,
    ObjectIdArray,
    Int16ListWithCounts,
    Enum8(&'static [(i16, &'static str)]),
    Enum(&'static [(i16, &'static str)]),
    BitFlags(&'static [(i16, &'static str)]),
}

const BOND_DISPLAY: &[(i16, &str)] = &[
    (0, "Solid"),
    (1, "Dash"),
    (2, "Hash"),
    (3, "WedgedHashBegin"),
    (4, "WedgedHashEnd"),
    (5, "Bold"),
    (6, "WedgeBegin"),
    (7, "WedgeEnd"),
    (8, "Wavy"),
    (9, "HollowWedgeBegin"),
    (10, "HollowWedgeEnd"),
    (11, "WavyWedgeBegin"),
    (12, "WavyWedgeEnd"),
    (13, "Dot"),
    (14, "DashDot"),
];
const DOUBLE_POSITION: &[(i16, &str)] = &[
    (0, "Center"),
    (1, "Right"),
    (2, "Left"),
    (256, "Center"),
    (257, "Right"),
    (258, "Left"),
];
const ATOM_STEREO: &[(i16, &str)] = &[
    (0, "U"),
    (1, "N"),
    (2, "R"),
    (3, "S"),
    (4, "r"),
    (5, "s"),
    (6, "u"),
];
const ATOM_RADICAL: &[(i16, &str)] = &[(0, "None"), (1, "Singlet"), (2, "Doublet"), (3, "Triplet")];
const ISOTOPIC_ABUNDANCE: &[(i16, &str)] = &[
    (0, "Unspecified"),
    (1, "Any"),
    (2, "Natural"),
    (3, "Enriched"),
    (4, "Deficient"),
    (5, "Nonnatural"),
];
const RING_BOND_COUNT: &[(i16, &str)] = &[
    (-1, "Unspecified"),
    (0, "NoRingBonds"),
    (1, "AsDrawn"),
    (2, "SimpleRing"),
    (3, "Fusion"),
    (4, "SpiroOrHigher"),
];
const UNSATURATED_BONDS: &[(i16, &str)] = &[
    (0, "Unspecified"),
    (1, "MustBeAbsent"),
    (2, "MustBePresent"),
];
const QUERY_TRANSLATION: &[(i16, &str)] = &[(0, "Equal"), (1, "Broad"), (2, "Narrow"), (3, "Any")];
const ATOM_REACTION_STEREO: &[(i16, &str)] =
    &[(0, "Unspecified"), (1, "Inversion"), (2, "Retention")];
const BOND_TOPOLOGY: &[(i16, &str)] = &[
    (0, "Unspecified"),
    (1, "Ring"),
    (2, "Chain"),
    (3, "RingOrChain"),
];
const BOND_REACTION_PARTICIPATION: &[(i16, &str)] = &[
    (0, "Unspecified"),
    (1, "ReactionCenter"),
    (2, "MakeOrBreak"),
    (3, "ChangeType"),
    (4, "MakeAndChange"),
    (5, "NotReactionCenter"),
    (6, "NoChange"),
    (7, "Unmapped"),
];
const BOND_STEREO: &[(i16, &str)] = &[(0, "U"), (1, "N"), (2, "E"), (3, "Z")];
const NODE_TYPE: &[(i16, &str)] = &[
    (0, "Unspecified"),
    (1, "Element"),
    (2, "ElementList"),
    (3, "ElementListNickname"),
    (4, "Nickname"),
    (5, "Fragment"),
    (6, "Formula"),
    (7, "GenericNickname"),
    (8, "AnonymousAlternativeGroup"),
    (9, "NamedAlternativeGroup"),
    (10, "MultiAttachment"),
    (11, "VariableAttachment"),
    (12, "ExternalConnectionPoint"),
    (13, "LinkNode"),
];
const EXTERNAL_CONNECTION_TYPE: &[(i16, &str)] = &[
    (0, "Unspecified"),
    (1, "Diamond"),
    (2, "Star"),
    (3, "PolymerBead"),
    (4, "Wavy"),
    (5, "Residue"),
    (6, "Peptide"),
    (7, "DNA"),
    (8, "RNA"),
    (9, "Terminus"),
    (10, "Sulfide"),
    (11, "Nucleotide"),
    (12, "UnlinkedBranch"),
];
const LABEL_DISPLAY: &[(i16, &str)] = &[
    (0, "Auto"),
    (1, "Left"),
    (2, "Center"),
    (3, "Right"),
    (4, "Above"),
    (5, "Below"),
];
const JUSTIFICATION: &[(i16, &str)] = &[
    (-1, "Right"),
    (0, "Left"),
    (1, "Center"),
    (2, "Full"),
    (3, "Above"),
    (4, "Below"),
    (5, "Auto"),
    (6, "Best"),
];
const LABEL_ALIGNMENT: &[(i16, &str)] = &[
    (0, "Auto"),
    (1, "Left"),
    (2, "Center"),
    (3, "Right"),
    (4, "Above"),
    (5, "Below"),
    (6, "Best"),
];
const GRAPHIC_TYPE: &[(i16, &str)] = &[
    (1, "Line"),
    (2, "Arc"),
    (3, "Rectangle"),
    (4, "Oval"),
    (5, "Orbital"),
    (6, "Bracket"),
    (7, "Symbol"),
];
const BRACKET_TYPE: &[(i16, &str)] = &[
    (0, "RoundPair"),
    (1, "SquarePair"),
    (2, "CurlyPair"),
    (3, "Square"),
    (4, "Curly"),
    (5, "Round"),
];
const POSITIONING_TYPE: &[(i16, &str)] =
    &[(0, "auto"), (1, "angle"), (2, "offset"), (3, "absolute")];
const GEOMETRIC_FEATURE: &[(i16, &str)] = &[
    (1, "PointFromPointPointDistance"),
    (2, "PointFromPointPointPercentage"),
    (3, "PointFromPointNormalDistance"),
    (4, "LineFromPoints"),
    (5, "PlaneFromPoints"),
    (6, "PlaneFromPointLine"),
    (7, "CentroidFromPoints"),
    (8, "NormalFromPointPlane"),
];
const CONSTRAINT_TYPE: &[(i16, &str)] = &[(1, "Distance"), (2, "Angle"), (3, "ExclusionSphere")];
const SYMBOL_TYPE: &[(i16, &str)] = &[
    (0, "LonePair"),
    (1, "Electron"),
    (2, "RadicalCation"),
    (3, "RadicalAnion"),
    (4, "CirclePlus"),
    (5, "CircleMinus"),
    (6, "Dagger"),
    (7, "DoubleDagger"),
    (8, "Plus"),
    (9, "Minus"),
    (10, "Racemic"),
    (11, "Absolute"),
    (12, "Relative"),
];
const LINE_TYPE: &[(i16, &str)] = &[(0, "Solid"), (1, "Dashed"), (2, "Bold"), (4, "Wavy")];
const TABLE_SIDE: &[(i16, &str)] = &[(1, "top"), (2, "left"), (3, "bottom"), (4, "right")];
const RECTANGLE_TYPE: &[(i16, &str)] = &[
    (0, "Plain"),
    (1, "RoundEdge"),
    (2, "Shadow"),
    (4, "Shaded"),
    (8, "Filled"),
    (16, "Dashed"),
    (32, "Bold"),
];
const OVAL_TYPE: &[(i16, &str)] = &[
    (0, "Plain"),
    (1, "Circle"),
    (2, "Shaded"),
    (4, "Filled"),
    (8, "Dashed"),
    (16, "Bold"),
    (32, "Shadowed"),
];
const ARROW_TYPE: &[(i16, &str)] = &[
    (0, "NoHead"),
    (1, "HalfHead"),
    (2, "FullHead"),
    (4, "Resonance"),
    (8, "Equilibrium"),
    (16, "Hollow"),
    (32, "RetroSynthetic"),
    (64, "NoGo"),
    (128, "Dipole"),
];
const ARROW_HEAD_TYPE: &[(i16, &str)] = &[
    (0, "Unspecified"),
    (1, "Solid"),
    (2, "Hollow"),
    (3, "Angle"),
];
// Modern Arrow objects use a different enum from legacy Graphic/ArrowType.
// Values 2..4 are confirmed by ChemDraw's own CDXML -> CDX round trip.
const ARROW_HEAD_POSITION: &[(i16, &str)] = &[
    (0, "None"),
    (1, "Unspecified"),
    (2, "Full"),
    (3, "HalfLeft"),
    (4, "HalfRight"),
];
const NO_GO: &[(i16, &str)] = &[(0, "None"), (1, "None"), (2, "Cross"), (3, "Hash")];
const FILL_TYPE: &[(i16, &str)] = &[(0, "Unspecified"), (1, "None"), (2, "Solid"), (3, "Shaded")];
const ORBITAL_TYPE: &[(i16, &str)] = &[
    (0, "s"),
    (1, "oval"),
    (2, "lobe"),
    (3, "p"),
    (4, "hybridPlus"),
    (5, "hybridMinus"),
    (6, "dz2Plus"),
    (7, "dz2Minus"),
    (8, "dxy"),
    (256, "sFilled"),
    (257, "ovalFilled"),
    (258, "lobeFilled"),
    (259, "pFilled"),
    (260, "hybridPlusFilled"),
    (261, "hybridMinusFilled"),
    (262, "dz2PlusFilled"),
    (263, "dz2MinusFilled"),
    (264, "dxyFilled"),
    (512, "sShaded"),
    (513, "ovalShaded"),
    (514, "lobeShaded"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cdx_string_ffff_style_components_remain_inherited() {
        let text = CdxNode {
            name: "t".to_string(),
            tag: 0x8006,
            id: 2,
            attrs: BTreeMap::from([("id".to_string(), "2".to_string())]),
            properties: Vec::new(),
            text_runs: vec![CdxTextRun {
                start: 0,
                font: u16::MAX,
                face: u16::MAX,
                size: 20.0,
                color: u16::MAX,
            }],
            text: Some("BINAP".to_string()),
            children: Vec::new(),
        };
        let root = CdxNode {
            name: "CDXML".to_string(),
            tag: 0x8000,
            id: 1,
            attrs: BTreeMap::from([("id".to_string(), "1".to_string())]),
            properties: Vec::new(),
            text_runs: Vec::new(),
            text: None,
            children: vec![text],
        };
        let decoded = CdxmlWriter::new().write(&root);
        assert!(decoded.contains("<s size=\"20\">BINAP</s>"));
        assert!(!decoded.contains("65535"));
    }

    #[test]
    fn cdx_font_style_ffff_components_remain_inherited() {
        let mut cdx = CDX_HEADER.to_vec();
        write_u16(&mut cdx, 0x8000);
        write_u32(&mut cdx, 1);
        write_property(
            &mut cdx,
            0x080B,
            &[0xff, 0xff, 0xff, 0xff, 0xc8, 0x00, 0xff, 0xff],
        );
        write_u16(&mut cdx, 0);
        write_u16(&mut cdx, 0);

        let decoded = cdx_to_cdxml(&cdx).expect("font style should decode");
        assert!(decoded.contains("CaptionSize=\"10\""));
        assert!(!decoded.contains("CaptionFont="));
        assert!(!decoded.contains("CaptionFace="));
        assert!(!decoded.contains("CaptionColor="));
        assert!(!decoded.contains("65535"));
    }

    #[test]
    fn cdx_int16_properties_accept_legacy_single_byte_storage() {
        assert_eq!(
            decode_property(0x0402, &[7], None),
            Some(("Element", "7".to_string()))
        );
        assert_eq!(
            encode_property("Element", "7"),
            Some((0x0402, 7_i16.to_le_bytes().to_vec()))
        );
    }

    #[test]
    fn cdx_roundtrip_imports_basic_molecule() {
        let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE CDXML SYSTEM "http://www.cambridgesoft.com/xml/cdxml.dtd" >
<CDXML CreationProgram="ChemSema" BoundingBox="0 0 120 80" LabelFont="3" LabelSize="10" LabelFace="96" CaptionFont="3" CaptionSize="10" LineWidth="1" BoldWidth="4" BondLength="18" BondSpacing="18" HashSpacing="2.5" MarginWidth="2">
  <colortable>
    <color r="1" g="1" b="1"/>
    <color r="0" g="0" b="0"/>
  </colortable>
  <fonttable>
    <font id="3" charset="iso-8859-1" name="Arial"/>
  </fonttable>
  <page id="1" BoundingBox="0 0 120 80" Width="120" Height="80">
    <fragment id="2" BoundingBox="10 10 60 20">
      <n id="3" p="10 10" Element="6"/>
      <n id="4" p="60 20" Element="8" NumHydrogens="1"/>
      <b id="5" B="3" E="4" Order="1" Display="Dash"/>
    </fragment>
  </page>
</CDXML>
"#;
        let cdx = cdxml_to_cdx(cdxml).expect("CDXML should encode to CDX");
        let decoded = cdx_to_cdxml(&cdx).expect("CDX should decode to CDXML");
        assert!(decoded.contains("<fragment"));
        assert!(decoded.contains("Display=\"Dash\""));
        let doc = parse_cdx_document(&cdx, Some("basic")).expect("CDX should import");
        assert_eq!(doc.resources.len(), 1);
    }

    #[test]
    fn cdx_symbol_type_uses_official_enum_names_and_values() {
        let encoded = encode_property("SymbolType", "Plus").expect("plus symbol should encode");
        assert_eq!(encoded.0, 0x0A07);
        assert_eq!(encoded.1, 8_i16.to_le_bytes());
        let (_, decoded) =
            decode_property(encoded.0, &encoded.1, None).expect("plus symbol should decode");
        assert_eq!(decoded, "Plus");

        let cdxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LineWidth="0.6" BondLength="14.4">
  <page id="1">
    <graphic id="2" GraphicType="Symbol" SymbolType="Plus"
      BoundingBox="20 20 20 30"/>
  </page>
</CDXML>"#;
        let cdx = cdxml_to_cdx(cdxml).expect("symbol CDXML should encode");
        let decoded_cdxml = cdx_to_cdxml(&cdx).expect("symbol CDX should decode");
        assert!(
            decoded_cdxml.contains("SymbolType=\"Plus\""),
            "{decoded_cdxml}"
        );
        let document = parse_cdx_document(&cdx, Some("plus symbol"))
            .expect("symbol CDX should import into the document model");
        assert!(document.scene_objects().iter().any(|object| {
            object.object_type == "symbol"
                && object
                    .payload
                    .extra
                    .get("kind")
                    .and_then(serde_json::Value::as_str)
                    == Some("plus")
        }));
    }

    #[test]
    fn cdx_restrict_implicit_hydrogens_uses_official_implied_boolean_tag() {
        let encoded = encode_property("ImplicitHydrogens", "yes")
            .expect("implicit-hydrogen restriction should encode");
        assert_eq!(encoded.0, 0x0424);
        assert!(encoded.1.is_empty());
        let (_, decoded) = decode_property(encoded.0, &encoded.1, None)
            .expect("implicit-hydrogen restriction should decode");
        assert_eq!(decoded, "yes");

        let cdxml = r#"<?xml version="1.0" encoding="UTF-8"?>
<CDXML><page><fragment>
  <n id="1" p="10 10" Element="6" ImplicitHydrogens="yes">
    <t p="10 10" UTF8Text="C"/>
  </n>
  <n id="2" p="30 10" Element="6"/>
  <b id="3" B="1" E="2"/>
</fragment></page></CDXML>"#;
        let cdx = cdxml_to_cdx(cdxml).expect("query CDXML should encode to CDX");
        let decoded_cdxml = cdx_to_cdxml(&cdx).expect("query CDX should decode");
        assert!(
            decoded_cdxml.contains("ImplicitHydrogens=\"yes\""),
            "{decoded_cdxml}"
        );
        let document = parse_cdx_document(&cdx, Some("atom query"))
            .expect("query CDX should import into the document model");
        let node = document
            .resources
            .values()
            .find_map(|resource| resource.data.as_fragment())
            .and_then(|fragment| fragment.nodes.iter().find(|node| node.id == "1"))
            .expect("query node should survive");
        assert_eq!(
            node.meta
                .pointer("/import/cdxml/restrictImplicitHydrogens")
                .and_then(serde_json::Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn cdx_atom_query_enums_use_chemdraw_binary_values() {
        for (name, value, tag, byte) in [
            ("RingBondCount", "NoRingBonds", 0x0425, 0_u8),
            ("RingBondCount", "AsDrawn", 0x0425, 1),
            ("RingBondCount", "SimpleRing", 0x0425, 2),
            ("RingBondCount", "Fusion", 0x0425, 3),
            ("RingBondCount", "SpiroOrHigher", 0x0425, 4),
            ("UnsaturatedBonds", "MustBeAbsent", 0x0426, 1),
            ("UnsaturatedBonds", "MustBePresent", 0x0426, 2),
            ("Translation", "Broad", 0x0438, 1),
            ("Translation", "Narrow", 0x0438, 2),
            ("Translation", "Any", 0x0438, 3),
        ] {
            let encoded = encode_property(name, value).expect("query enum should encode");
            assert_eq!(encoded, (tag, vec![byte]), "{name}={value}");
            let (_, decoded) =
                decode_property(tag, &[byte], None).expect("query enum should decode");
            assert_eq!(decoded, value);
        }
    }

    #[test]
    fn cdx_crossing_bonds_use_official_object_id_array_tag_and_round_trip() {
        let encoded =
            encode_property("CrossingBonds", "20 21").expect("CrossingBonds should encode");
        assert_eq!(encoded.0, 0x060E);
        let (_, decoded) =
            decode_property(encoded.0, &encoded.1, None).expect("CrossingBonds should decode");
        assert_eq!(decoded, "20 21");

        let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="0 0 120 120" LineWidth="0.6" MarginWidth="1.6">
  <page id="1" BoundingBox="0 0 120 120">
    <fragment id="2">
      <n id="10" p="20 60"/><n id="11" p="100 60"/>
      <n id="12" p="60 20"/><n id="13" p="60 100"/>
      <b id="20" Z="7" B="10" E="11" CrossingBonds="21"/>
      <b id="21" Z="8" B="12" E="13" CrossingBonds="20"/>
    </fragment>
  </page>
</CDXML>"#;
        let cdx = cdxml_to_cdx(cdxml).expect("crossing CDXML should encode");
        let decoded_cdxml = cdx_to_cdxml(&cdx).expect("crossing CDX should decode");
        assert!(
            decoded_cdxml.contains("CrossingBonds=\"21\""),
            "{decoded_cdxml}"
        );
        assert!(
            decoded_cdxml.contains("CrossingBonds=\"20\""),
            "{decoded_cdxml}"
        );

        let imported = parse_cdx_document(&cdx, Some("crossings")).expect("CDX should import");
        let first = document_to_cdx(&imported).expect("crossing CDX should export");
        let reopened = parse_cdx_document(&first, Some("crossings"))
            .expect("exported crossing CDX should reopen");
        let second = document_to_cdx(&reopened).expect("crossing CDX should stabilize");
        assert_eq!(second, first);
    }

    #[test]
    fn cdx_text_preserves_style_runs_as_cdxml_s_elements() {
        let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="0 0 120 80">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1" BoundingBox="0 0 120 80">
    <t id="2" p="10 10" BoundingBox="10 10 60 25"><s font="3" size="12" face="1" color="0">Hello</s><s font="3" size="8" face="0" color="0">2</s></t>
  </page>
</CDXML>
"#;
        let cdx = cdxml_to_cdx(cdxml).expect("CDXML should encode to CDX");
        let decoded = cdx_to_cdxml(&cdx).expect("CDX should decode to CDXML");
        assert!(decoded.contains("<s font=\"3\" size=\"12\" face=\"1\" color=\"0\">Hello</s>"));
        assert!(decoded.contains("<s font=\"3\" size=\"8\" face=\"0\" color=\"0\">2</s>"));
    }

    #[test]
    fn cdx_justification_enums_use_signed_single_byte_values() {
        for (name, encoded) in [
            ("Right", 0xff),
            ("Left", 0x00),
            ("Center", 0x01),
            ("Full", 0x02),
            ("Above", 0x03),
            ("Below", 0x04),
            ("Auto", 0x05),
            ("Best", 0x06),
        ] {
            let (tag, bytes) =
                encode_property("LabelJustification", name).expect("justification should encode");
            assert_eq!(tag, 0x0823);
            assert_eq!(bytes, vec![encoded], "{name}");
            let (_, decoded) =
                decode_property(tag, &bytes, None).expect("justification should decode");
            assert_eq!(decoded, name);
        }
    }

    #[test]
    fn cdx_label_display_and_alignment_use_single_byte_values() {
        for (property, expected_tag) in [("LabelDisplay", 0x0401), ("LabelAlignment", 0x0705)] {
            let (tag, bytes) = encode_property(property, "Right").expect("enum should encode");
            assert_eq!(tag, expected_tag);
            assert_eq!(bytes, vec![3]);
            let (_, decoded) = decode_property(tag, &bytes, None).expect("enum should decode");
            assert_eq!(decoded, "Right");
        }
    }

    #[test]
    fn cdx_open_save_stabilizes_label_layout_fields() {
        let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="0 0 80 40" BondLength="14.4" LabelFont="3" LabelSize="10">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1" BoundingBox="0 0 80 40">
    <fragment id="2" BoundingBox="0 0 80 40">
      <n id="3" p="30 20" NodeType="Nickname">
        <t id="4" p="30 24" BoundingBox="0 10 30 26" LabelJustification="Right" Justification="Right" LabelAlignment="Right" UTF8Text="C10H21">
          <s font="3" size="10" face="96" color="0">C10H21</s>
        </t>
      </n>
      <n id="5" p="48 20"/>
      <b id="6" B="3" E="5"/>
    </fragment>
  </page>
</CDXML>"#;
        let source = cdxml_to_cdx(cdxml).expect("source CDX should encode");
        let imported = parse_cdx_document(&source, Some("stable CDX")).expect("source CDX import");
        let first = document_to_cdx(&imported).expect("first CDX export");
        let reopened = parse_cdx_document(&first, Some("stable CDX")).expect("reopen CDX");
        let second = document_to_cdx(&reopened).expect("second CDX export");

        assert_eq!(
            second, first,
            "CDX must stabilize after the first ChemSema save"
        );
        let decoded = cdx_to_cdxml(&first).expect("saved CDX should decode");
        for expected in [
            "LabelJustification=\"Right\"",
            "Justification=\"Right\"",
            "LabelAlignment=\"Right\"",
        ] {
            assert!(decoded.contains(expected), "missing {expected}: {decoded}");
        }
    }

    #[test]
    fn cdx_right_aligned_chemical_label_stays_reversed_across_open_save() {
        let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="0 0 80 40" BondLength="14.4" LabelFont="3" LabelSize="10">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1" BoundingBox="0 0 80 40">
    <fragment id="2" BoundingBox="0 0 80 40">
      <n id="3" p="30 20" NodeType="Nickname">
        <t id="4" p="30 24" BoundingBox="0 10 30 26" LabelJustification="Right" Justification="Right" LabelAlignment="Right" UTF8Text="OCF3">
          <s font="3" size="10" face="96" color="0">OCF3</s>
        </t>
      </n>
      <n id="5" p="48 20"/>
      <b id="6" B="3" E="5"/>
    </fragment>
  </page>
</CDXML>"#;
        let source = cdxml_to_cdx(cdxml).expect("source CDX should encode");
        let imported =
            parse_cdx_document(&source, Some("right-aligned OCF3")).expect("source CDX import");
        let imported_label = imported
            .resources
            .values()
            .find_map(|resource| resource.data.as_fragment())
            .and_then(|fragment| fragment.nodes.iter().find_map(|node| node.label.as_ref()))
            .expect("imported OCF3 label");
        assert_eq!(imported_label.source_text.as_deref(), Some("OCF3"));
        assert_eq!(imported_label.text, "F3CO");

        let first = document_to_cdx(&imported).expect("first CDX export");
        let reopened = parse_cdx_document(&first, Some("right-aligned OCF3")).expect("reopen CDX");
        let reopened_label = reopened
            .resources
            .values()
            .find_map(|resource| resource.data.as_fragment())
            .and_then(|fragment| fragment.nodes.iter().find_map(|node| node.label.as_ref()))
            .expect("reopened OCF3 label");
        assert_eq!(reopened_label.source_text.as_deref(), Some("OCF3"));
        assert_eq!(reopened_label.text, "F3CO");

        let second = document_to_cdx(&reopened).expect("second CDX export");
        assert_eq!(second, first, "right-aligned OCF3 CDX must stabilize");
        let decoded = cdx_to_cdxml(&first).expect("saved CDX should decode");
        assert!(decoded.contains("LabelJustification=\"Right\""));
        assert!(decoded.contains("Justification=\"Right\""));
        assert!(decoded.contains("LabelAlignment=\"Right\""));
        assert!(
            !decoded.contains("LabelDisplay=\"Right\""),
            "alignment fields must not be promoted to a fixed LabelDisplay: {decoded}"
        );
    }

    #[test]
    fn cdx_right_aligned_hyphenated_label_token_stays_whole() {
        let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="0 0 80 40" BondLength="14.4" LabelFont="3" LabelSize="10">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1" BoundingBox="0 0 80 40">
    <fragment id="2" BoundingBox="0 0 80 40">
      <n id="3" p="30 20" NodeType="Nickname">
        <t id="4" p="30 24" BoundingBox="0 10 30 26" LabelJustification="Right" Justification="Right" LabelAlignment="Right" UTF8Text="2-Np">
          <s font="3" size="10" face="0" color="0">2-Np</s>
        </t>
      </n>
      <n id="5" p="48 20"/>
      <b id="6" B="3" E="5"/>
    </fragment>
  </page>
</CDXML>"#;
        let source = cdxml_to_cdx(cdxml).expect("source CDX should encode");
        let imported =
            parse_cdx_document(&source, Some("right-aligned 2-Np")).expect("source CDX import");
        let imported_label = imported
            .resources
            .values()
            .find_map(|resource| resource.data.as_fragment())
            .and_then(|fragment| fragment.nodes.iter().find_map(|node| node.label.as_ref()))
            .expect("imported 2-Np label");
        assert_eq!(imported_label.source_text.as_deref(), Some("2-Np"));
        assert_eq!(imported_label.text, "2-Np");

        let first = document_to_cdx(&imported).expect("first CDX export");
        let reopened = parse_cdx_document(&first, Some("right-aligned 2-Np")).expect("reopen CDX");
        let reopened_label = reopened
            .resources
            .values()
            .find_map(|resource| resource.data.as_fragment())
            .and_then(|fragment| fragment.nodes.iter().find_map(|node| node.label.as_ref()))
            .expect("reopened 2-Np label");
        assert_eq!(reopened_label.source_text.as_deref(), Some("2-Np"));
        assert_eq!(reopened_label.text, "2-Np");

        let second = document_to_cdx(&reopened).expect("second CDX export");
        assert_eq!(second, first, "right-aligned 2-Np CDX must stabilize");
    }

    #[test]
    fn cdx_inferred_centered_metal_label_keeps_vertical_anchor_and_stable_fields() {
        let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="0 0 80 40" BondLength="14.4" LabelFont="3" LabelSize="10" MarginWidth="1.6">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1" BoundingBox="0 0 80 40">
    <fragment id="2" BoundingBox="0 0 80 40">
      <n id="3" p="30 20" Element="46" NumHydrogens="0">
        <t id="4" p="30 23.9" BoundingBox="23.9 14.9 36.1 26.4" LabelJustification="Center" Justification="Center" LabelAlignment="Center" UTF8Text="Pd">
          <s font="3" size="10" face="96" color="0">Pd</s>
        </t>
      </n>
      <n id="5" p="44.4 20" Element="7" NumHydrogens="0">
        <t id="6" p="40.8 23.9" BoundingBox="40.8 15.7 48 24.6" LabelJustification="Left" UTF8Text="N">
          <s font="3" size="10" face="96" color="0">N</s>
        </t>
      </n>
      <b id="7" B="3" E="5"/>
    </fragment>
  </page>
</CDXML>"#;
        let source = cdxml_to_cdx(cdxml).expect("source CDX should encode");
        let imported =
            parse_cdx_document(&source, Some("inferred centered Pd")).expect("source CDX import");
        let imported_fragment = imported
            .resources
            .values()
            .find_map(|resource| resource.data.as_fragment())
            .expect("imported fragment");
        let imported_metal = imported_fragment
            .nodes
            .iter()
            .find(|node| node.element == "Pd")
            .expect("imported Pd node");
        let imported_label = imported_metal.label.as_ref().expect("imported Pd label");
        assert_eq!(imported_label.align.as_deref(), Some("center"));
        assert_eq!(imported_label.anchor.as_deref(), Some("middle"));
        assert_eq!(
            imported_label.layout.as_deref(),
            Some("attached-group-center")
        );
        assert_eq!(
            imported_label.meta.pointer("/import/cdxml/labelDisplay"),
            Some(&serde_json::Value::Null)
        );
        assert!(
            (imported_label.position.expect("Pd baseline")[1]
                - imported_metal.position[1]
                - 3.9)
                .abs()
                < 0.01,
            "inferred centered Pd baseline must use the ChemDraw 0.39 font-size anchor: node={imported_metal:?}"
        );

        let first = document_to_cdx(&imported).expect("first CDX export");
        let decoded = cdx_to_cdxml(&first).expect("saved CDX should decode");
        for expected in [
            "LabelJustification=\"Center\"",
            "Justification=\"Center\"",
            "LabelAlignment=\"Center\"",
        ] {
            assert!(decoded.contains(expected), "missing {expected}: {decoded}");
        }
        assert!(
            !decoded.contains("LabelDisplay=\"Center\""),
            "inferred center fields must not be promoted to LabelDisplay: {decoded}"
        );

        let reopened =
            parse_cdx_document(&first, Some("inferred centered Pd")).expect("reopen CDX");
        let reopened_fragment = reopened
            .resources
            .values()
            .find_map(|resource| resource.data.as_fragment())
            .expect("reopened fragment");
        let reopened_metal = reopened_fragment
            .nodes
            .iter()
            .find(|node| node.element == "Pd")
            .expect("reopened Pd node");
        let reopened_label = reopened_metal.label.as_ref().expect("reopened Pd label");
        assert!(
            (reopened_label.position.expect("reopened Pd baseline")[1]
                - reopened_metal.position[1]
                - 3.9)
                .abs()
                < 0.01
        );
        let second = document_to_cdx(&reopened).expect("second CDX export");
        assert_eq!(second, first, "inferred centered Pd CDX must stabilize");
    }

    #[test]
    fn cdx_internal_fragment_attachment_round_trips_stably() {
        let cdxml = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="0 0 80 40" BondLength="14.4" LabelFont="3" LabelSize="10" MarginWidth="1.6">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1" BoundingBox="0 0 80 40">
    <fragment id="2" BoundingBox="0 0 80 40">
      <n id="3" p="10 30" NodeType="Fragment">
        <t id="4" p="8 34" BoundingBox="8 22 62 36" LabelAlignment="Left" LabelJustification="Left" InterpretChemically="yes">
          <s font="3" size="10" face="0" color="0">(PhO)</s>
          <s font="3" size="10" face="96" color="0">2</s>
          <s font="3" size="10" face="0" color="0">POH</s>
        </t>
      </n>
      <n id="5" p="42 16" Element="8" NumHydrogens="0">
        <t id="6" p="38 20" BoundingBox="38 10 46 21"><s font="3" size="10" face="96" color="0">O</s></t>
      </n>
      <b id="7" B="3" BeginAttach="6" E="5" Order="2"/>
    </fragment>
  </page>
</CDXML>"#;
        let source = cdxml_to_cdx(cdxml).expect("source CDX should encode");
        let source_decoded = cdx_to_cdxml(&source).expect("source CDX should decode");
        assert!(
            source_decoded.contains("BeginAttach=\"6\""),
            "{source_decoded}"
        );
        let imported = parse_cdx_document(&source, Some("internal attachment"))
            .expect("source CDX should import");
        let imported_bond = imported
            .resources
            .values()
            .find_map(|resource| resource.data.as_fragment())
            .and_then(|fragment| fragment.bonds.first())
            .expect("imported bond");
        assert_eq!(
            imported_bond
                .meta
                .pointer("/endpointAttachments/begin/characterIndex")
                .and_then(serde_json::Value::as_u64),
            Some(6)
        );
        assert_eq!(
            imported_bond
                .meta
                .pointer("/endpointAttachments/begin/character"),
            Some(&serde_json::json!("P"))
        );

        let first = document_to_cdx(&imported).expect("first CDX export");
        let decoded = cdx_to_cdxml(&first).expect("saved CDX should decode");
        assert!(decoded.contains("BeginAttach=\"6\""), "{decoded}");
        let reopened = parse_cdx_document(&first, Some("internal attachment")).expect("reopen CDX");
        let second = document_to_cdx(&reopened).expect("second CDX export");
        assert_eq!(second, first, "internal attachment CDX must stabilize");
    }

    #[test]
    fn cdx_chain_angle_uses_readable_degrees_and_fixed_point_binary() {
        let encoded = encode_property("ChainAngle", "120").expect("chain angle should encode");
        assert_eq!(encoded.1, (120_i32 * 65536).to_le_bytes());
        let (_, decoded) =
            decode_property(encoded.0, &encoded.1, None).expect("chain angle should decode");
        assert_eq!(decoded, "120");
    }

    #[test]
    fn cdx_geometry_and_constraint_properties_use_the_official_tags_and_types() {
        for (name, value, tag, bytes) in [
            (
                "GeometricFeature",
                "PointFromPointNormalDistance",
                0x0B80,
                vec![3],
            ),
            (
                "RelationValue",
                "12.5",
                0x0B81,
                12.5_f64.to_le_bytes().to_vec(),
            ),
            (
                "BasisObjects",
                "17 23",
                0x0B82,
                [17_u32.to_le_bytes(), 23_u32.to_le_bytes()].concat(),
            ),
            ("ConstraintType", "Angle", 0x0B83, vec![2]),
        ] {
            let encoded = encode_property(name, value).expect("property should encode");
            assert_eq!(encoded, (tag, bytes.clone()), "{name}={value}");
            assert_eq!(
                decode_property(tag, &bytes, None),
                Some((name, value.to_string())),
                "{name}={value}"
            );
        }
        assert_eq!(
            encode_property("GeometricFeature", "3"),
            Some((0x0B80, vec![3]))
        );
        assert_eq!(
            encode_property("ConstraintType", "2"),
            Some((0x0B83, vec![2]))
        );

        assert_eq!(property_tag("ExternalConnectionID"), None);
        assert_eq!(property_tag("BracketedObjects"), None);
        assert_eq!(property_tag("RepeatPattern"), None);
    }

    #[test]
    fn cdx_arrow_properties_follow_chemdraws_binary_enums_and_units() {
        for (name, value, tag, bytes) in [
            ("ArrowheadHead", "None", 0x0A35, vec![0, 0]),
            ("ArrowheadHead", "Full", 0x0A35, vec![2, 0]),
            ("ArrowheadHead", "HalfLeft", 0x0A35, vec![3, 0]),
            ("ArrowheadHead", "HalfRight", 0x0A35, vec![4, 0]),
            ("ArrowheadTail", "Full", 0x0A36, vec![2, 0]),
            ("NoGo", "Cross", 0x0A3B, vec![2]),
            ("NoGo", "Hash", 0x0A3B, vec![3]),
            ("AngularSize", "90", 0x0A21, vec![0x84, 0x03]),
            ("CurveSpacing", "777", 0x0A38, vec![0x09, 0x03]),
        ] {
            let encoded = encode_property(name, value).expect("arrow property should encode");
            assert_eq!(encoded, (tag, bytes.clone()), "{name}={value}");
            assert_eq!(
                decode_property(tag, &bytes, None),
                Some((name, value.to_string())),
                "{name}={value}"
            );
        }

        assert_eq!(encode_property("Dipole", "yes"), Some((0x0A3A, Vec::new())));
        assert_eq!(
            decode_property(0x0A3A, &[], None),
            Some(("Dipole", "yes".to_string()))
        );
    }

    #[test]
    fn cdx_external_connection_types_cover_the_complete_chemdraw_enum() {
        for (value, name) in EXTERNAL_CONNECTION_TYPE {
            let encoded =
                encode_property("ExternalConnectionType", name).expect("type should encode");
            assert_eq!(encoded, (0x0440, vec![*value as u8]), "{name}");
            assert_eq!(
                decode_property(0x0440, &encoded.1, None),
                Some(("ExternalConnectionType", (*name).to_string())),
                "{name}"
            );
        }
    }

    #[test]
    fn cdx_legacy_arrow_type_preserves_base_type_and_modifiers() {
        let encoded = encode_property("ArrowType", "FullHead NoGo Dipole")
            .expect("legacy arrow flags should encode");
        assert_eq!(encoded, (0x0A02, vec![0xC2, 0x00]));
        assert_eq!(
            decode_property(encoded.0, &encoded.1, None),
            Some(("ArrowType", "FullHead NoGo Dipole".to_string()))
        );
        assert_eq!(
            decode_property(0x0A02, &[2], None),
            Some(("ArrowType", "FullHead".to_string())),
            "ChemDraw 8 wrote this INT16 property as a single byte"
        );
    }

    #[test]
    fn cdx_text_layout_properties_follow_the_official_tags_and_special_values() {
        for (name, value, tag, bytes, decoded) in [
            ("LineHeight", "variable", 0x0702, vec![0, 0], "variable"),
            ("LineHeight", "auto", 0x0702, vec![1, 0], "auto"),
            ("WordWrapWidth", "144", 0x0703, vec![144, 0], "144"),
            ("LabelLineHeight", "12", 0x0706, vec![240, 0], "12"),
            ("CaptionLineHeight", "8.25", 0x0707, vec![165, 0], "8.25"),
            ("CaptionLineHeight", "auto", 0x0707, vec![1, 0], "auto"),
            ("BondSpacing", "12.5", 0x0804, vec![125, 0], "12.5"),
            ("BondSpacingAbs", "1.25", 0x0822, vec![0, 64, 1, 0], "1.25"),
            ("Side", "left", 0x0825, vec![2, 0], "left"),
            ("BracketType", "Square", 0x0A06, vec![3, 0], "Square"),
            ("BracketType", "Round", 0x0A06, vec![5, 0], "Round"),
            (
                "OvalType",
                "Circle Shaded",
                0x0A04,
                vec![3, 0],
                "Circle Shaded",
            ),
            (
                "RectangleType",
                "RoundEdge Shadow",
                0x0A03,
                vec![3, 0],
                "RoundEdge Shadow",
            ),
            ("LineType", "Bold Dashed", 0x0A01, vec![3, 0], "Dashed Bold"),
            ("LipSize", "60", 0x0A22, vec![60, 0], "60"),
            ("PositioningType", "absolute", 0x0D06, vec![3], "absolute"),
            (
                "NodeType",
                "GenericNickname",
                0x0400,
                vec![7, 0],
                "GenericNickname",
            ),
        ] {
            let encoded = encode_property(name, value).expect("property should encode");
            assert_eq!(encoded.0, tag, "{name}");
            assert_eq!(encoded.1, bytes, "{name}");
            let (decoded_name, decoded_value) =
                decode_property(tag, &encoded.1, None).expect("property should decode");
            assert_eq!(decoded_name, name);
            assert_eq!(decoded_value, decoded);
        }

        let best = encode_property("LabelAlignment", "Best").expect("Best should encode");
        assert_eq!(best, (0x0705, vec![6]));
        assert_eq!(
            decode_property(best.0, &best.1, None),
            Some(("LabelAlignment", "Best".to_string()))
        );
    }

    #[test]
    fn chemdraw_8_one_byte_bracket_type_uses_the_documented_enum() {
        assert_eq!(
            decode_property(0x0A06, &[3], None),
            Some(("BracketType", "Square".to_string()))
        );
    }

    #[test]
    fn cdx_superseded_by_reads_the_legacy_alias_and_writes_the_official_tag() {
        let encoded = encode_property("SupersededBy", "203").expect("property should encode");
        assert_eq!(encoded, (0x0012, 203_u32.to_le_bytes().to_vec()));

        for tag in [0x0012, 0x0013] {
            assert_eq!(
                decode_property(tag, &203_u32.to_le_bytes(), None),
                Some(("SupersededBy", "203".to_string()))
            );
        }
    }

    #[test]
    fn cdx_text_uses_utf8_when_valid_and_windows_1252_for_legacy_bytes() {
        assert_eq!(decode_text("11 °F".as_bytes(), None, None), "11 °F");
        assert_eq!(decode_text(b"11 \xB0F", None, None), "11 °F");
    }

    #[test]
    fn chemdraw_object_tags_override_the_shifted_static_registry() {
        assert_eq!(object_tag("geometry"), Some(0x801B));
        assert_eq!(object_tag("constraint"), Some(0x801C));
        assert_eq!(object_tag("tlcplate"), Some(0x801D));
        assert_eq!(object_tag("tlclane"), Some(0x801E));
        assert_eq!(object_tag("tlcspot"), Some(0x801F));
        assert_eq!(object_tag("chemicalproperty"), Some(0x8020));
        assert_eq!(object_tag("arrow"), Some(0x8021));
        assert_eq!(object_tag("border"), Some(0x801A));
        assert_eq!(legacy_chemsema_object_name(0x801A), Some("border"));
        assert_eq!(legacy_chemsema_object_name(0x8021), Some("arrow"));

        let chemdraw = r#"<CDXML CreationProgram="ChemSema" ModificationProgram="ChemSema/1.0.0-beta.1;cdx-tags=chemdraw"><geometry id="2" /></CDXML>"#;
        let chemdraw_tree = CdxReader::new(&cdxml_to_cdx(chemdraw).unwrap())
            .read()
            .unwrap();
        assert_eq!(chemdraw_tree.children[0].name, "geometry");

        let mut shifted_beta = CDX_HEADER.to_vec();
        write_u16(&mut shifted_beta, 0x8000);
        write_u32(&mut shifted_beta, 1);
        write_property(
            &mut shifted_beta,
            0x0006,
            &encode_plain_cdx_string("ChemSema/1.0.0-beta.1;cdx-tags=official"),
        );
        write_u16(&mut shifted_beta, 0x8027);
        write_u32(&mut shifted_beta, 2);
        write_u16(&mut shifted_beta, 0);
        write_u16(&mut shifted_beta, 0);
        write_u16(&mut shifted_beta, 0);
        assert_eq!(
            CdxReader::new(&shifted_beta).read().unwrap().children[0].name,
            "arrow"
        );
    }

    #[test]
    fn public_complex_cdx_types_follow_the_official_binary_layouts() {
        for (kind, lexical) in [
            ("CDXDate", "2026 7 23 9 30 45 125"),
            ("CDXElementList", "9 17 35"),
            ("CDXElementList", "NOT 9 17 35"),
            ("CDXObjectIDArrayWithCounts", "1 2 3 4"),
            ("CDXCurvePoints", "1 2 3 4"),
            ("CDXCurvePoints3D", "1 2 3 4 5 6"),
            ("CDXRepresentsProperty", "6 0x0421"),
            ("CDXGenericList", "R X A"),
            ("CDXGenericList", "NOT R X A"),
        ] {
            let bytes = encode_official_lexical(kind, lexical).expect("complex type encodes");
            assert_eq!(
                decode_official_lexical(kind, &bytes).as_deref(),
                Some(lexical),
                "{kind}"
            );
        }
        assert_eq!(
            encode_official_lexical("CDXElementList", "NOT 9 17 35").unwrap(),
            vec![0xFD, 0xFF, 9, 0, 17, 0, 35, 0]
        );
        assert_eq!(
            encode_official_lexical("CDXObjectIDArrayWithCounts", "1 2 3 4").unwrap()[..2],
            [4, 0]
        );
        assert_eq!(
            encode_official_lexical("CDXGenericList", "R X A").unwrap(),
            vec![3, 0, 3, 0, 0, 0, b'R', 3, 0, 0, 0, b'X', 3, 0, 0, 0, b'A']
        );
    }

    #[test]
    fn repeated_cdx_properties_keep_order_values_and_distinct_json_keys() {
        let mut cdx = CDX_HEADER.to_vec();
        write_u16(&mut cdx, 0x8000);
        write_u32(&mut cdx, 1);
        write_property(&mut cdx, 0x0A86, &1.25_f64.to_le_bytes());
        write_property(&mut cdx, 0x0A86, &2.5_f64.to_le_bytes());
        write_u16(&mut cdx, 0);
        write_u16(&mut cdx, 0);

        let document = parse_cdx_document(&cdx, Some("repeated")).unwrap();
        let properties = &document.interchange["cdx"].root.properties;
        assert_eq!(properties["Spectrum_DataPoint"].value, "1.25");
        assert_eq!(properties["Spectrum_DataPoint#2"].value, "2.5");
        assert_eq!(
            properties["Spectrum_DataPoint#2"].name,
            "Spectrum_DataPoint"
        );

        let saved = document_to_cdx(&document).unwrap();
        let reopened = parse_cdx_document(&saved, Some("repeated")).unwrap();
        let properties = &reopened.interchange["cdx"].root.properties;
        assert_eq!(properties["Spectrum_DataPoint"].value, "1.25");
        assert_eq!(properties["Spectrum_DataPoint#2"].value, "2.5");
    }

    #[test]
    fn unknown_cdx_objects_keep_their_position_tag_and_payload() {
        let mut cdx = CDX_HEADER.to_vec();
        write_u16(&mut cdx, 0x8000);
        write_u32(&mut cdx, 1);
        for (tag, id) in [(0x8002, 2), (0xC001, 3), (0x8001, 4)] {
            write_u16(&mut cdx, tag);
            write_u32(&mut cdx, id);
            if tag == 0xC001 {
                write_property(&mut cdx, 0x4001, &[1, 2, 3, 4]);
            }
            write_u16(&mut cdx, 0);
        }
        write_u16(&mut cdx, 0);
        write_u16(&mut cdx, 0);

        let document = parse_cdx_document(&cdx, Some("unknown object")).unwrap();
        let saved = document_to_cdx(&document).unwrap();
        let reopened = parse_cdx_document(&saved, Some("unknown object")).unwrap();
        let children: Vec<_> = reopened.interchange["cdx"]
            .root
            .children
            .iter()
            .filter(|child| {
                child
                    .format_tag
                    .as_deref()
                    .and_then(parse_hex_u16)
                    .is_some_and(is_object_tag)
            })
            .collect();
        assert_eq!(
            children
                .iter()
                .map(|child| child.name.as_str())
                .collect::<Vec<_>>(),
            vec!["group", "unknown", "page"]
        );
        assert_eq!(children[1].format_tag.as_deref(), Some("0xC001"));
        assert_eq!(
            children[1].properties["tag_4001"].raw_base64.as_deref(),
            Some("AQIDBA==")
        );
    }

    #[test]
    fn cdx_official_properties_and_objects_are_editable_and_lossless_in_ccjs() {
        let mut cdx = CDX_HEADER.to_vec();
        write_u16(&mut cdx, 0x8000);
        write_u32(&mut cdx, 1);
        write_property(&mut cdx, 0x000B, b"CAS-1");
        write_u16(&mut cdx, 0x800C);
        write_u32(&mut cdx, 2);
        write_property(&mut cdx, 0x000C, b"CAS");
        write_u16(&mut cdx, 0);
        write_u16(&mut cdx, 0);
        write_u16(&mut cdx, 0);

        let mut document = parse_cdx_document(&cdx, Some("registry")).expect("CDX parses");
        let source = document.interchange.get("cdx").expect("CDX tree");
        assert_eq!(source.root.properties["RegistryNumber"].value, "CAS-1");
        assert_eq!(source.root.children[0].name, "regnum");
        assert_eq!(
            source.root.children[0].properties["RegistryAuthority"].value,
            "CAS"
        );
        document
            .set_interchange_property("cdx", &[], "RegistryNumber", "CAS-2")
            .expect("public interchange edit API updates a CDX field");

        let saved = document_to_cdx(&document).expect("edited CDX saves");
        let reopened = CdxReader::new(&saved).read().expect("saved CDX reopens");
        assert_eq!(
            reopened.attrs.get("RegistryNumber").map(String::as_str),
            Some("CAS-2")
        );
        assert!(reopened.children.iter().any(|child| child.name == "regnum"));
    }
}
