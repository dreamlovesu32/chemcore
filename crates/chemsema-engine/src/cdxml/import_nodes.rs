use super::*;

pub(super) fn validate_external_connection_values(root: &XmlNode) -> Result<(), String> {
    for node in descendants(root)
        .into_iter()
        .filter(|node| node.is("n") && node.attr("NodeType") == Some("ExternalConnectionPoint"))
    {
        cdxml_external_connection_type(node.attr("ExternalConnectionType"))?;
        if let Some(value) = node.attr("ExternalConnectionNum") {
            value.parse::<u16>().map_err(|_| {
                format!(
                    "invalid ExternalConnectionNum `{value}` on node `{}`",
                    node.attr("id").unwrap_or("<missing id>")
                )
            })?;
        }
    }
    Ok(())
}

pub(super) fn normalize_node(
    node: &XmlNode,
    origin: [f64; 2],
    node_positions: &BTreeMap<String, [f64; 2]>,
    colors: &CdxmlColorTable,
    fonts: &BTreeMap<String, String>,
    defaults: CdxmlDefaults,
) -> Option<Node> {
    let id = node.attr("id")?.to_string();
    let position = parse_xy(node.attr("p")).or_else(|| node_positions.get(id.as_str()).copied())?;
    let local_position = [
        round2(position[0] - origin[0]),
        round2(position[1] - origin[1]),
    ];
    let atomic_number = parse_u8(node.attr("Element")).unwrap_or(6);
    let charge = parse_i32(node.attr("Charge")).unwrap_or(0);
    let node_type = node.attr("NodeType").unwrap_or("");
    let (element_list, element_list_excluded) = parse_element_list(node.attr("ElementList"));
    let (generic_list, generic_list_excluded) =
        crate::document::parse_query_string_list(node.attr("GenericList"));
    let mut label = node_label(node, origin, colors, fonts, defaults);
    if let Some(label) = &mut label {
        if label.position.is_none() {
            label.position = Some(local_position);
        }
    }
    if label.is_none() && (!element_list.is_empty() || !generic_list.is_empty()) {
        let mut parts: Vec<String> = element_list
            .iter()
            .map(|value| element_symbol(*value).to_string())
            .collect();
        parts.extend(generic_list.iter().cloned());
        let excluded = element_list_excluded || generic_list_excluded;
        let generated_text = format!("{}{}", if excluded { "NOT " } else { "" }, parts.join(", "));
        let mut generated =
            crate::engine::make_periodic_element_node_label(&generated_text, local_position);
        generated.font_size = Some(defaults.label_size);
        generated.font_family = fonts.get(&defaults.label_font.to_string()).cloned();
        generated.meta = json!({"queryListLabel": {"source": "cdxml-generated"}});
        label = Some(generated);
    } else if label.is_none() && atomic_number != 6 {
        let element = element_symbol(atomic_number);
        let generated_text = match charge {
            0 => element.to_string(),
            1 => format!("{element}+"),
            -1 => format!("{element}-"),
            value if value > 1 => format!("{element}{value}+"),
            value => format!("{element}{}-", value.unsigned_abs()),
        };
        let mut generated =
            crate::engine::make_periodic_element_node_label(&generated_text, local_position);
        generated.font_size = Some(defaults.label_size);
        generated.font_family = Some(
            fonts
                .get(&defaults.label_font.to_string())
                .cloned()
                .unwrap_or_else(|| "Arial".to_string()),
        );
        for run in &mut generated.runs {
            run.font_size = Some(defaults.label_size);
            run.font_family = generated.font_family.clone();
        }
        let inherited_spacing = imported_document_text_style(
            defaults.label_font,
            defaults.label_face,
            defaults.label_size,
            defaults.color,
            colors,
            fonts,
            defaults
                .label_line_height
                .or(defaults.line_height)
                .unwrap_or(CdxmlLineHeight::Variable),
        );
        generated.line_height = Some(inherited_spacing.line_height);
        generated.line_height_mode = inherited_spacing.line_height_mode;
        generated.line_advances.clear();
        generated.meta = json!({
            "implicitHydrogenLabel": {
                "source": "cdxml-generated",
                "userEdited": false,
            }
        });
        label = Some(generated);
    }
    let is_bullet_carbon = atomic_number == 6
        && label
            .as_ref()
            .is_some_and(imported_cdxml_bullet_carbon_node_label);
    let radical = cdxml_atom_radical(node.attr("Radical"));
    let radical_count = radical.electron_count();
    let explicit_num_hydrogens = parse_u8(node.attr("NumHydrogens"));
    let nmr_assignments = parse_nmr_assignments(node, origin, colors, fonts, defaults);
    let mut meta = json!({
        "import": {
            "cdxml": {
                "z": parse_i32(node.attr("Z")),
                "nodeType": empty_as_null(node.attr("NodeType")),
                "hasCollapsedFragment": node.direct_children("fragment").next().is_some(),
                "geometry": empty_as_null(node.attr("Geometry")),
                "bondOrdering": empty_as_null(node.attr("BondOrdering")),
                "hDot": parse_cdxml_bool(node.attr("HDot")).unwrap_or(false),
                "hDash": parse_cdxml_bool(node.attr("HDash")).unwrap_or(false),
                "attachments": empty_as_null(node.attr("Attachments")),
                "enhancedStereoType": empty_as_null(node.attr("EnhancedStereoType")),
                "enhancedStereoGroupNum": empty_as_null(node.attr("EnhancedStereoGroupNum")),
                "elementList": empty_as_null(node.attr("ElementList")),
                "labelDisplay": empty_as_null(node.attr("LabelDisplay")),
                "explicitNumHydrogens": explicit_num_hydrogens,
                "implicitHydrogens": empty_as_null(node.attr("ImplicitHydrogens")),
                "restrictImplicitHydrogens": parse_cdxml_bool(node.attr("ImplicitHydrogens")).unwrap_or(false),
                "generatedPosition": node.attr("p").is_none(),
            }
        }
    });
    if radical_count != 0 {
        meta["radicalCount"] = json!(radical_count);
    }
    Some(Node {
        id,
        element: element_symbol(atomic_number).to_string(),
        atomic_number,
        position: local_position,
        charge,
        num_hydrogens: explicit_num_hydrogens.unwrap_or(0),
        highlight_color: node
            .attr("highlightColor")
            .map(|color| colors.resolve(Some(color))),
        external_connection: (node_type == "ExternalConnectionPoint").then(|| {
            crate::ExternalConnection {
                connection_type: cdxml_external_connection_type(
                    node.attr("ExternalConnectionType"),
                )
                .expect("external connection values are validated before normalization"),
                number: node
                    .attr("ExternalConnectionNum")
                    .and_then(|value| value.parse::<u16>().ok()),
            }
        }),
        is_placeholder: matches!(
            node_type,
            "Fragment" | "Nickname" | "GenericNickname" | "Unspecified"
        ) && !is_bullet_carbon,
        label,
        atom_properties: crate::AtomProperties {
            isotope_mass: parse_i16(node.attr("Isotope")),
            isotopic_abundance: cdxml_isotopic_abundance(node.attr("IsotopicAbundance")),
            radical,
            atom_number: nonempty_string(node.attr("AtomNumber")),
            show_atom_number: node
                .attr("ShowAtomNumber")
                .and_then(|value| parse_cdxml_bool(Some(value))),
            cip_stereo: nonempty_string(node.attr("AS"))
                .filter(|value| !matches!(value.as_str(), "N" | "U")),
            show_atom_stereo: node
                .attr("ShowAtomStereo")
                .and_then(|value| parse_cdxml_bool(Some(value))),
            atom_number_position: None,
            stereo_position: None,
            element_list,
            element_list_excluded,
            generic_list,
            generic_list_excluded,
            free_sites: parse_u8(node.attr("FreeSites")),
            show_atom_query: node
                .attr("ShowAtomQuery")
                .and_then(|value| parse_cdxml_bool(Some(value))),
            ring_bond_count: cdxml_ring_bond_count(node.attr("RingBondCount")),
            unsaturated_bonds: cdxml_unsaturated_bonds(node.attr("UnsaturatedBonds")),
            substituents_up_to: parse_u8(node.attr("SubstituentsUpTo")),
            substituents_exactly: parse_u8(node.attr("SubstituentsExactly")),
            translation: cdxml_query_translation(node.attr("Translation")),
            abnormal_valence: parse_cdxml_bool(node.attr("AbnormalValence")).unwrap_or(false),
            reaction_change: parse_cdxml_bool(node.attr("RxnChange")).unwrap_or(false),
            reaction_stereo: cdxml_atom_reaction_stereo(node.attr("RxnStereo")),
            show_terminal_carbon_label: node
                .attr("ShowTerminalCarbonLabels")
                .and_then(|value| parse_cdxml_bool(Some(value))),
            show_non_terminal_carbon_label: node
                .attr("ShowNonTerminalCarbonLabels")
                .and_then(|value| parse_cdxml_bool(Some(value))),
        },
        nmr_assignments,
        meta,
    })
}

