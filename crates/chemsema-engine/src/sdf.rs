use crate::{
    Bond, BondStereo, ChemSemaDocument, DocumentInfo, DocumentStyleInfo, DoubleBond,
    DoubleBondPlacement, FormatInfo, MoleculeFragment, Node, ObjectPayload, Page, Resource,
    ResourceData, SceneObject, Transform,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write;

const DEFAULT_SDF_BOND_LENGTH_PT: f64 = 54.0;
const SDF_RECORD_SEPARATOR: &str = "$$$$";

#[derive(Debug, Clone)]
struct SdfRecord {
    molblock: String,
    data: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone)]
struct MolAtom {
    x: f64,
    y: f64,
    symbol: String,
    charge: i32,
}

#[derive(Debug, Clone)]
struct MolBond {
    begin: usize,
    end: usize,
    order: u8,
    stereo: u8,
}

#[derive(Debug, Clone)]
struct Molfile {
    title: String,
    atoms: Vec<MolAtom>,
    bonds: Vec<MolBond>,
}

pub fn parse_sdf_document(sdf: &str, title: Option<&str>) -> Result<ChemSemaDocument, String> {
    let records = parse_sdf_records(sdf)?;
    if records.is_empty() {
        return Err("SDF contains no molecule records.".to_string());
    }

    let mut resources = BTreeMap::new();
    let mut objects = Vec::new();
    let mut cursor_x = 36.0;
    let mut cursor_y = 36.0;
    let mut row_height: f64 = 0.0;
    let max_row_width = 720.0;

    for (index, record) in records.iter().enumerate() {
        let molfile = parse_v2000_molfile(&record.molblock)
            .map_err(|error| format!("Record {}: {error}", index + 1))?;
        let (fragment, width, height) = molfile_to_fragment(&molfile, record);
        let resource_id = format!("sdf_mol_{:03}", index + 1);
        if cursor_x > 36.0 && cursor_x + width > max_row_width {
            cursor_x = 36.0;
            cursor_y += row_height + 36.0;
            row_height = 0.0;
        }
        let object_id = format!("obj_sdf_mol_{:03}", index + 1);
        resources.insert(
            resource_id.clone(),
            Resource {
                resource_type: "molecule_fragment2d".to_string(),
                encoding: "chemsema.molecule.fragment2d".to_string(),
                data: ResourceData::Fragment(fragment),
                meta: json!({
                    "import": {
                        "sdf": {
                            "recordIndex": index + 1,
                            "title": empty_string_as_null(&molfile.title),
                            "data": record.data,
                        }
                    }
                }),
            },
        );
        objects.push(SceneObject {
            id: object_id,
            object_type: "molecule".to_string(),
            name: if molfile.title.trim().is_empty() {
                format!("molecule {}", index + 1)
            } else {
                molfile.title.trim().to_string()
            },
            visible: true,
            locked: false,
            z_index: 10 + index as i32,
            transform: Transform {
                translate: [round2(cursor_x), round2(cursor_y)],
                rotate: 0.0,
                scale: [1.0, 1.0],
            },
            style_ref: Some("style_molecule_default".to_string()),
            link_policy: Default::default(),
            meta: json!({
                "source": "sdf",
                "recordIndex": index + 1,
            }),
            payload: ObjectPayload {
                resource_ref: Some(resource_id),
                bbox: Some([0.0, 0.0, round2(width), round2(height)]),
                spectrum: None,
                extra: BTreeMap::new(),
            },
            children: Vec::new(),
        });
        cursor_x += width + 36.0;
        row_height = row_height.max(height);
    }

    let page_width = (max_row_width + 36.0).max(cursor_x + 36.0);
    let page_height = (cursor_y + row_height + 36.0).max(360.0);
    Ok(ChemSemaDocument {
        format: FormatInfo {
            name: "chemsema".to_string(),
            version: "0.1".to_string(),
            unit: "pt".to_string(),
        },
        document: DocumentInfo {
            id: "doc_sdf_import".to_string(),
            title: title
                .filter(|value| !value.trim().is_empty())
                .unwrap_or("Imported SDF")
                .to_string(),
            page: Page {
                width: round2(page_width),
                height: round2(page_height),
                background: "#ffffff".to_string(),
            },
            meta: json!({
                "import": {
                    "sdf": {
                        "recordCount": records.len(),
                    }
                }
            }),
        },
        style: DocumentStyleInfo::default(),
        styles: default_sdf_styles(),
        objects,
        links: Vec::new(),
        resources,
        interchange: BTreeMap::new(),
    })
}

