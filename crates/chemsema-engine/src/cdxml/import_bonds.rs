use super::*;

pub(super) fn normalize_bond(
    bond: &XmlNode,
    index: usize,
    node_ids: &BTreeSet<String>,
    nodes: &[Node],
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
) -> Option<Bond> {
    let begin = bond.attr("B")?.to_string();
    let end = bond.attr("E")?.to_string();
    if !node_ids.contains(&begin) || !node_ids.contains(&end) {
        return None;
    }
    let display = bond.attr("Display").unwrap_or("");
    let display2 = bond.attr("Display2").unwrap_or("");
    let source_order = bond.attr("Order").unwrap_or("");
    let is_aromatic =
        parse_f64(Some(source_order)).is_some_and(|order| (order - 1.5).abs() <= EPSILON);
    let is_topology_only_aromatic_dash = is_aromatic
        && display == "Dash"
        && display2.is_empty()
        && [&begin, &end].iter().all(|node_id| {
            nodes.iter().any(|node| {
                node.id == **node_id
                    && node
                        .meta
                        .pointer("/import/cdxml/generatedPosition")
                        .and_then(Value::as_bool)
                        == Some(true)
            })
        });
    let stroke_width = parse_f64(bond.attr("LineWidth")).unwrap_or(defaults.line_width);
    let bold_width = parse_f64(bond.attr("BoldWidth")).unwrap_or(defaults.bold_width);
    let hash_spacing = parse_f64(bond.attr("HashSpacing")).unwrap_or(defaults.hash_spacing);
    // ChemDraw gives the absolute spacing field precedence when both encodings
    // are present. Keep it as an independent native value: unlike percentage
    // spacing, absolute spacing is not subject to the 2.5 * LineWidth floor.
    let bond_spacing_abs = parse_f64(bond.attr("BondSpacingAbs"));
    let bond_spacing = parse_f64(bond.attr("BondSpacing")).unwrap_or(defaults.bond_spacing);
    let stereo = match display {
        "WedgeBegin" => Some(BondStereo {
            kind: "solid-wedge".to_string(),
            wide_end: "end".to_string(),
        }),
        "WedgeEnd" => Some(BondStereo {
            kind: "solid-wedge".to_string(),
            wide_end: "begin".to_string(),
        }),
        "WedgedHashBegin" => Some(BondStereo {
            kind: "hashed-wedge".to_string(),
            wide_end: "end".to_string(),
        }),
        "WedgedHashEnd" => Some(BondStereo {
            kind: "hashed-wedge".to_string(),
            wide_end: "begin".to_string(),
        }),
        "HollowWedgeBegin" => Some(BondStereo {
            kind: "hollow-wedge".to_string(),
            wide_end: "end".to_string(),
        }),
        "HollowWedgeEnd" => Some(BondStereo {
            kind: "hollow-wedge".to_string(),
            wide_end: "begin".to_string(),
        }),
        _ => None,
    };
    let order = if is_aromatic {
        if display2.is_empty() {
            1
        } else {
            2
        }
    } else {
        cdxml_bond_order(bond.attr("Order"))
    };
    let collapsed_fragment_dash_displays_solid = order == 1
        && display == "Dash"
        && [&begin, &end].iter().all(|node_id| {
            nodes.iter().any(|node| {
                node.id == **node_id
                    && node
                        .meta
                        .pointer("/import/cdxml/nodeType")
                        .and_then(Value::as_str)
                        == Some("Fragment")
                    && node
                        .meta
                        .pointer("/import/cdxml/hasCollapsedFragment")
                        .and_then(Value::as_bool)
                        == Some(true)
            })
        });
    let mut line_styles = if is_topology_only_aromatic_dash {
        BondLineStyles::default()
    } else {
        cdxml_bond_line_styles(order, display, display2)
    };
    let mut line_weights = cdxml_bond_line_weights(order, display, display2);
    let placement = match bond
        .attr("DoublePosition")
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "left" => Some((crate::DoubleBondPlacement::Left, true)),
        "right" => Some((crate::DoubleBondPlacement::Right, true)),
        "center" => Some((crate::DoubleBondPlacement::Center, true)),
        _ => None,
    };
    if order >= 2 {
        if let Some((placement, _)) = placement {
            cdxml_apply_line_style_for_double_placement(
                order,
                display,
                display2,
                placement,
                &mut line_styles,
                &mut line_weights,
            );
        }
    }
    let begin_attach = parse_u32(bond.attr("BeginAttach"));
    let end_attach = parse_u32(bond.attr("EndAttach"));
    let source_id = bond.attr("id").filter(|id| !id.trim().is_empty());
    let id = source_id
        .map(str::to_string)
        .unwrap_or_else(|| format!("cdxml_bond_{:03}", index + 1));
    let mut meta = json!({"import": {"cdxml": {
        "z": parse_i32(bond.attr("Z")),
        "display": empty_as_null(bond.attr("Display")),
        "display2": empty_as_null(bond.attr("Display2")),
        "doublePosition": empty_as_null(bond.attr("DoublePosition")),
        "order": empty_as_null(bond.attr("Order")),
        "sourceId": source_id,
        "generatedId": source_id.is_none(),
        "aromatic": is_aromatic,
        "topologyOnlyAromaticDash": is_topology_only_aromatic_dash,
        "collapsedFragmentDashDisplaysSolid": collapsed_fragment_dash_displays_solid,
        "bondSpacingAbs": bond_spacing_abs,
    }}});
    if let Some(value) = bond.attr("CrossingBonds") {
        let crossing_bonds: Vec<_> = value
            .split_whitespace()
            .filter(|id| !id.is_empty())
            .map(str::to_string)
            .collect();
        meta.pointer_mut("/import/cdxml")
            .and_then(Value::as_object_mut)
            .expect("bond CDXML metadata must be an object")
            .insert("crossingBonds".to_string(), json!(crossing_bonds));
    }
    if begin_attach.is_some() || end_attach.is_some() {
        let mut attachments = serde_json::Map::new();
        if let Some(value) = begin_attach {
            attachments.insert(
                "begin".to_string(),
                semantic_endpoint_attachment(nodes, &begin, value),
            );
        }
        if let Some(value) = end_attach {
            attachments.insert(
                "end".to_string(),
                semantic_endpoint_attachment(nodes, &end, value),
            );
        }
        meta.as_object_mut()
            .expect("bond metadata must be an object")
            .insert(
                "endpointAttachments".to_string(),
                Value::Object(attachments),
            );
    }
    Some(Bond {
        id,
        begin,
        end,
        order,
        highlight_color: bond
            .attr("highlightColor")
            .map(|color| colors.resolve(Some(color))),
        properties: crate::BondProperties {
            query_orders: cdxml_bond_query_orders(bond.attr("Order")),
            topology: cdxml_bond_topology(bond.attr("Topology")),
            reaction_participation: cdxml_bond_reaction_participation(
                bond.attr("RxnParticipation"),
            ),
            absolute_stereo: cdxml_bond_absolute_stereo(bond.attr("BS")),
            show_query: bond
                .attr("ShowBondQuery")
                .and_then(|value| parse_cdxml_bool(Some(value))),
            show_reaction: bond
                .attr("ShowBondRxn")
                .and_then(|value| parse_cdxml_bool(Some(value))),
            show_stereo: bond
                .attr("ShowBondStereo")
                .and_then(|value| parse_cdxml_bool(Some(value))),
        },
        double: placement.map(|(placement, frozen)| crate::DoubleBond {
            placement,
            center_exit_side: None,
            frozen,
        }),
        stereo,
        stroke_width,
        stroke: bond.attr("color").map(|color| colors.resolve(Some(color))),
        bold_width: Some(bold_width),
        wedge_width: Some(cdxml_import_wedge_width(stroke_width, bold_width)),
        label_clip_margin: None,
        hash_spacing: Some(hash_spacing),
        bond_spacing: Some(bond_spacing),
        bond_spacing_absolute: bond_spacing_abs,
        margin_width: None,
        line_styles,
        line_weights,
        meta,
    })
}