fn parse_nmr_assignments(
    node: &XmlNode,
    origin: [f64; 2],
    colors: &CdxmlColorTable,
    fonts: &BTreeMap<String, String>,
    defaults: CdxmlDefaults,
) -> Vec<crate::NmrAssignment> {
    node.direct_children("objecttag")
        .filter(|tag| tag.attr("Name") == Some("/CS/CD/assign"))
        .filter_map(|tag| {
            let mut label = node_label(tag, origin, colors, fonts, defaults)?;
            let shift_ppm = label.text.trim().parse::<f64>().ok()?;
            if !shift_ppm.is_finite() {
                return None;
            }
            if let Some(text) = tag.direct_children("t").next() {
                if let Some(point) = parse_xy(text.attr("p")) {
                    label.position =
                        Some([round2(point[0] - origin[0]), round2(point[1] - origin[1])]);
                }
                if label.bbox().is_none() {
                    label.box_field = parse_bbox(text.attr("BoundingBox")).map(|bbox| {
                        [
                            round2(bbox[0] - origin[0]),
                            round2(bbox[1] - origin[1]),
                            round2(bbox[2] - origin[0]),
                            round2(bbox[3] - origin[1]),
                        ]
                    });
                }
            }
            label.meta["defaultChemical"] = json!(false);
            label.meta["chemicalCheck"] = json!(false);
            let (range_low_ppm, range_high_ppm) =
                parse_nmr_assignment_range(tag.attr("Value"), shift_ppm);
            let quality = match label
                .fill
                .as_deref()
                .unwrap_or("")
                .to_ascii_lowercase()
                .as_str()
            {
                "#0000ff" => crate::NmrAssignmentQuality::Good,
                "#ff00ff" => crate::NmrAssignmentQuality::Medium,
                "#ff0000" => crate::NmrAssignmentQuality::Rough,
                _ => crate::NmrAssignmentQuality::Unknown,
            };
            Some(crate::NmrAssignment {
                nucleus: crate::NmrNucleus::Unknown,
                shift_ppm,
                range_low_ppm,
                range_high_ppm,
                quality,
                label,
            })
        })
        .collect()
}