pub fn document_to_sdf(document: &ChemSemaDocument) -> Result<String, String> {
    let mut records = Vec::new();
    for object in flatten_visible_objects(&document.objects) {
        if object.object_type != "molecule" {
            continue;
        }
        let Some(resource_ref) = object.payload.resource_ref.as_deref() else {
            continue;
        };
        let Some(resource) = document.resources.get(resource_ref) else {
            continue;
        };
        let ResourceData::Fragment(fragment) = &resource.data else {
            continue;
        };
        if fragment.nodes.is_empty() {
            continue;
        }
        records.push(fragment_to_sdf_record(
            document, object, resource, fragment,
        )?);
    }

    if records.is_empty() {
        return Err("No molecule objects are available for SDF export.".to_string());
    }

    let mut out = String::new();
    for record in records {
        out.push_str(&record);
        if !record.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(SDF_RECORD_SEPARATOR);
        out.push('\n');
    }
    Ok(out)
}

fn parse_sdf_records(sdf: &str) -> Result<Vec<SdfRecord>, String> {
    let normalized = sdf.replace("\r\n", "\n").replace('\r', "\n");
    let mut records = Vec::new();
    for raw_record in normalized.split(SDF_RECORD_SEPARATOR) {
        let trimmed = raw_record.trim_matches('\n');
        if trimmed.trim().is_empty() {
            continue;
        }
        records.push(parse_sdf_record(trimmed)?);
    }
    Ok(records)
}

fn parse_sdf_record(record: &str) -> Result<SdfRecord, String> {
    let end = record
        .find("\nM  END")
        .map(|index| index + "\nM  END".len())
        .or_else(|| {
            record
                .trim_end()
                .ends_with("M  END")
                .then_some(record.len())
        })
        .ok_or_else(|| "SDF record is missing M  END.".to_string())?;
    let molblock = record[..end].trim_matches('\n').to_string();
    let data_block = record[end..].trim_matches('\n');
    Ok(SdfRecord {
        molblock,
        data: parse_sdf_data_fields(data_block),
    })
}

fn parse_sdf_data_fields(data_block: &str) -> BTreeMap<String, Vec<String>> {
    let mut fields = BTreeMap::new();
    let mut current_name: Option<String> = None;
    let mut current_lines = Vec::new();

    for line in data_block.lines() {
        if line.starts_with(">") {
            if let Some(name) = current_name.take() {
                fields.insert(name, trim_trailing_empty_lines(current_lines));
                current_lines = Vec::new();
            }
            current_name = sdf_field_name(line);
            continue;
        }
        if current_name.is_some() {
            current_lines.push(line.to_string());
        }
    }

    if let Some(name) = current_name {
        fields.insert(name, trim_trailing_empty_lines(current_lines));
    }
    fields
}

fn sdf_field_name(line: &str) -> Option<String> {
    let start = line.find('<')?;
    let end = line[start + 1..].find('>')? + start + 1;
    let name = line[start + 1..end].trim();
    (!name.is_empty()).then(|| name.to_string())
}

fn trim_trailing_empty_lines(mut lines: Vec<String>) -> Vec<String> {
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    lines
}