pub(super) fn semantic_endpoint_attachment(
    nodes: &[Node],
    node_id: &str,
    character_index: u32,
) -> Value {
    let character = nodes
        .iter()
        .find(|node| node.id == node_id)
        .and_then(|node| node.label.as_ref())
        .and_then(|label| {
            label
                .source_text
                .as_deref()
                .unwrap_or(&label.text)
                .chars()
                .nth(character_index as usize)
        })
        .map(|character| character.to_string());
    json!({
        "target": "label-character",
        "characterIndex": character_index,
        "character": character,
    })
}

pub(super) fn infer_cdxml_ring_double_bond_placements(fragment: &mut MoleculeFragment) {
    infer_unspecified_cdxml_double_bond_placements(fragment);
}

pub(super) fn infer_unspecified_cdxml_double_bond_placements(fragment: &mut MoleculeFragment) {
    let inferred: Vec<_> = fragment
        .bonds
        .iter()
        .enumerate()
        .filter_map(|(index, bond)| {
            if bond.order != 2
                || bond.double.is_some()
                || cdxml_bond_has_explicit_double_position(bond)
            {
                return None;
            }
            let placement = crate::engine::automatic_double_bond_placement_for_segment(
                fragment,
                &bond.begin,
                &bond.end,
                Some(&bond.id),
            );
            Some((index, placement))
        })
        .collect();
    for (index, placement) in inferred {
        cdxml_apply_imported_line_style_for_current_double_placement(
            &mut fragment.bonds[index],
            placement,
        );
        fragment.bonds[index].double = Some(DoubleBond {
            placement,
            center_exit_side: None,
            frozen: false,
        });
    }
}