fn parse_nmr_assignment_range(value: Option<&str>, shift_ppm: f64) -> (f64, f64) {
    let value = value.unwrap_or("").trim().trim_end_matches(',');
    let separator = value
        .char_indices()
        .skip(1)
        .find_map(|(index, character)| (character == '-').then_some(index));
    let Some(separator) = separator else {
        return (shift_ppm, shift_ppm);
    };
    let low = value[..separator].trim().parse::<f64>().ok();
    let high = value[separator + 1..].trim().parse::<f64>().ok();
    match (low, high) {
        (Some(low), Some(high)) if low.is_finite() && high.is_finite() && low <= high => {
            (low, high)
        }
        _ => (shift_ppm, shift_ppm),
    }
}

fn cdxml_external_connection_type(
    value: Option<&str>,
) -> Result<crate::ExternalConnectionType, String> {
    let connection_type = match value {
        None | Some("Unspecified") => crate::ExternalConnectionType::Unspecified,
        Some("Diamond") => crate::ExternalConnectionType::Diamond,
        Some("Star") => crate::ExternalConnectionType::Star,
        Some("PolymerBead") => crate::ExternalConnectionType::PolymerBead,
        Some("Wavy") => crate::ExternalConnectionType::Wavy,
        Some("Residue") => crate::ExternalConnectionType::Residue,
        Some("Peptide") => crate::ExternalConnectionType::Peptide,
        Some("DNA") => crate::ExternalConnectionType::Dna,
        Some("RNA") => crate::ExternalConnectionType::Rna,
        Some("Terminus") => crate::ExternalConnectionType::Terminus,
        Some("Sulfide") => crate::ExternalConnectionType::Sulfide,
        Some("Nucleotide") => crate::ExternalConnectionType::Nucleotide,
        Some("UnlinkedBranch") => crate::ExternalConnectionType::UnlinkedBranch,
        Some(value) => return Err(format!("invalid ExternalConnectionType `{value}`")),
    };
    Ok(connection_type)
}