fn parse_v2000_molfile(molblock: &str) -> Result<Molfile, String> {
    let normalized = molblock.replace('\r', "");
    let lines: Vec<&str> = normalized.lines().collect();
    if lines.len() < 4 {
        return Err("Molfile is too short.".to_string());
    }
    let counts_line = lines[3];
    if counts_line.contains("V3000") {
        return Err("V3000 molfile is not supported yet.".to_string());
    }
    let atom_count = parse_usize(slice_ascii(counts_line, 0, 3))
        .ok_or_else(|| "Invalid atom count in V2000 counts line.".to_string())?;
    let bond_count = parse_usize(slice_ascii(counts_line, 3, 6))
        .ok_or_else(|| "Invalid bond count in V2000 counts line.".to_string())?;
    if lines.len() < 4 + atom_count + bond_count {
        return Err("Molfile ended before atom and bond blocks completed.".to_string());
    }

    let mut atoms = Vec::with_capacity(atom_count);
    for index in 0..atom_count {
        let line = lines[4 + index];
        let symbol = slice_ascii(line, 31, 34).trim();
        atoms.push(MolAtom {
            x: parse_f64(slice_ascii(line, 0, 10)).unwrap_or(0.0),
            y: parse_f64(slice_ascii(line, 10, 20)).unwrap_or(0.0),
            symbol: if symbol.is_empty() {
                "C".to_string()
            } else {
                normalize_element_symbol(symbol)
            },
            charge: 0,
        });
    }

    let mut bonds = Vec::with_capacity(bond_count);
    for index in 0..bond_count {
        let line = lines[4 + atom_count + index];
        let begin = parse_usize(slice_ascii(line, 0, 3))
            .unwrap_or(1)
            .saturating_sub(1);
        let end = parse_usize(slice_ascii(line, 3, 6))
            .unwrap_or(1)
            .saturating_sub(1);
        if begin >= atom_count || end >= atom_count || begin == end {
            continue;
        }
        bonds.push(MolBond {
            begin,
            end,
            order: parse_u8(slice_ascii(line, 6, 9)).unwrap_or(1).max(1),
            stereo: parse_u8(slice_ascii(line, 9, 12)).unwrap_or(0),
        });
    }

    for line in lines.iter().skip(4 + atom_count + bond_count) {
        if line.starts_with("M  CHG") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let pair_count = parts
                .get(2)
                .and_then(|value| parse_usize(value))
                .unwrap_or(0);
            for pair_index in 0..pair_count {
                let atom_index = parts
                    .get(3 + pair_index * 2)
                    .and_then(|value| parse_usize(value))
                    .unwrap_or(1)
                    .saturating_sub(1);
                let charge = parts
                    .get(4 + pair_index * 2)
                    .and_then(|value| value.parse::<i32>().ok())
                    .unwrap_or(0);
                if let Some(atom) = atoms.get_mut(atom_index) {
                    atom.charge = charge;
                }
            }
        }
    }

    if atoms.is_empty() {
        return Err("Molfile contains no atoms.".to_string());
    }

    Ok(Molfile {
        title: lines[0].trim().to_string(),
        atoms,
        bonds,
    })
}