pub(super) fn cdxml_bond_has_explicit_double_position(bond: &Bond) -> bool {
    bond.meta
        .pointer("/import/cdxml/doublePosition")
        .and_then(Value::as_str)
        .is_some_and(|value| !value.trim().is_empty())
}

pub(super) fn cdxml_apply_imported_line_style_for_current_double_placement(
    bond: &mut Bond,
    placement: crate::DoubleBondPlacement,
) {
    let display = bond
        .meta
        .pointer("/import/cdxml/display")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let display2 = bond
        .meta
        .pointer("/import/cdxml/display2")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    cdxml_apply_line_style_for_double_placement(
        bond.order,
        &display,
        &display2,
        placement,
        &mut bond.line_styles,
        &mut bond.line_weights,
    );
}

pub(super) fn cdxml_apply_line_style_for_double_placement(
    order: u8,
    display: &str,
    display2: &str,
    placement: crate::DoubleBondPlacement,
    line_styles: &mut BondLineStyles,
    line_weights: &mut BondLineWeights,
) {
    if order < 2 {
        return;
    }
    if placement == crate::DoubleBondPlacement::Center {
        *line_styles = cdxml_bond_line_styles(order, display, display2);
        *line_weights = cdxml_bond_line_weights(order, display, display2);
        return;
    }

    *line_styles = BondLineStyles::default();
    *line_weights = BondLineWeights::default();
    if matches!(display, "Dash" | "Hash") {
        line_styles.main = crate::BondLinePattern::Dashed;
    } else if display == "Wavy" {
        line_styles.main = crate::BondLinePattern::Wavy;
    }
    if display == "Bold" {
        line_weights.main = crate::BondLineWeight::Bold;
    }

    let outer_style = match placement {
        crate::DoubleBondPlacement::Left => &mut line_styles.left,
        crate::DoubleBondPlacement::Right => &mut line_styles.right,
        crate::DoubleBondPlacement::Center => unreachable!(),
    };
    if matches!(display2, "Dash" | "Hash") {
        *outer_style = crate::BondLinePattern::Dashed;
    }

    let outer_weight = match placement {
        crate::DoubleBondPlacement::Left => &mut line_weights.left,
        crate::DoubleBondPlacement::Right => &mut line_weights.right,
        crate::DoubleBondPlacement::Center => unreachable!(),
    };
    if display2 == "Bold" {
        *outer_weight = crate::BondLineWeight::Bold;
    }
}

