use super::*;

#[allow(clippy::too_many_arguments)]
pub(in crate::cdxml) fn append_text_objects(
    root: &XmlNode,
    objects: &mut Vec<SceneObject>,
    styles: &mut BTreeMap<String, Value>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
    fonts: &BTreeMap<String, String>,
    display_fragment_ids: &BTreeSet<String>,
    bonded_node_ids: &BTreeSet<String>,
) {
    let mut index = 1;
    let node_positions: BTreeMap<String, [f64; 2]> = descendants(root)
        .into_iter()
        .filter(|node| node.is("n"))
        .filter_map(|node| Some((node.attr("id")?.to_string(), parse_xy(node.attr("p"))?)))
        .collect();
    let chemical_property_display_ids = descendants(root)
        .into_iter()
        .filter(|node| node.is("chemicalproperty"))
        .filter_map(|node| node.attr("ChemicalPropertyDisplayID"))
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>();
    let implicit_object_tag_positioning_is_absolute = root
        .attr("CreationProgram")
        .is_some_and(|program| program.starts_with("ChemDraw JS"));
    append_text_objects_recursive(
        root,
        false,
        false,
        true,
        false,
        false,
        None,
        0,
        None,
        CdxmlTextObjectRole::FreeText,
        None,
        None,
        None,
        None,
        None,
        false,
        implicit_object_tag_positioning_is_absolute,
        &node_positions,
        &chemical_property_display_ids,
        None,
        &mut index,
        objects,
        styles,
        defaults,
        colors,
        fonts,
        display_fragment_ids,
        bonded_node_ids,
    );
    objects
        .retain(|object| object.meta.get("role").and_then(Value::as_str) != Some("nmr_assignment"));
    let used_style_ids = objects
        .iter()
        .flat_map(super::import_chemical_properties::flatten_scene_object)
        .filter_map(|object| object.style_ref.as_deref())
        .collect::<BTreeSet<_>>();
    styles.retain(|style_id, _| {
        !is_generated_text_style_id(style_id) || used_style_ids.contains(style_id.as_str())
    });
}

fn is_generated_text_style_id(style_id: &str) -> bool {
    style_id.strip_prefix("style_text_").is_some_and(|suffix| {
        !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
    })
}

