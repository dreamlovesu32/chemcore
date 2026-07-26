use super::*;

pub(in crate::cdxml) fn append_bracket_objects(
    root: &XmlNode,
    objects: &mut Vec<SceneObject>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
) {
    let mut brackets = Vec::new();
    let mut symbol_index = 1;
    let repeat_counts = bracket_repeat_counts_by_graphic_id(root);
    for node in descendants(root) {
        if !node.is("graphic") || node.attr("SupersededBy").is_some() {
            continue;
        }
        match node.attr("GraphicType").unwrap_or("") {
            "Bracket" => {
                let Some(bbox) = parse_ordered_bbox(node.attr("BoundingBox")) else {
                    continue;
                };
                let graphic_id = node.attr("id").map(ToString::to_string);
                let repeat_count = graphic_id
                    .as_deref()
                    .and_then(|id| repeat_counts.get(id).copied())
                    .or_else(|| bracket_usage_count(node));
                brackets.push(PendingCdxmlBracket {
                    kind: match node.attr("BracketType").unwrap_or("") {
                        "Square" => "square",
                        "Curly" => "curly",
                        _ => "round",
                    }
                    .to_string(),
                    bbox,
                    z_index: parse_i32(node.attr("Z")).unwrap_or(15),
                    graphic_id,
                    repeat_count,
                    stroke: colors.resolve(node.attr("color")),
                    stroke_width: parse_f64(node.attr("LineWidth")).unwrap_or(defaults.line_width),
                    lip_size: parse_i32(node.attr("LipSize"))
                        .and_then(|value| i16::try_from(value).ok())
                        .unwrap_or(60),
                });
            }
            "Symbol" => {
                let symbol_type = node.attr("SymbolType").unwrap_or("");
                let Some(kind) = cdxml_symbol_kind(symbol_type) else {
                    continue;
                };
                let Some(raw_bbox) = parse_ordered_bbox(node.attr("BoundingBox")) else {
                    continue;
                };
                let style = crate::cdxml_symbol_style_from_line_width(defaults.line_width);
                let metrics =
                    crate::cdxml_symbol_metrics_from_bbox(kind, raw_bbox, defaults.line_width);
                let (width, height) = (metrics.width, metrics.height);
                let [cx, cy] = cdxml_symbol_center(kind, raw_bbox);
                let fill = colors.resolve(node.attr("color"));
                let mut extra = BTreeMap::new();
                extra.insert("kind".to_string(), json!(kind));
                extra.insert("fill".to_string(), json!(fill));
                extra.insert(
                    "symbolStyle".to_string(),
                    json!(crate::cdxml_symbol_style_name(style)),
                );
                extra.insert(
                    "symbolAnchorWidth".to_string(),
                    json!(metrics.cdxml_anchor_width),
                );
                extra.insert(
                    "symbolAnchorHeight".to_string(),
                    json!(metrics.cdxml_anchor_height),
                );
                extra.insert("symbolLineWidth".to_string(), json!(metrics.line_width));
                extra.insert("cdxmlBoundingBox".to_string(), json!(raw_bbox));
                if let Some(stroke_width) = metrics.stroke_width {
                    extra.insert("strokeWidth".to_string(), json!(stroke_width));
                }
                if let Some(attribute) = node
                    .direct_children("represent")
                    .find_map(|represent| represent.attr("attribute"))
                {
                    extra.insert("representAttribute".to_string(), json!(attribute));
                }
                objects.push(SceneObject {
                    id: format!("obj_symbol_{symbol_index:03}"),
                    object_type: "symbol".to_string(),
                    name: format!("symbol {symbol_index}"),
                    visible: true,
                    locked: false,
                    z_index: parse_i32(node.attr("Z")).unwrap_or(15),
                    transform: Transform {
                        translate: [round2(cx - width * 0.5), round2(cy - height * 0.5)],
                        rotate: 0.0,
                        scale: [1.0, 1.0],
                    },
                    style_ref: None,
                    link_policy: Default::default(),
                    meta: json!({"source": "cdxml", "graphicId": node.attr("id")}),
                    payload: ObjectPayload {
                        resource_ref: None,
                        bbox: Some([0.0, 0.0, width, height]),
                        spectrum: None,
                        geometry: None,
                        constraint: None,
                        table: None,
                        stoichiometry_grid: None,
                        gel_electrophoresis: None,
                        extra,
                    },
                    children: Vec::new(),
                });
                symbol_index += 1;
            }
            _ => {}
        }
    }

    let mut used = vec![false; brackets.len()];
    let mut pairs = Vec::new();
    let bracket_indices_by_graphic_id: BTreeMap<_, _> = brackets
        .iter()
        .enumerate()
        .filter_map(|(index, bracket)| {
            bracket
                .graphic_id
                .as_ref()
                .map(|graphic_id| (graphic_id.as_str(), index))
        })
        .collect();
    for group in descendants(root)
        .into_iter()
        .filter(|node| node.is("bracketedgroup"))
    {
        let attachment_indices: Vec<_> = group
            .direct_children("bracketattachment")
            .filter_map(|attachment| attachment.attr("GraphicID"))
            .filter_map(|graphic_id| bracket_indices_by_graphic_id.get(graphic_id).copied())
            .collect();
        let [left_index, right_index] = attachment_indices.as_slice() else {
            continue;
        };
        if left_index == right_index || used[*left_index] || used[*right_index] {
            continue;
        }
        used[*left_index] = true;
        used[*right_index] = true;
        pairs.push((*left_index, *right_index));
    }

    for left_index in 0..brackets.len() {
        if used[left_index] {
            continue;
        }
        let left = &brackets[left_index];
        let left_bounds = normalized_bbox(left.bbox);
        let mut best_index = None;
        let mut best_dx = f64::INFINITY;
        for right_index in 0..brackets.len() {
            if left_index == right_index || used[right_index] {
                continue;
            }
            let right = &brackets[right_index];
            if right.kind != left.kind {
                continue;
            }
            let right_bounds = normalized_bbox(right.bbox);
            if (center_y(left_bounds) - center_y(right_bounds)).abs() > 2.0
                || (height_of(left_bounds) - height_of(right_bounds)).abs() > 2.0
            {
                continue;
            }
            let dx = (center_x(right_bounds) - center_x(left_bounds)).abs();
            if dx > crate::EPSILON && dx < best_dx {
                best_dx = dx;
                best_index = Some(right_index);
            }
        }
        let Some(right_index) = best_index else {
            continue;
        };
        used[left_index] = true;
        used[right_index] = true;
        pairs.push((left_index, right_index));
    }

    let mut object_index = 1;
    for (left_index, right_index) in pairs {
        let left = &brackets[left_index];
        let right = &brackets[right_index];
        let lb = normalized_bbox(left.bbox);
        let rb = normalized_bbox(right.bbox);
        let (left_bracket, right_bracket, left_bounds, right_bounds) =
            if center_x(lb) <= center_x(rb) {
                (left, right, lb, rb)
            } else {
                (right, left, rb, lb)
            };
        let min_x = lb[0].min(rb[0]);
        let min_y = lb[1].min(rb[1]);
        let max_x = lb[2].max(rb[2]);
        let max_y = lb[3].max(rb[3]);
        let pair_width = round2(max_x - min_x);
        let pair_height = round2(max_y - min_y);
        let group_id = format!("obj_bracket_{object_index:03}");
        let mut meta = json!({
            "source": "cdxml",
            "kind": "bracket-group",
            "graphicIds": [left_bracket.graphic_id.clone(), right_bracket.graphic_id.clone()],
        });
        if let Some(repeat_count) = left.repeat_count.or(right.repeat_count) {
            meta["repeatCount"] = json!(repeat_count);
        }
        let left_child = cdxml_bracket_side_scene_object(
            format!("{group_id}_left"),
            "left",
            left_bracket,
            left_bounds,
            min_x,
            pair_width,
        );
        let right_child = cdxml_bracket_side_scene_object(
            format!("{group_id}_right"),
            "right",
            right_bracket,
            right_bounds,
            min_x,
            pair_width,
        );
        objects.push(SceneObject {
            id: group_id,
            object_type: "group".to_string(),
            name: "bracket-group".to_string(),
            visible: true,
            locked: false,
            z_index: left.z_index.min(right.z_index),
            transform: Transform::identity(),
            style_ref: None,
            link_policy: Default::default(),
            meta,
            payload: ObjectPayload {
                resource_ref: None,
                bbox: Some([round2(min_x), round2(min_y), pair_width, pair_height]),
                spectrum: None,
                geometry: None,
                constraint: None,
                table: None,
                stoichiometry_grid: None,
                gel_electrophoresis: None,
                extra: BTreeMap::new(),
            },
            children: vec![left_child, right_child],
        });
        object_index += 1;
    }

    // A single CDXML bracket is a complete authored graphic, not necessarily
    // one side of a polymer pair.  Horizontal curly braces used as scheme
    // annotations are commonly stored as two ordered points with identical
    // y coordinates; retain that orientation and render one rotated side.
    for (index, bracket) in brackets.iter().enumerate() {
        if used[index] {
            continue;
        }
        let dx = bracket.bbox[2] - bracket.bbox[0];
        let dy = bracket.bbox[3] - bracket.bbox[1];
        if dx.abs() <= dy.abs() || dx.abs() <= crate::EPSILON {
            continue;
        }
        let length = dx.abs();
        let depth =
            cdxml_bracket_side_width(&bracket.kind, length, length).max(bracket.stroke_width);
        let min_x = bracket.bbox[0].min(bracket.bbox[2]);
        let y = bracket.bbox[1];
        let mut extra = BTreeMap::new();
        extra.insert("kind".to_string(), json!(bracket.kind.clone()));
        extra.insert("side".to_string(), json!("left"));
        extra.insert("stroke".to_string(), json!(bracket.stroke.clone()));
        extra.insert("strokeWidth".to_string(), json!(bracket.stroke_width));
        extra.insert("lipSize".to_string(), json!(bracket.lip_size));
        extra.insert("orientation".to_string(), json!("horizontal"));
        objects.push(SceneObject {
            id: format!("obj_bracket_{object_index:03}"),
            object_type: "bracket".to_string(),
            name: "standalone horizontal bracket".to_string(),
            visible: true,
            locked: false,
            z_index: bracket.z_index,
            transform: Transform {
                translate: [
                    round2(min_x + (length - depth) * 0.5),
                    round2(y - (length - depth) * 0.5),
                ],
                rotate: -90.0,
                scale: [1.0, 1.0],
            },
            style_ref: None,
            link_policy: Default::default(),
            meta: json!({
                "source": "cdxml",
                "graphicId": bracket.graphic_id.clone(),
                "standalone": true,
            }),
            payload: ObjectPayload {
                resource_ref: None,
                bbox: Some([0.0, 0.0, round2(depth), round2(length)]),
                spectrum: None,
                geometry: None,
                constraint: None,
                table: None,
                stoichiometry_grid: None,
                gel_electrophoresis: None,
                extra,
            },
            children: Vec::new(),
        });
        object_index += 1;
    }
}