fn parse_element_list(value: Option<&str>) -> (Vec<u8>, bool) {
    let mut tokens = value.unwrap_or("").split_whitespace();
    let first = tokens.next();
    let excluded = first.is_some_and(|value| value.eq_ignore_ascii_case("NOT"));
    let values = first
        .filter(|_| !excluded)
        .into_iter()
        .chain(tokens)
        .filter_map(|value| value.parse::<u8>().ok())
        .collect();
    (values, excluded)
}

fn cdxml_ring_bond_count(value: Option<&str>) -> crate::RingBondCount {
    match value.unwrap_or("") {
        "NoRingBonds" => crate::RingBondCount::NoRingBonds,
        "AsDrawn" => crate::RingBondCount::AsDrawn,
        "SimpleRing" => crate::RingBondCount::SimpleRing,
        "Fusion" => crate::RingBondCount::Fusion,
        "SpiroOrHigher" => crate::RingBondCount::SpiroOrHigher,
        _ => crate::RingBondCount::Unspecified,
    }
}

fn cdxml_unsaturated_bonds(value: Option<&str>) -> crate::UnsaturatedBonds {
    match value.unwrap_or("") {
        "MustBeAbsent" => crate::UnsaturatedBonds::MustBeAbsent,
        "MustBePresent" => crate::UnsaturatedBonds::MustBePresent,
        _ => crate::UnsaturatedBonds::Unspecified,
    }
}

fn cdxml_query_translation(value: Option<&str>) -> crate::QueryTranslation {
    match value.unwrap_or("") {
        "Broad" => crate::QueryTranslation::Broad,
        "Narrow" => crate::QueryTranslation::Narrow,
        "Any" => crate::QueryTranslation::Any,
        _ => crate::QueryTranslation::Equal,
    }
}

fn cdxml_atom_reaction_stereo(value: Option<&str>) -> crate::AtomReactionStereo {
    match value.unwrap_or("") {
        "Inversion" => crate::AtomReactionStereo::Inversion,
        "Retention" => crate::AtomReactionStereo::Retention,
        _ => crate::AtomReactionStereo::Unspecified,
    }
}

pub(super) fn cdxml_atom_radical(value: Option<&str>) -> crate::AtomRadical {
    match value.unwrap_or("").trim().to_ascii_lowercase().as_str() {
        "singlet" | "divalentsinglet" => crate::AtomRadical::Singlet,
        "doublet" | "monovalent" | "radical" => crate::AtomRadical::Doublet,
        "triplet" | "divalent" | "divalenttriplet" => crate::AtomRadical::Triplet,
        _ => crate::AtomRadical::None,
    }
}