pub(in crate::cdxml) fn append_synthesized_enhanced_stereo_text_objects(
    root: &XmlNode,
    objects: &mut Vec<SceneObject>,
    styles: &mut BTreeMap<String, Value>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
    fonts: &BTreeMap<String, String>,
) {
    let node_positions: BTreeMap<&str, [f64; 2]> = descendants(root)
        .into_iter()
        .filter(|node| node.is("n"))
        .filter_map(|node| Some((node.attr("id")?, parse_xy(node.attr("p"))?)))
        .collect();
    let bonds = descendants(root)
        .into_iter()
        .filter(|node| node.is("b"))
        .collect::<Vec<_>>();
    let font_size = defaults.label_size * 0.75;
    let font_id = defaults.label_font.to_string();
    let font_family = fonts
        .get(&font_id)
        .cloned()
        .unwrap_or_else(|| "Arial".to_string());
    let fill = colors.resolve(Some(&defaults.color.to_string()));
    let mut index = objects
        .iter()
        .filter(|object| object.object_type == "text")
        .count()
        + 1;

    for node in descendants(root).into_iter().filter(|node| node.is("n")) {
        let Some(stereo_type) = node.attr("EnhancedStereoType") else {
            continue;
        };
        if node.direct_children("objecttag").any(|tag| {
            tag.attr("Name") == Some("enhancedstereo")
                && !tag
                    .attr("Visible")
                    .is_some_and(|value| value.eq_ignore_ascii_case("no"))
        }) {
            continue;
        }
        let Some(node_id) = node.attr("id") else {
            continue;
        };
        let Some(position) = node_positions.get(node_id).copied() else {
            continue;
        };
        let text = match stereo_type.to_ascii_lowercase().as_str() {
            "absolute" | "abs" => "abs".to_string(),
            "or" => format!("or{}", node.attr("EnhancedStereoGroupNum").unwrap_or("1")),
            "and" => format!("&{}", node.attr("EnhancedStereoGroupNum").unwrap_or("1")),
            _ => continue,
        };
        let direction = enhanced_stereo_label_direction(node_id, position, &bonds, &node_positions)
            .unwrap_or(Point::new(1.0, -1.0));
        let width = estimated_annotation_text_width(&text, font_size);
        let height = font_size * 0.86;
        let left = if direction.x >= 0.0 {
            position[0] + font_size * 0.48
        } else {
            position[0] - width - font_size * 0.5
        };
        let top = if direction.y < 0.0 {
            position[1] - font_size * 1.06
        } else {
            position[1] + font_size * 0.09
        };
        let style_id = format!("style_text_auto_enhanced_{index:03}");
        styles.insert(
            style_id.clone(),
            json!({
                "kind": "text",
                "fontFamily": font_family,
                "fontSize": font_size,
                "fontWeight": 400,
                "fill": fill,
                "stroke": null,
            }),
        );
        let mut extra = BTreeMap::new();
        extra.insert("text".to_string(), json!(text));
        extra.insert("box".to_string(), json!([0.0, 0.0, width, height]));
        extra.insert("align".to_string(), json!("left"));
        extra.insert("valign".to_string(), json!("top"));
        extra.insert("lineHeight".to_string(), json!(font_size * 1.15));
        extra.insert("fontSize".to_string(), json!(font_size));
        extra.insert("anchorOffsetX".to_string(), json!(0.0));
        extra.insert("baselineOffset".to_string(), json!(font_size * 0.84));
        extra.insert("preserveLines".to_string(), json!(true));
        extra.insert(
            "runs".to_string(),
            json!([LabelRun {
                text: text.clone(),
                font_family: Some(font_family.clone()),
                font_size: Some(font_size),
                fill: Some(fill.clone()),
                font_weight: Some(400),
                font_style: Some("normal".to_string()),
                underline: Some(false),
                outline: Some(false),
                shadow: Some(false),
                script: Some("normal".to_string()),
            }]),
        );
        objects.push(SceneObject {
            id: format!("obj_text_auto_enhanced_{index:03}"),
            object_type: "text".to_string(),
            name: format!("enhanced stereo label {node_id}"),
            visible: true,
            locked: false,
            z_index: parse_i32(node.attr("Z")).unwrap_or(30),
            transform: Transform {
                translate: [round2(left), round2(top)],
                rotate: 0.0,
                scale: [1.0, 1.0],
            },
            style_ref: Some(style_id),
            link_policy: Default::default(),
            meta: json!({
                "source": "cdxml",
                "role": "enhanced_stereo",
                "synthetic": true,
                "nodeId": node_id,
            }),
            payload: ObjectPayload {
                resource_ref: None,
                bbox: None,
                spectrum: None,
                geometry: None,
                constraint: None,
                table: None,
                stoichiometry_grid: None,
                gel_electrophoresis: None,
                plasmid_map: None,
                bio_shape: None,
                extra,
            },
            children: Vec::new(),
        });
        index += 1;
    }
}

pub(super) fn enhanced_stereo_label_direction(
    node_id: &str,
    position: [f64; 2],
    bonds: &[&XmlNode],
    node_positions: &BTreeMap<&str, [f64; 2]>,
) -> Option<Point> {
    let bond = bonds.iter().copied().find(|bond| {
        (bond.attr("B") == Some(node_id) || bond.attr("E") == Some(node_id))
            && bond
                .attr("Display")
                .is_some_and(|display| display.to_ascii_lowercase().contains("wedge"))
    })?;
    let other_id = if bond.attr("B") == Some(node_id) {
        bond.attr("E")?
    } else {
        bond.attr("B")?
    };
    let other = node_positions.get(other_id)?;
    let dx = position[0] - other[0];
    let dy = position[1] - other[1];
    let length = (dx * dx + dy * dy).sqrt();
    (length > crate::EPSILON).then(|| Point::new(dx / length, dy / length))
}

pub(super) fn estimated_annotation_text_width(text: &str, font_size: f64) -> f64 {
    text.chars()
        .map(|character| match character {
            '&' => 0.68,
            '0'..='9' => 0.5,
            'a'..='z' => 0.52,
            _ => 0.55,
        })
        .sum::<f64>()
        * font_size
}