pub(super) fn cdxml_bracket_side_scene_object(
    object_id: String,
    side: &str,
    bracket: &PendingCdxmlBracket,
    bounds: [f64; 4],
    pair_x: f64,
    pair_width: f64,
) -> SceneObject {
    let stroke_width = bracket.stroke_width;
    let side_height = height_of(bounds);
    let side_width = cdxml_bracket_side_width(&bracket.kind, pair_width, side_height)
        .max(stroke_width)
        .max(bounds[2] - bounds[0]);
    let translate_x = match side {
        "right" if bracket.kind == "round" => pair_x + pair_width,
        "right" => pair_x + pair_width - side_width,
        "left" if bracket.kind == "round" => pair_x - side_width,
        _ => pair_x,
    };
    let mut extra = BTreeMap::new();
    extra.insert("kind".to_string(), json!(bracket.kind.clone()));
    extra.insert("side".to_string(), json!(side));
    extra.insert("stroke".to_string(), json!(bracket.stroke.clone()));
    extra.insert("strokeWidth".to_string(), json!(stroke_width));
    extra.insert("lipSize".to_string(), json!(bracket.lip_size));
    SceneObject {
        id: object_id,
        object_type: "bracket".to_string(),
        name: format!("bracket-{side}"),
        visible: true,
        locked: false,
        z_index: bracket.z_index,
        transform: Transform {
            translate: [round2(translate_x), round2(bounds[1])],
            rotate: 0.0,
            scale: [1.0, 1.0],
        },
        style_ref: None,
        link_policy: Default::default(),
        meta: json!({
            "source": "cdxml",
            "graphicId": bracket.graphic_id.clone(),
            "bracketSide": side,
        }),
        payload: ObjectPayload {
            resource_ref: None,
            bbox: Some([0.0, 0.0, round2(side_width), round2(side_height)]),
            spectrum: None,
            geometry: None,
            constraint: None,
            table: None,
            stoichiometry_grid: None,
            gel_electrophoresis: None,
            extra,
        },
        children: Vec::new(),
    }
}