pub(super) fn cdxml_isotopic_abundance(value: Option<&str>) -> crate::IsotopicAbundance {
    match value.unwrap_or("").trim().to_ascii_lowercase().as_str() {
        "any" => crate::IsotopicAbundance::Any,
        "natural" => crate::IsotopicAbundance::Natural,
        "enriched" => crate::IsotopicAbundance::Enriched,
        "deficient" => crate::IsotopicAbundance::Deficient,
        "nonnatural" | "non-natural" => crate::IsotopicAbundance::Nonnatural,
        _ => crate::IsotopicAbundance::Unspecified,
    }
}

pub(super) fn imported_cdxml_bullet_carbon_node_label(label: &NodeLabel) -> bool {
    label.attachment.as_deref() == Some("node")
        && label.source_text.as_deref().unwrap_or(label.text.as_str()) == "•"
        && label.meta.pointer("/import/cdxml/boundingBox").is_some()
        && label.meta.pointer("/import/cdxml/textPosition").is_some()
}

pub(super) fn node_label(
    node: &XmlNode,
    origin: [f64; 2],
    colors: &CdxmlColorTable,
    fonts: &BTreeMap<String, String>,
    defaults: CdxmlDefaults,
) -> Option<NodeLabel> {
    let text_el = node.direct_children("t").next()?;
    let text = text_el
        .attr("UTF8Text")
        .map(ToString::to_string)
        .unwrap_or_else(|| text_el.full_text())
        .trim()
        .to_string();
    if text.is_empty() {
        return None;
    }
    let bbox = parse_bbox(text_el.attr("BoundingBox"));
    let explicit_interpret_chemically = parse_cdxml_bool(text_el.attr("InterpretChemically"))
        .or_else(|| parse_cdxml_bool(node.attr("InterpretChemically")));
    let parent_face = parse_u32(text_el.attr("face")).unwrap_or(defaults.label_face);
    let interpret_chemically = explicit_interpret_chemically
        .or(defaults.interpret_chemically)
        // A text child of a node is an atom/fragment label by construction.
        // Face controls its appearance; absent semantic settings still use
        // ChemDraw's normal chemically interpreted node-label behavior.
        .unwrap_or(true);
    let default_label_font = defaults.label_font.to_string();
    let parent_font = text_el
        .attr("font")
        .or_else(|| {
            text_el
                .direct_children("s")
                .find_map(|run| run.attr("font"))
        })
        .unwrap_or(default_label_font.as_str());
    let parent_color = text_el
        .attr("color")
        .or_else(|| {
            text_el
                .direct_children("s")
                .find_map(|run| run.attr("color"))
        })
        .unwrap_or("0");
    let parent_size = parse_f64(text_el.attr("size")).unwrap_or_else(|| {
        text_el
            .direct_children("s")
            .find_map(|run| parse_f64(run.attr("size")))
            .unwrap_or(defaults.label_size)
    });
    let mut source_runs: Vec<LabelRun> = text_el
        .direct_children("s")
        .filter_map(|run| {
            let run_text = run.full_text();
            (!run_text.is_empty()).then(|| {
                label_source_run(
                    &run_text,
                    parse_u32(run.attr("face")).unwrap_or(parent_face),
                    run.attr("font").unwrap_or(parent_font),
                    run.attr("color").unwrap_or(parent_color),
                    parse_f64(run.attr("size")).unwrap_or(parent_size),
                    colors,
                    fonts,
                )
            })
        })
        .collect();
    let (text, wrapped_source_runs, normalized_line_starts) =
        if text_el.attr("WordWrapWidth").is_some() || text_el.attr("LineStarts").is_some() {
            apply_cdxml_line_starts(&text, source_runs, text_el.attr("LineStarts"))
        } else {
            (text, source_runs, None)
        };
    source_runs = wrapped_source_runs;
    let runs = label_display_runs_from_source_runs(&source_runs);
    let line_runs = if text.contains('\n') {
        split_label_runs_by_line(&runs)
    } else {
        Vec::new()
    };
    let text_position = parse_xy(text_el.attr("p")).or_else(|| parse_xy(node.attr("p")));
    let local_node_position = parse_xy(node.attr("p"))
        .map(|point| [round2(point[0] - origin[0]), round2(point[1] - origin[1])]);
    let label_display = node.attr("LabelDisplay");
    let label_justification = text_el
        .attr("LabelJustification")
        .or_else(|| text_el.attr("Justification"))
        .or(Some(defaults.label_justification.as_cdxml()));
    let inferred_align = infer_cdxml_label_align(
        label_display,
        label_justification,
        text_el.attr("LabelAlignment"),
    );
    let is_centered = inferred_align == "center";
    let layout = is_centered.then(|| "attached-group-center".to_string());
    let line_spacing =
        resolved_cdxml_label_line_spacing(text_el, defaults, parent_size, &runs, &line_runs);
    Some(NodeLabel {
        text: text.clone(),
        source_text: Some(text.clone()),
        position: local_node_position,
        box_field: None,
        runs: if line_runs.is_empty() {
            runs
        } else {
            Vec::new()
        },
        line_runs,
        lines: if text.contains('\n') {
            text.lines().map(ToString::to_string).collect()
        } else {
            Vec::new()
        },
        align: Some(inferred_align.to_string()),
        layout,
        attachment: Some("node".to_string()),
        anchor: Some(
            match inferred_align {
                "center" => "middle",
                "right" => "end",
                _ => "start",
            }
            .to_string(),
        ),
        font_family: Some(
            fonts
                .get(parent_font)
                .cloned()
                .unwrap_or_else(|| "Arial".to_string()),
        ),
        fill: Some(colors.resolve(Some(parent_color))),
        font_size: Some(parent_size),
        line_height: Some(round2(line_spacing.line_height)),
        line_height_mode: line_spacing.mode.to_string(),
        line_advances: line_spacing
            .line_advances
            .iter()
            .copied()
            .map(round2)
            .collect(),
        glyph_polygons: Vec::new(),
        glyph_clip_polygons: Vec::new(),
        box_value: None,
        meta: json!({
            "import": {
                "cdxml": {
                    "textPosition": text_position,
                    "boundingBox": bbox,
                    "sourceId": empty_as_null(text_el.attr("id")),
                    "labelDisplay": empty_as_null(label_display),
                    "labelAlignment": empty_as_null(text_el.attr("LabelAlignment")),
                    "labelJustification": empty_as_null(text_el.attr("LabelJustification")),
                    "justification": empty_as_null(text_el.attr("Justification")),
                    "lineHeight": empty_as_null(text_el.attr("LineHeight")),
                    "labelLineHeight": empty_as_null(text_el.attr("LabelLineHeight")),
                    "wordWrapWidth": empty_as_null(text_el.attr("WordWrapWidth")),
                    "lineStarts": normalized_line_starts,
                    "resolvedLineHeight": round2(line_spacing.line_height),
                    "resolvedLineHeightMode": line_spacing.mode,
                    "interpretChemically": interpret_chemically,
                    "interpretChemicallyExplicit": explicit_interpret_chemically.is_some(),
                    "marginWidth": defaults.margin_width,
                    "naturalOutsetPt": defaults.margin_width,
                    "circleRadiusPt": defaults.margin_width * 2.0,
                }
            },
            "defaultChemical": interpret_chemically,
            "implicitHydrogenLabel": {
                "source": "cdxml",
                "userEdited": true,
            },
            "sourceRuns": source_runs,
        }),
    })
}