fn molfile_to_fragment(molfile: &Molfile, record: &SdfRecord) -> (MoleculeFragment, f64, f64) {
    let min_x = molfile
        .atoms
        .iter()
        .map(|atom| atom.x)
        .fold(f64::INFINITY, f64::min);
    let max_x = molfile
        .atoms
        .iter()
        .map(|atom| atom.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = molfile
        .atoms
        .iter()
        .map(|atom| atom.y)
        .fold(f64::INFINITY, f64::min);
    let max_y = molfile
        .atoms
        .iter()
        .map(|atom| atom.y)
        .fold(f64::NEG_INFINITY, f64::max);
    let bond_length = median_mol_bond_length(molfile).unwrap_or(1.5).max(0.1);
    let scale = DEFAULT_SDF_BOND_LENGTH_PT / bond_length;
    let padding = 18.0;
    let width = ((max_x - min_x).abs() * scale + padding * 2.0).max(72.0);
    let height = ((max_y - min_y).abs() * scale + padding * 2.0).max(72.0);

    let mut nodes = Vec::with_capacity(molfile.atoms.len());
    for (index, atom) in molfile.atoms.iter().enumerate() {
        let atomic_number = atomic_number_for_symbol(&atom.symbol);
        nodes.push(Node {
            id: format!("n{}", index + 1),
            element: atom.symbol.clone(),
            atomic_number,
            position: [
                round2((atom.x - min_x) * scale + padding),
                round2((max_y - atom.y) * scale + padding),
            ],
            charge: atom.charge,
            num_hydrogens: 0,
            external_connection: None,
            is_placeholder: atomic_number == 0,
            label: None,
            atom_properties: crate::AtomProperties::default(),
            nmr_assignments: Vec::new(),
            meta: json!({
                "import": {
                    "sdf": {
                        "atomIndex": index + 1,
                    }
                }
            }),
        });
    }

    let bonds = molfile
        .bonds
        .iter()
        .enumerate()
        .map(|(index, bond)| {
            let order = match bond.order {
                1..=3 => bond.order,
                _ => 1,
            };
            Bond {
                id: format!("b{}", index + 1),
                begin: format!("n{}", bond.begin + 1),
                end: format!("n{}", bond.end + 1),
                order,
                properties: Default::default(),
                double: (order == 2).then_some(DoubleBond {
                    placement: DoubleBondPlacement::Center,
                    center_exit_side: None,
                    frozen: false,
                }),
                stereo: sdf_bond_stereo(bond),
                stroke_width: crate::DEFAULT_BOND_STROKE,
                stroke: None,
                bold_width: Some(crate::BOLD_BOND_WIDTH_PT.value()),
                wedge_width: Some(crate::SOLID_WEDGE_WIDTH_PT.value()),
                label_clip_margin: None,
                hash_spacing: Some(crate::DEFAULT_HASH_SPACING_PT.value()),
                bond_spacing: Some(crate::DEFAULT_BOND_SPACING_PERCENT),
                margin_width: Some(crate::DEFAULT_BOND_MARGIN_WIDTH_PT.value()),
                line_styles: Default::default(),
                line_weights: Default::default(),
                meta: json!({
                    "import": {
                        "sdf": {
                            "bondIndex": index + 1,
                            "stereo": bond.stereo,
                        }
                    }
                }),
            }
        })
        .collect();

    let fragment = MoleculeFragment {
        schema: "chemsema.molecule.fragment2d".to_string(),
        bbox: [0.0, 0.0, round2(width), round2(height)],
        nodes,
        bonds,
        stereo: Vec::new(),
        interactions: Vec::new(),
        meta: json!({
            "import": {
                "sdf": {
                    "title": empty_string_as_null(&molfile.title),
                    "data": record.data,
                }
            }
        }),
    };
    (fragment, width, height)
}

fn sdf_bond_stereo(bond: &MolBond) -> Option<BondStereo> {
    match bond.stereo {
        1 => Some(BondStereo {
            kind: "solid-wedge".to_string(),
            wide_end: "end".to_string(),
        }),
        6 => Some(BondStereo {
            kind: "hashed-wedge".to_string(),
            wide_end: "end".to_string(),
        }),
        _ => None,
    }
}

fn median_mol_bond_length(molfile: &Molfile) -> Option<f64> {
    let mut lengths = molfile
        .bonds
        .iter()
        .filter_map(|bond| {
            let a = molfile.atoms.get(bond.begin)?;
            let b = molfile.atoms.get(bond.end)?;
            Some((b.x - a.x).hypot(b.y - a.y))
        })
        .filter(|value| *value > 1.0e-6)
        .collect::<Vec<_>>();
    if lengths.is_empty() {
        return None;
    }
    lengths.sort_by(f64::total_cmp);
    Some(lengths[lengths.len() / 2])
}

fn fragment_to_sdf_record(
    document: &ChemSemaDocument,
    object: &SceneObject,
    resource: &Resource,
    fragment: &MoleculeFragment,
) -> Result<String, String> {
    let title = export_record_title(document, object, resource, fragment);
    let mut export_fragment = fragment.clone();
    let options = crate::engine::editor_options_from_document(document);
    crate::engine::expand_complete_labels_in_fragment(&mut export_fragment, &options);
    let fragment = &export_fragment;
    if fragment.nodes.len() > 999 || fragment.bonds.len() > 999 {
        return Err(
            "V2000 SDF export supports at most 999 atoms and 999 bonds per molecule.".to_string(),
        );
    }
    let mut out = String::new();
    writeln!(out, "{title}").unwrap();
    writeln!(out, "  ChemSema").unwrap();
    writeln!(out).unwrap();
    writeln!(
        out,
        "{:>3}{:>3}  0  0  0  0            999 V2000",
        fragment.nodes.len(),
        fragment.bonds.len()
    )
    .unwrap();

    let positions = export_mol_positions(fragment, object);
    for node in &fragment.nodes {
        let (x, y) = positions
            .get(node.id.as_str())
            .copied()
            .unwrap_or((0.0, 0.0));
        writeln!(
            out,
            "{:>10.4}{:>10.4}{:>10.4} {:<3} 0  0  0  0  0  0  0  0  0  0  0  0",
            x,
            y,
            0.0,
            export_node_symbol(node)
        )
        .unwrap();
    }

    let node_index = fragment
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.as_str(), index + 1))
        .collect::<HashMap<_, _>>();
    for bond in &fragment.bonds {
        let begin = *node_index
            .get(bond.begin.as_str())
            .ok_or_else(|| format!("Bond {} references missing begin node.", bond.id))?;
        let end = *node_index
            .get(bond.end.as_str())
            .ok_or_else(|| format!("Bond {} references missing end node.", bond.id))?;
        let (begin, end, stereo) = export_bond_direction(bond, begin, end);
        writeln!(
            out,
            "{:>3}{:>3}{:>3}{:>3}  0  0  0",
            begin,
            end,
            bond.order.clamp(1, 3),
            stereo
        )
        .unwrap();
    }

    let charges = fragment
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| (node.charge != 0).then_some((index + 1, node.charge)))
        .collect::<Vec<_>>();
    for chunk in charges.chunks(8) {
        write!(out, "M  CHG{:>3}", chunk.len()).unwrap();
        for (atom_index, charge) in chunk {
            write!(out, "{:>4}{:>4}", atom_index, charge).unwrap();
        }
        out.push('\n');
    }
    out.push_str("M  END\n");

    for (name, values) in export_sdf_data_fields(resource, fragment) {
        writeln!(out, ">  <{}>", sanitize_sdf_field_name(&name)).unwrap();
        for line in values {
            writeln!(out, "{line}").unwrap();
        }
        writeln!(out).unwrap();
    }
    Ok(out)
}