pub(super) fn cdxml_bracket_side_width(kind: &str, pair_width: f64, height: f64) -> f64 {
    match kind {
        "square" => (height * 0.07248).min(pair_width * 0.22).max(0.0),
        "curly" => (height * 0.14423).min(pair_width * 0.24).max(0.0),
        _ => (height * (1.0 - 3.0_f64.sqrt() * 0.5))
            .min(pair_width * 0.22)
            .max(0.0),
    }
}

pub(super) fn bracket_repeat_counts_by_graphic_id(root: &XmlNode) -> BTreeMap<String, u32> {
    let mut counts = BTreeMap::new();
    for node in descendants(root) {
        if !node.is("bracketedgroup") {
            continue;
        }
        let Some(count) = parse_u32(node.attr("RepeatCount")).filter(|value| *value >= 2) else {
            continue;
        };
        for attachment in node.direct_children("bracketattachment") {
            if let Some(graphic_id) = attachment.attr("GraphicID") {
                counts.insert(graphic_id.to_string(), count);
            }
        }
    }
    counts
}

pub(super) fn bracket_usage_count(node: &XmlNode) -> Option<u32> {
    node.direct_children("objecttag")
        .filter(|tag| tag.attr("Name") == Some("bracketusage"))
        .flat_map(descendants)
        .find_map(|tag_node| {
            tag_node
                .is("t")
                .then(|| tag_node.full_text().trim().parse::<u32>().ok())
                .flatten()
        })
        .filter(|value| *value >= 2)
}