pub(super) fn split_label_runs_by_line(runs: &[LabelRun]) -> Vec<Vec<LabelRun>> {
    let mut lines = vec![Vec::new()];
    for run in runs {
        let parts: Vec<&str> = run.text.split('\n').collect();
        for (index, part) in parts.iter().enumerate() {
            if !part.is_empty() {
                let mut part_run = run.clone();
                part_run.text = (*part).to_string();
                lines.last_mut().expect("line run bucket").push(part_run);
            }
            if index + 1 < parts.len() {
                lines.push(Vec::new());
            }
        }
    }
    lines
}

pub(super) fn apply_cdxml_line_starts(
    text: &str,
    runs: Vec<LabelRun>,
    line_starts: Option<&str>,
) -> (String, Vec<LabelRun>, Option<String>) {
    if line_starts.is_none() {
        return (text.to_string(), runs, None);
    }
    // CDXML stores zero-based offsets into the authored styled-text stream.
    // End-of-line characters are part of that stream and therefore advance
    // subsequent offsets even though they normalize to a single rendered LF.
    // The final offset may be the end-of-text sentinel.
    let raw_len = runs
        .iter()
        .map(|run| run.text.len())
        .sum::<usize>()
        .max(text.len());
    let raw_starts = line_starts
        .into_iter()
        .flat_map(str::split_whitespace)
        .filter_map(|value| value.parse::<usize>().ok())
        .filter(|offset| *offset > 0 && *offset < raw_len)
        .collect::<Vec<_>>();
    let has_end_sentinel = line_starts
        .into_iter()
        .flat_map(str::split_whitespace)
        .filter_map(|value| value.parse::<usize>().ok())
        .any(|offset| offset >= raw_len);
    let starts = raw_starts.iter().copied().collect::<BTreeSet<_>>();
    let source_runs = if runs.is_empty() {
        vec![LabelRun {
            text: text.to_string(),
            ..LabelRun::default()
        }]
    } else {
        runs
    };
    let mut offset = 0usize;
    let mut output_ends_with_newline = false;
    let mut previous_was_carriage_return = false;
    let mut wrapped_runs = Vec::with_capacity(source_runs.len() + starts.len());
    for run in source_runs {
        let mut current = run.clone();
        current.text.clear();
        for character in run.text.chars() {
            let is_newline = matches!(character, '\r' | '\n');
            if starts.contains(&offset) && !output_ends_with_newline && !is_newline {
                current.text.push('\n');
            }
            if is_newline {
                if character != '\n' || !previous_was_carriage_return {
                    current.text.push('\n');
                }
                output_ends_with_newline = true;
            } else {
                current.text.push(character);
                output_ends_with_newline = false;
            }
            previous_was_carriage_return = character == '\r';
            offset += character.len_utf8();
        }
        if !current.text.is_empty() {
            wrapped_runs.push(current);
        }
    }

    let text = wrapped_runs
        .iter()
        .map(|run| run.text.as_str())
        .collect::<String>();
    // Once authored wrap positions have been materialized as LF characters,
    // their offsets must describe that materialized stream. Re-exporting the
    // original offsets alongside the inserted LFs shifts every later break and
    // causes another LF to be inserted on each save. CDXML offsets count UTF-8
    // bytes, and an existing LF advances the following line start by one byte.
    let mut normalized_starts = text
        .bytes()
        .enumerate()
        .filter_map(|(offset, byte)| (byte == b'\n').then_some(offset + 1))
        .take(raw_starts.len())
        .collect::<Vec<_>>();
    if has_end_sentinel {
        normalized_starts.push(text.len());
    }
    let normalized_line_starts = (!normalized_starts.is_empty()).then(|| {
        normalized_starts
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ")
    });
    (text, wrapped_runs, normalized_line_starts)
}

pub(super) fn attr_eq_ignore_ascii_case(value: Option<&str>, expected: &str) -> bool {
    value.is_some_and(|value| value.eq_ignore_ascii_case(expected))
}

pub(super) fn infer_cdxml_label_align(
    label_display: Option<&str>,
    label_justification: Option<&str>,
    label_alignment: Option<&str>,
) -> &'static str {
    if attr_eq_ignore_ascii_case(label_display, "Center") {
        "center"
    } else if attr_eq_ignore_ascii_case(label_display, "Right") {
        "right"
    } else if attr_eq_ignore_ascii_case(label_display, "Left") {
        "left"
    } else if attr_eq_ignore_ascii_case(label_alignment, "Center") {
        "center"
    } else if attr_eq_ignore_ascii_case(label_alignment, "Right") {
        "right"
    } else if attr_eq_ignore_ascii_case(label_alignment, "Left") {
        "left"
    } else if attr_eq_ignore_ascii_case(label_justification, "Center") {
        "center"
    } else if attr_eq_ignore_ascii_case(label_justification, "Right") {
        "right"
    } else {
        "left"
    }
}