pub(super) fn cdxml_import_wedge_width(_stroke_width: f64, bold_width: f64) -> f64 {
    (bold_width * crate::WEDGE_BOLD_WIDTH_MULTIPLIER).max(crate::DEFAULT_BOND_STROKE)
}

pub(super) fn cdxml_bond_order(value: Option<&str>) -> u8 {
    let order = parse_f64(value).unwrap_or(1.0);
    if order >= 2.5 {
        3
    } else if order >= 1.5 {
        2
    } else {
        1
    }
}

fn cdxml_bond_query_orders(value: Option<&str>) -> Vec<crate::BondQueryOrder> {
    let tokens = value.unwrap_or("").split_whitespace().collect::<Vec<_>>();
    if tokens.len() < 2 {
        return Vec::new();
    }
    tokens
        .into_iter()
        .map(|token| match token {
            "1" => Some(crate::BondQueryOrder::Single),
            "1.5" => Some(crate::BondQueryOrder::Aromatic),
            "2" => Some(crate::BondQueryOrder::Double),
            "3" => Some(crate::BondQueryOrder::Triple),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()
        .unwrap_or_default()
}

fn cdxml_bond_topology(value: Option<&str>) -> crate::BondTopology {
    match value.unwrap_or("") {
        "Ring" => crate::BondTopology::Ring,
        "Chain" => crate::BondTopology::Chain,
        "RingOrChain" => crate::BondTopology::RingOrChain,
        _ => crate::BondTopology::Unspecified,
    }
}

fn cdxml_bond_reaction_participation(value: Option<&str>) -> crate::BondReactionParticipation {
    match value.unwrap_or("") {
        "ReactionCenter" => crate::BondReactionParticipation::ReactionCenter,
        "MakeOrBreak" => crate::BondReactionParticipation::MakeOrBreak,
        "ChangeType" => crate::BondReactionParticipation::ChangeType,
        "MakeAndChange" => crate::BondReactionParticipation::MakeAndChange,
        "NotReactionCenter" => crate::BondReactionParticipation::NotReactionCenter,
        "NoChange" => crate::BondReactionParticipation::NoChange,
        "Unmapped" => crate::BondReactionParticipation::Unmapped,
        _ => crate::BondReactionParticipation::Unspecified,
    }
}

fn cdxml_bond_absolute_stereo(value: Option<&str>) -> crate::BondAbsoluteStereo {
    match value.unwrap_or("") {
        "N" => crate::BondAbsoluteStereo::None,
        "E" => crate::BondAbsoluteStereo::E,
        "Z" => crate::BondAbsoluteStereo::Z,
        _ => crate::BondAbsoluteStereo::Unspecified,
    }
}

pub(super) fn cdxml_bond_line_styles(order: u8, display: &str, display2: &str) -> BondLineStyles {
    let mut styles = BondLineStyles::default();
    if matches!(display, "Dash" | "Hash") {
        styles.main = crate::BondLinePattern::Dashed;
        if order >= 2 {
            styles.left = crate::BondLinePattern::Dashed;
        }
    } else if display == "Wavy" {
        styles.main = crate::BondLinePattern::Wavy;
    }
    if order >= 2 && matches!(display2, "Dash" | "Hash") {
        styles.right = crate::BondLinePattern::Dashed;
    }
    styles
}

pub(super) fn cdxml_bond_line_weights(order: u8, display: &str, display2: &str) -> BondLineWeights {
    let mut weights = BondLineWeights::default();
    if display == "Bold" {
        weights.main = crate::BondLineWeight::Bold;
        if order >= 2 {
            weights.left = crate::BondLineWeight::Bold;
        }
    }
    if order >= 2 && display2 == "Bold" {
        weights.right = crate::BondLineWeight::Bold;
    }
    weights
}