pub(super) fn cdxml_symbol_center(_kind: &str, bbox: [f64; 4]) -> [f64; 2] {
    // For CDXML Symbol graphics, the first BoundingBox point is the symbol
    // center; the second point stores the ChemDraw anchor extent/direction.
    [bbox[0], bbox[1]]
}

pub(super) fn parse_ordered_bbox(value: Option<&str>) -> Option<[f64; 4]> {
    let mut parts = value?.split_whitespace();
    Some([
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    ])
}

pub(super) fn normalized_bbox(bbox: [f64; 4]) -> [f64; 4] {
    [
        bbox[0].min(bbox[2]),
        bbox[1].min(bbox[3]),
        bbox[0].max(bbox[2]),
        bbox[1].max(bbox[3]),
    ]
}

pub(super) fn center_x(bbox: [f64; 4]) -> f64 {
    (bbox[0] + bbox[2]) * 0.5
}

pub(super) fn center_y(bbox: [f64; 4]) -> f64 {
    (bbox[1] + bbox[3]) * 0.5
}

pub(super) fn height_of(bbox: [f64; 4]) -> f64 {
    bbox[3] - bbox[1]
}

pub(super) fn cdxml_symbol_kind(symbol_type: &str) -> Option<&'static str> {
    Some(match symbol_type {
        "DoubleDagger" => "double-dagger",
        "Dagger" => "dagger",
        "CirclePlus" => "circle-plus",
        "Plus" => "plus",
        "RadicalCation" => "radical-cation",
        "LonePair" => "lone-pair",
        "CircleMinus" => "circle-minus",
        "Minus" => "minus",
        "RadicalAnion" => "radical-anion",
        "Electron" => "electron",
        _ => return None,
    })
}