#[allow(clippy::too_many_arguments)]
pub(in crate::cdxml) fn append_text_objects_recursive(
    node: &XmlNode,
    skip_text: bool,
    inside_native_annotation: bool,
    text_visible: bool,
    force_text_visible: bool,
    prefer_parameterized_bracket_label: bool,
    auto_bracket_label_right_x: Option<f64>,
    placeholder_depth: usize,
    inherited_z: Option<i32>,
    text_role: CdxmlTextObjectRole,
    containing_node_position: Option<[f64; 2]>,
    containing_node_id: Option<String>,
    containing_bond_id: Option<String>,
    containing_source_id: Option<String>,
    object_tag_owner_source_id: Option<String>,
    automatic_object_tag: bool,
    implicit_object_tag_positioning_is_absolute: bool,
    node_positions: &BTreeMap<String, [f64; 2]>,
    chemical_property_display_ids: &BTreeSet<String>,
    containing_bond_points: Option<([f64; 2], [f64; 2])>,
    index: &mut usize,
    objects: &mut Vec<SceneObject>,
    styles: &mut BTreeMap<String, Value>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
    fonts: &BTreeMap<String, String>,
    display_fragment_ids: &BTreeSet<String>,
    bonded_node_ids: &BTreeSet<String>,
) {
    let object_tag_role = node
        .is("objecttag")
        .then(|| CdxmlTextObjectRole::from_object_tag_name(node.attr("Name")))
        .flatten();
    let object_tag_uses_automatic_positioning = object_tag_role.is_some()
        && uses_automatic_object_tag_positioning(node)
        && !(implicit_object_tag_positioning_is_absolute && node.attr("PositioningType").is_none());
    let use_parameterized_bracket_label = object_tag_role
        == Some(CdxmlTextObjectRole::ParameterizedBracketLabel)
        && prefer_parameterized_bracket_label;
    let suppress_bracket_usage = object_tag_role == Some(CdxmlTextObjectRole::BracketUsage)
        && prefer_parameterized_bracket_label;
    let next_force_text_visible = if object_tag_role.is_some() {
        use_parameterized_bracket_label
    } else {
        force_text_visible
    };
    let next_text_visible = if use_parameterized_bracket_label {
        true
    } else if suppress_bracket_usage {
        false
    } else if object_tag_role.is_some() {
        !node
            .attr("Visible")
            .is_some_and(|value| value.eq_ignore_ascii_case("no"))
    } else if node.is("objecttag") {
        node.attr("Visible")
            .is_some_and(|value| value.eq_ignore_ascii_case("yes"))
    } else {
        text_visible
    };
    let next_skip_text = if object_tag_role.is_some() {
        false
    } else {
        skip_text
            || (node.is("fragment")
                && node
                    .attr("id")
                    .is_some_and(|id| display_fragment_ids.contains(id)))
            || (node.is("n")
                && node.attr("Element").is_some()
                && node
                    .attr("id")
                    .is_none_or(|id| bonded_node_ids.contains(id)))
    };
    let next_placeholder_depth = if node.is("n")
        && matches!(
            node.attr("NodeType").unwrap_or(""),
            "Fragment" | "Nickname" | "Unspecified"
        ) {
        1
    } else if placeholder_depth > 0 {
        placeholder_depth + 1
    } else {
        0
    };
    let next_text_role = object_tag_role.unwrap_or(text_role);
    let next_containing_node_position = if node.is("n") {
        parse_xy(node.attr("p")).or(containing_node_position)
    } else {
        containing_node_position
    };
    let next_containing_node_id = if node.is("n") {
        node.attr("id")
            .map(ToString::to_string)
            .or(containing_node_id)
    } else {
        containing_node_id
    };
    let next_containing_bond_id = if node.is("b") {
        node.attr("id")
            .map(ToString::to_string)
            .or(containing_bond_id)
    } else {
        containing_bond_id
    };
    let next_object_tag_owner_source_id = if object_tag_role.is_some() {
        containing_source_id.clone()
    } else {
        object_tag_owner_source_id
    };
    let next_containing_source_id = if node.is("objecttag") || node.is("t") || node.is("s") {
        containing_source_id
    } else {
        node.attr("id")
            .map(ToString::to_string)
            .or(containing_source_id)
    };
    let next_automatic_object_tag = if object_tag_role.is_some() {
        object_tag_uses_automatic_positioning
    } else {
        automatic_object_tag
    };
    let next_containing_bond_points = if node.is("b") {
        node.attr("B")
            .zip(node.attr("E"))
            .and_then(|(begin, end)| Some((*node_positions.get(begin)?, *node_positions.get(end)?)))
            .or(containing_bond_points)
    } else {
        containing_bond_points
    };
    let next_auto_bracket_label_right_x =
        if node.is("graphic") && node.attr("GraphicType") == Some("Bracket") {
            parse_bbox(node.attr("BoundingBox")).map(|bbox| bbox[0].max(bbox[2]))
        } else if object_tag_role.is_some() && !object_tag_uses_automatic_positioning {
            None
        } else {
            auto_bracket_label_right_x
        };
    let current_z = parse_i32(node.attr("Z")).or(inherited_z);
    if node.is("t") && !skip_text && !inside_native_annotation && placeholder_depth <= 1 {
        let is_chemical_property_display = node
            .attr("id")
            .is_some_and(|id| chemical_property_display_ids.contains(id));
        let visible = text_visible
            && (force_text_visible
                || !node
                    .attr("Visible")
                    .is_some_and(|value| value.eq_ignore_ascii_case("no")));
        if let Some(object) = text_object(
            node,
            *index,
            current_z.unwrap_or(30),
            next_text_role,
            next_containing_node_id.as_deref(),
            next_containing_bond_id.as_deref(),
            next_object_tag_owner_source_id.as_deref(),
            visible,
            auto_bracket_label_right_x,
            (next_text_role == CdxmlTextObjectRole::EnhancedStereo && next_automatic_object_tag)
                .then_some(next_containing_node_position)
                .flatten(),
            (next_text_role == CdxmlTextObjectRole::Query && next_automatic_object_tag)
                .then_some(next_containing_bond_points)
                .flatten(),
            styles,
            defaults,
            colors,
            fonts,
            is_chemical_property_display,
        ) {
            objects.push(object);
            *index += 1;
        }
    }
    for child in &node.children {
        append_text_objects_recursive(
            child,
            next_skip_text,
            inside_native_annotation
                || node.is("constraint")
                || (node.is("graphic") && node.attr("GraphicType") == Some("Symbol"))
                || (node.is("geometry") && node.attr("GeometricFeature").is_some()),
            next_text_visible,
            next_force_text_visible,
            if node.is("graphic") {
                node.direct_children("objecttag")
                    .any(|tag| tag.attr("Name") == Some("parameterizedBracketLabel"))
            } else {
                prefer_parameterized_bracket_label
            },
            next_auto_bracket_label_right_x,
            next_placeholder_depth,
            current_z,
            next_text_role,
            next_containing_node_position,
            next_containing_node_id.clone(),
            next_containing_bond_id.clone(),
            next_containing_source_id.clone(),
            next_object_tag_owner_source_id.clone(),
            next_automatic_object_tag,
            implicit_object_tag_positioning_is_absolute,
            node_positions,
            chemical_property_display_ids,
            next_containing_bond_points,
            index,
            objects,
            styles,
            defaults,
            colors,
            fonts,
            display_fragment_ids,
            bonded_node_ids,
        );
    }
}