fn export_record_title(
    document: &ChemSemaDocument,
    object: &SceneObject,
    resource: &Resource,
    fragment: &MoleculeFragment,
) -> String {
    fragment
        .meta
        .pointer("/import/sdf/title")
        .and_then(Value::as_str)
        .or_else(|| {
            resource
                .meta
                .pointer("/import/sdf/title")
                .and_then(Value::as_str)
        })
        .filter(|value| !value.trim().is_empty())
        .or_else(|| (!object.name.trim().is_empty()).then_some(object.name.as_str()))
        .or(Some(document.document.title.as_str()))
        .unwrap_or("ChemSema Molecule")
        .trim()
        .chars()
        .take(80)
        .collect()
}

fn export_mol_positions<'a>(
    fragment: &'a MoleculeFragment,
    object: &SceneObject,
) -> HashMap<&'a str, (f64, f64)> {
    let mut lengths = Vec::new();
    let points = fragment
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), (node.position[0], node.position[1])))
        .collect::<HashMap<_, _>>();
    for bond in &fragment.bonds {
        let Some(a) = points.get(bond.begin.as_str()) else {
            continue;
        };
        let Some(b) = points.get(bond.end.as_str()) else {
            continue;
        };
        let dx = (b.0 - a.0) * object.transform.scale[0];
        let dy = (b.1 - a.1) * object.transform.scale[1];
        let length = dx.hypot(dy);
        if length > 1.0e-6 {
            lengths.push(length);
        }
    }
    lengths.sort_by(f64::total_cmp);
    let median = lengths
        .get(lengths.len().saturating_sub(1) / 2)
        .copied()
        .unwrap_or(DEFAULT_SDF_BOND_LENGTH_PT)
        .max(1.0);
    let scale = 1.5 / median;
    fragment
        .nodes
        .iter()
        .map(|node| {
            let x = (node.position[0] * object.transform.scale[0]) * scale;
            let y = -(node.position[1] * object.transform.scale[1]) * scale;
            (node.id.as_str(), (x, y))
        })
        .collect()
}

fn export_node_symbol(node: &Node) -> String {
    if node.is_placeholder || node.atomic_number == 0 || node.element.trim().is_empty() {
        "*".to_string()
    } else {
        normalize_element_symbol(&node.element)
    }
}

fn export_bond_direction(bond: &Bond, begin: usize, end: usize) -> (usize, usize, u8) {
    let Some(stereo) = bond.stereo.as_ref() else {
        return (begin, end, 0);
    };
    let code = match stereo.kind.as_str() {
        "wedge" | "solid-wedge" => 1,
        "hashed" | "hashed-wedge" => 6,
        _ => return (begin, end, 0),
    };
    if stereo.wide_end == "begin" || stereo.wide_end == bond.begin {
        (end, begin, code)
    } else {
        (begin, end, code)
    }
}

fn export_sdf_data_fields(
    resource: &Resource,
    fragment: &MoleculeFragment,
) -> BTreeMap<String, Vec<String>> {
    let mut fields = BTreeMap::new();
    merge_sdf_data_fields(&mut fields, resource.meta.pointer("/import/sdf/data"));
    merge_sdf_data_fields(&mut fields, fragment.meta.pointer("/import/sdf/data"));
    fields
}

fn merge_sdf_data_fields(fields: &mut BTreeMap<String, Vec<String>>, value: Option<&Value>) {
    let Some(Value::Object(map)) = value else {
        return;
    };
    for (name, value) in map {
        if name.trim().is_empty() {
            continue;
        }
        let lines = match value {
            Value::Array(items) => items
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            Value::String(text) => text.lines().map(ToString::to_string).collect(),
            _ => continue,
        };
        fields.insert(name.clone(), lines);
    }
}

fn sanitize_sdf_field_name(name: &str) -> String {
    name.chars()
        .filter(|ch| *ch != '<' && *ch != '>' && *ch != '\n' && *ch != '\r')
        .collect::<String>()
        .trim()
        .to_string()
}

fn flatten_visible_objects(objects: &[SceneObject]) -> Vec<&SceneObject> {
    let mut out = Vec::new();
    for object in objects {
        if !object.visible {
            continue;
        }
        out.push(object);
        out.extend(flatten_visible_objects(&object.children));
    }
    out
}

fn default_sdf_styles() -> BTreeMap<String, Value> {
    let mut styles = BTreeMap::new();
    styles.insert(
        "style_molecule_default".to_string(),
        json!({
            "kind": "molecule",
            "stroke": "#000000",
            "strokeWidth": crate::DEFAULT_BOND_STROKE,
            "fontFamily": "Arial",
            "fontSize": crate::DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT,
        }),
    );
    styles
}

fn empty_string_as_null(value: &str) -> Value {
    if value.trim().is_empty() {
        Value::Null
    } else {
        json!(value)
    }
}