pub(super) fn uses_automatic_object_tag_positioning(node: &XmlNode) -> bool {
    node.attr("PositioningType")
        .is_none_or(|value| value.eq_ignore_ascii_case("auto"))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn text_object(
    node: &XmlNode,
    index: usize,
    z_index: i32,
    role: CdxmlTextObjectRole,
    containing_node_id: Option<&str>,
    containing_bond_id: Option<&str>,
    object_tag_owner_source_id: Option<&str>,
    visible: bool,
    auto_bracket_label_right_x: Option<f64>,
    auto_enhanced_stereo_anchor: Option<[f64; 2]>,
    auto_query_bond_points: Option<([f64; 2], [f64; 2])>,
    styles: &mut BTreeMap<String, Value>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
    fonts: &BTreeMap<String, String>,
    allow_empty: bool,
) -> Option<SceneObject> {
    let text = node
        .attr("UTF8Text")
        .map(ToString::to_string)
        .unwrap_or_else(|| node.full_text())
        .trim()
        .to_string();
    if text.is_empty() && !allow_empty {
        return None;
    }
    let bbox = parse_bbox(node.attr("BoundingBox"));
    let point = parse_xy(node.attr("p")).or_else(|| bbox.map(|bbox| [bbox[0], bbox[1]]))?;
    let align = node
        .attr("CaptionJustification")
        .or_else(|| node.attr("Justification"))
        .unwrap_or(defaults.caption_justification.as_cdxml())
        .to_ascii_lowercase();
    let face = parse_u32(node.attr("face")).unwrap_or(defaults.caption_face);
    let color_id = node
        .attr("color")
        .or_else(|| node.direct_children("s").find_map(|run| run.attr("color")))
        .unwrap_or("0");
    let font_size = parse_f64(node.attr("size")).unwrap_or_else(|| {
        node.direct_children("s")
            .find_map(|run| parse_f64(run.attr("size")))
            .unwrap_or(defaults.caption_size)
    });
    let mut font_state = CdxmlFontRunState::default();
    let runs: Vec<LabelRun> = node
        .direct_children("s")
        .flat_map(|run| {
            let run_text = run.full_text();
            if run_text.is_empty() {
                Vec::new()
            } else {
                let font_id = font_state.resolve(run.attr("font"));
                label_display_runs(
                    &run_text,
                    parse_u32(run.attr("face")).unwrap_or(face),
                    font_id,
                    run.attr("color").unwrap_or(color_id),
                    parse_f64(run.attr("size")).unwrap_or(font_size),
                    colors,
                    fonts,
                )
            }
        })
        .collect();
    let font_family = runs
        .first()
        .and_then(|run| run.font_family.clone())
        .unwrap_or_else(|| "Arial".to_string());
    let style_id = format!("style_text_{index:03}");
    styles.entry(style_id.clone()).or_insert_with(|| {
        json!({
            "kind": "text",
            "fontFamily": font_family,
            "fontSize": font_size,
            "fontWeight": 400,
            "fill": colors.resolve(Some(color_id)),
            "stroke": null,
        })
    });
    let (text, runs, normalized_line_starts) =
        if node.attr("WordWrapWidth").is_some() || node.attr("LineStarts").is_some() {
            apply_cdxml_line_starts(
                &text,
                runs,
                node.attr("LineStarts"),
                node.attr("WordWrapWidth").is_some(),
            )
        } else {
            (text, runs, None)
        };
    let text_anchor = match align.as_str() {
        "center" => Some("middle"),
        "right" => Some("end"),
        _ => Some("start"),
    };
    let inferred_ink_bounds = crate::shared_text_horizontal_ink_bounds(
        &text,
        &runs,
        font_size,
        Some(&font_family),
        text_anchor,
    );
    let inferred_width = (inferred_ink_bounds[1] - inferred_ink_bounds[0]).max(0.0);
    let width = bbox
        .map(|bbox| (bbox[2] - bbox[0]).abs())
        .filter(|width| *width > crate::EPSILON)
        .unwrap_or_else(|| {
            if text.is_empty() {
                font_size
            } else {
                inferred_width
            }
        });
    let height = bbox
        .map(|bbox| (bbox[3] - bbox[1]).abs())
        .filter(|height| *height > crate::EPSILON)
        .unwrap_or_else(|| {
            crate::shared_estimated_text_max_font_size(round2(font_size), &runs) * 1.4
        });
    let auto_enhanced_stereo_placement = bbox.and_then(|bbox| {
        auto_enhanced_stereo_anchor.and_then(|anchor| {
            automatic_enhanced_stereo_text_placement(anchor, point, bbox, defaults.margin_width)
        })
    });
    let auto_query_placement = bbox.and_then(|bbox| {
        auto_query_bond_points
            .and_then(|points| automatic_query_bond_text_placement(points, bbox, font_size))
    });
    let automatic_placement = auto_enhanced_stereo_placement
        .map(|(translate, baseline_offset, _)| (translate, baseline_offset))
        .or_else(|| {
            auto_query_placement.map(|(translate, baseline_offset, _)| (translate, baseline_offset))
        });
    let translate = if let Some((translate, _)) = automatic_placement {
        translate
    } else if let Some(bbox) = bbox {
        let x = match align.as_str() {
            _ if role.is_bracket_label() => auto_bracket_label_right_x
                .map(|right_x| right_x + font_size * CHEMDRAW_AUTO_BRACKET_LABEL_GAP_EM)
                .unwrap_or(point[0]),
            "center" => (bbox[0] + bbox[2]) * 0.5,
            "right" => bbox[2],
            _ => bbox[0],
        };
        [round2(x), round2(bbox[1])]
    } else {
        [round2(point[0]), round2(point[1])]
    };
    let mut extra = BTreeMap::new();
    extra.insert("text".to_string(), json!(text));
    let box_x = bbox.map_or(inferred_ink_bounds[0], |bbox| bbox[0] - translate[0]);
    extra.insert(
        "box".to_string(),
        json!([round2(box_x), 0.0, round2(width), round2(height)]),
    );
    extra.insert("align".to_string(), json!(align));
    extra.insert("valign".to_string(), json!("top"));
    let line_spacing = cdxml_text_line_spacing(node, defaults, font_size, &runs);
    extra.insert(
        "lineHeight".to_string(),
        json!(round2(line_spacing.line_height)),
    );
    extra.insert("lineHeightMode".to_string(), json!(line_spacing.mode));
    if !line_spacing.line_advances.is_empty() {
        extra.insert(
            "lineAdvances".to_string(),
            json!(line_spacing
                .line_advances
                .iter()
                .copied()
                .map(round2)
                .collect::<Vec<_>>()),
        );
    }
    extra.insert("fontSize".to_string(), json!(round2(font_size)));
    if let Some((_, baseline_offset)) = automatic_placement {
        extra.insert("anchorOffsetX".to_string(), json!(0.0));
        extra.insert("baselineOffset".to_string(), json!(round2(baseline_offset)));
    } else if let Some(point) = parse_xy(node.attr("p")) {
        extra.insert(
            "anchorOffsetX".to_string(),
            json!(round2(point[0] - translate[0])),
        );
        extra.insert(
            "baselineOffset".to_string(),
            json!(round2(point[1] - translate[1])),
        );
    }
    if let Some(cached_vector) = auto_enhanced_stereo_placement
        .map(|(_, _, cached_vector)| cached_vector)
        .or_else(|| auto_query_placement.map(|(_, _, cached_vector)| cached_vector))
    {
        extra.insert(
            "automaticPositioningVector".to_string(),
            json!([round2(cached_vector[0]), round2(cached_vector[1])]),
        );
    }
    extra.insert("preserveLines".to_string(), json!(true));
    if !runs.is_empty() {
        extra.insert("runs".to_string(), serde_json::to_value(runs).ok()?);
    }
    Some(SceneObject {
        id: format!("obj_text_{index:03}"),
        object_type: "text".to_string(),
        name: format!("text {index}"),
        visible,
        locked: false,
        z_index,
        transform: Transform {
            translate,
            rotate: 0.0,
            scale: [1.0, 1.0],
        },
        style_ref: Some(style_id),
        link_policy: Default::default(),
        meta: json!({
            "source": "cdxml",
            "role": role.as_str(),
            "attachedNodeId": containing_node_id,
            "attachedBondId": containing_bond_id,
            "objectTagOwnerSourceId": object_tag_owner_source_id,
            "textId": node.attr("id"),
            "import": {
                "cdxml": {
                    "captionJustification": node.attr("CaptionJustification"),
                    "justification": node.attr("Justification"),
                    "lineHeight": node.attr("LineHeight"),
                    "captionLineHeight": node.attr("CaptionLineHeight"),
                    "wordWrapWidth": node.attr("WordWrapWidth"),
                    "lineStarts": normalized_line_starts,
                    "authoredBoundingBox": bbox.is_some(),
                }
            }
        }),
        payload: ObjectPayload {
            resource_ref: None,
            bbox: None,
            spectrum: None,
            geometry: None,
            constraint: None,
            table: None,
            stoichiometry_grid: None,
            gel_electrophoresis: None,
            plasmid_map: None,
            bio_shape: None,
            extra,
        },
        children: Vec::new(),
    })
}

pub(super) fn automatic_enhanced_stereo_text_placement(
    anchor: [f64; 2],
    point: [f64; 2],
    bbox: [f64; 4],
    margin_width: f64,
) -> Option<([f64; 2], f64, [f64; 2])> {
    let width = (bbox[2] - bbox[0]).abs();
    let height = (bbox[3] - bbox[1]).abs();
    if width <= crate::EPSILON || height <= crate::EPSILON {
        return None;
    }
    let cached_center = [(bbox[0] + bbox[2]) * 0.5, (bbox[1] + bbox[3]) * 0.5];
    let dx = cached_center[0] - anchor[0];
    let dy = cached_center[1] - anchor[1];
    let length = dx.hypot(dy);
    if length <= crate::EPSILON {
        return None;
    }
    let unit = [dx / length, dy / length];
    let center = [
        anchor[0] + unit[0] * (width * 0.5 + margin_width),
        anchor[1] + unit[1] * (height * 0.5 + margin_width),
    ];
    let translate = [
        round2(center[0] - width * 0.5),
        round2(center[1] - height * 0.5),
    ];
    Some((translate, round2(point[1] - bbox[1]), [dx, dy]))
}

pub(super) fn automatic_query_bond_text_placement(
    points: ([f64; 2], [f64; 2]),
    bbox: [f64; 4],
    font_size: f64,
) -> Option<([f64; 2], f64, [f64; 2])> {
    let width = (bbox[2] - bbox[0]).abs();
    let height = (bbox[3] - bbox[1]).abs();
    if width <= crate::EPSILON || height <= crate::EPSILON {
        return None;
    }
    let midpoint = [
        (points.0[0] + points.1[0]) * 0.5,
        (points.0[1] + points.1[1]) * 0.5,
    ];
    let cached_center = [(bbox[0] + bbox[2]) * 0.5, (bbox[1] + bbox[3]) * 0.5];
    let dx = cached_center[0] - midpoint[0];
    let dy = cached_center[1] - midpoint[1];
    let length = dx.hypot(dy);
    if length <= crate::EPSILON {
        return None;
    }
    let unit = [dx / length, dy / length];
    let translate = if unit[0] < -0.4 && unit[1] < -0.4 {
        [
            midpoint[0] - width - font_size * 0.18,
            midpoint[1] - height - font_size * 0.07,
        ]
    } else if unit[0] < -0.7 {
        [
            midpoint[0] - width - font_size * 0.47,
            midpoint[1] + font_size * 0.065,
        ]
    } else if unit[0] > 0.7 {
        [midpoint[0] + font_size * 0.317, midpoint[1] - height * 0.45]
    } else {
        return None;
    };
    Some((
        [round2(translate[0]), round2(translate[1])],
        height,
        [dx, dy],
    ))
}

pub(super) fn cdxml_text_line_spacing(
    node: &XmlNode,
    defaults: CdxmlDefaults,
    font_size: f64,
    runs: &[LabelRun],
) -> super::ResolvedCdxmlLineSpacing {
    let value = parse_cdxml_line_height(node.attr("CaptionLineHeight"))
        .or_else(|| parse_cdxml_line_height(node.attr("LineHeight")))
        .or(defaults.caption_line_height)
        .or(defaults.line_height)
        .unwrap_or(CdxmlLineHeight::Auto);
    match value {
        CdxmlLineHeight::Fixed(value) if value > 1.0 => super::ResolvedCdxmlLineSpacing {
            line_height: value,
            line_advances: Vec::new(),
            mode: "fixed",
        },
        CdxmlLineHeight::Variable => {
            let line_runs = super::split_label_runs_by_line(runs);
            let line_advances = crate::variable_text_line_advances(&line_runs, font_size);
            super::ResolvedCdxmlLineSpacing {
                line_height: line_advances
                    .first()
                    .copied()
                    .unwrap_or_else(|| crate::molecule_label_line_advance(font_size)),
                line_advances,
                mode: "variable",
            }
        }
        _ => super::ResolvedCdxmlLineSpacing {
            line_height: super::chemdraw_auto_text_line_height(font_size, runs),
            line_advances: Vec::new(),
            mode: "auto",
        },
    }
}