fn normalize_element_symbol(symbol: &str) -> String {
    let trimmed = symbol.trim();
    if trimmed.is_empty() {
        return "C".to_string();
    }
    let mut chars = trimmed.chars();
    let Some(first) = chars.next() else {
        return "C".to_string();
    };
    let mut out = String::new();
    out.extend(first.to_uppercase());
    for ch in chars {
        out.extend(ch.to_lowercase());
    }
    out
}

fn atomic_number_for_symbol(symbol: &str) -> u8 {
    ELEMENT_SYMBOLS
        .iter()
        .position(|candidate| candidate.eq_ignore_ascii_case(symbol.trim()))
        .unwrap_or(0) as u8
}

fn slice_ascii(input: &str, start: usize, end: usize) -> &str {
    input.get(start..end).unwrap_or_default()
}

fn parse_usize(value: &str) -> Option<usize> {
    value.trim().parse().ok()
}

fn parse_u8(value: &str) -> Option<u8> {
    value.trim().parse().ok()
}

fn parse_f64(value: &str) -> Option<f64> {
    value.trim().parse().ok()
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

const ELEMENT_SYMBOLS: [&str; 119] = [
    "", "H", "He", "Li", "Be", "B", "C", "N", "O", "F", "Ne", "Na", "Mg", "Al", "Si", "P", "S",
    "Cl", "Ar", "K", "Ca", "Sc", "Ti", "V", "Cr", "Mn", "Fe", "Co", "Ni", "Cu", "Zn", "Ga", "Ge",
    "As", "Se", "Br", "Kr", "Rb", "Sr", "Y", "Zr", "Nb", "Mo", "Tc", "Ru", "Rh", "Pd", "Ag", "Cd",
    "In", "Sn", "Sb", "Te", "I", "Xe", "Cs", "Ba", "La", "Ce", "Pr", "Nd", "Pm", "Sm", "Eu", "Gd",
    "Tb", "Dy", "Ho", "Er", "Tm", "Yb", "Lu", "Hf", "Ta", "W", "Re", "Os", "Ir", "Pt", "Au", "Hg",
    "Tl", "Pb", "Bi", "Po", "At", "Rn", "Fr", "Ra", "Ac", "Th", "Pa", "U", "Np", "Pu", "Am", "Cm",
    "Bk", "Cf", "Es", "Fm", "Md", "No", "Lr", "Rf", "Db", "Sg", "Bh", "Hs", "Mt", "Ds", "Rg", "Cn",
    "Nh", "Fl", "Mc", "Lv", "Ts", "Og",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sdf_record_into_editable_fragment() {
        let sdf = concat!(
            "Ethanol\n",
            "  ChemSema\n",
            "\n",
            "  3  2  0  0  0  0            999 V2000\n",
            "    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n",
            "    1.5000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n",
            "    3.0000    0.0000    0.0000 O   0  0  0  0  0  0  0  0  0  0  0  0\n",
            "  1  2  1  0  0  0  0\n",
            "  2  3  1  0  0  0  0\n",
            "M  END\n",
            ">  <NOTE>\n",
            "kept\n",
            "\n",
            "$$$$\n",
        );
        let document = parse_sdf_document(sdf, Some("sample.sdf")).expect("sdf should parse");
        assert_eq!(document.objects.len(), 1);
        let resource_ref = document.objects[0].payload.resource_ref.as_ref().unwrap();
        let fragment = document.resources[resource_ref].data.as_fragment().unwrap();
        assert_eq!(fragment.nodes.len(), 3);
        assert_eq!(fragment.bonds.len(), 2);
        assert_eq!(fragment.nodes[2].element, "O");
    }

    #[test]
    fn exports_sdf_records_with_data_fields() {
        let document = parse_sdf_document(
            concat!(
                "Water\n",
                "  ChemSema\n",
                "\n",
                "  1  0  0  0  0  0            999 V2000\n",
                "    0.0000    0.0000    0.0000 O   0  0  0  0  0  0  0  0  0  0  0  0\n",
                "M  END\n",
                ">  <ID>\n",
                "w1\n",
                "\n",
                "$$$$\n",
            ),
            Some("water.sdf"),
        )
        .unwrap();
        let sdf = document_to_sdf(&document).expect("sdf should export");
        assert!(sdf.contains("M  END\n"));
        assert!(sdf.contains(">  <ID>\n"));
        assert!(sdf.contains("w1\n"));
        assert!(sdf.ends_with("$$$$\n"));
    }

    #[test]
    fn sdf_stereo_uses_canonical_wedge_kinds_and_preserves_wide_end() {
        let mut document = parse_sdf_document(
            concat!(
                "Stereo\n",
                "  ChemSema\n",
                "\n",
                "  2  1  0  0  0  0            999 V2000\n",
                "    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n",
                "    1.5000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n",
                "  1  2  1  1  0  0  0\n",
                "M  END\n",
                "$$$$\n",
            ),
            Some("stereo.sdf"),
        )
        .unwrap();
        let resource_ref = document.objects[0]
            .payload
            .resource_ref
            .as_ref()
            .unwrap()
            .clone();
        let fragment = document.resources[&resource_ref]
            .data
            .as_fragment()
            .unwrap();
        assert_eq!(
            fragment.bonds[0].stereo.as_ref().unwrap().kind,
            "solid-wedge"
        );
        assert_eq!(fragment.bonds[0].stereo.as_ref().unwrap().wide_end, "end");

        let round_trip = document_to_sdf(&document).unwrap();
        assert!(round_trip
            .lines()
            .any(|line| line.starts_with("  1  2  1  1")));

        let fragment = document
            .resources
            .get_mut(&resource_ref)
            .unwrap()
            .data
            .as_fragment_mut()
            .unwrap();
        let stereo = fragment.bonds[0].stereo.as_mut().unwrap();
        stereo.kind = "hashed-wedge".to_string();
        stereo.wide_end = "begin".to_string();
        let reversed = document_to_sdf(&document).unwrap();
        assert!(reversed
            .lines()
            .any(|line| line.starts_with("  2  1  1  6")));
    }

    #[test]
    fn sdf_export_expands_complete_recognized_labels() {
        let mut document = parse_sdf_document(
            concat!(
                "Abbreviation\n",
                "  ChemSema\n",
                "\n",
                "  2  1  0  0  0  0            999 V2000\n",
                "    0.0000    0.0000    0.0000 C   0  0  0  0  0  0  0  0  0  0  0  0\n",
                "    1.5000    0.0000    0.0000 *   0  0  0  0  0  0  0  0  0  0  0  0\n",
                "  1  2  1  0  0  0  0\n",
                "M  END\n",
                "$$$$\n",
            ),
            Some("abbreviation.sdf"),
        )
        .unwrap();
        let resource_ref = document.objects[0]
            .payload
            .resource_ref
            .as_ref()
            .unwrap()
            .clone();
        let fragment = document
            .resources
            .get_mut(&resource_ref)
            .unwrap()
            .data
            .as_fragment_mut()
            .unwrap();
        let node = &mut fragment.nodes[1];
        node.element = "*".to_string();
        node.atomic_number = 0;
        node.is_placeholder = true;
        node.meta = json!({
            "labelRecognition": {
                "status": "recognized",
                "expansion": {
                    "complete": true,
                    "atoms": [
                        {"id": "c1", "element": "C"},
                        {"id": "f1", "element": "F"},
                        {"id": "f2", "element": "F"},
                        {"id": "f3", "element": "F"}
                    ],
                    "bonds": [
                        {"begin": "c1", "end": "f1", "order": 1},
                        {"begin": "c1", "end": "f2", "order": 1},
                        {"begin": "c1", "end": "f3", "order": 1}
                    ],
                    "attachments": [{"role": "external", "atomId": "c1"}]
                }
            }
        });

        let sdf = document_to_sdf(&document).unwrap();
        assert!(sdf.lines().nth(3).unwrap().starts_with("  5  4"));
        assert!(!sdf.lines().any(|line| line.get(31..34) == Some("*  ")));
    }
}
