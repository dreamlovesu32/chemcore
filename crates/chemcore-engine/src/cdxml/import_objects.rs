use super::*;
use crate::Point;

fn non_bond_dash_array(defaults: CdxmlDefaults) -> Value {
    json!([defaults
        .hash_spacing
        .max(crate::DEFAULT_HASH_SPACING_PT.value() * 0.25)])
}

pub(super) fn append_line_objects(
    root: &XmlNode,
    objects: &mut Vec<SceneObject>,
    styles: &mut BTreeMap<String, Value>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
) {
    let mut index = 1;
    for node in descendants(root) {
        if !(node.is("arrow") || (node.is("graphic") && node.attr("GraphicType") == Some("Line"))) {
            continue;
        }
        if node.attr("SupersededBy").is_some() {
            continue;
        }
        let head = parse_xyz2(node.attr("Head3D"));
        let tail = parse_xyz2(node.attr("Tail3D"));
        let (Some(tail), Some(head)) = (tail, head) else {
            continue;
        };
        let is_arrow = node.is("arrow") || has_arrow_attrs(node);
        let line_type = node.attr("LineType").unwrap_or("");
        let bold = line_type.contains("Bold");
        let stroke_width = if bold {
            defaults.bold_width
        } else {
            defaults.line_width
        };
        let head_enabled = arrow_endpoint_enabled(node.attr("ArrowheadHead"))
            || node
                .attr("ArrowType")
                .is_some_and(|value| value == "FullHead");
        let tail_enabled = arrow_endpoint_enabled(node.attr("ArrowheadTail"));
        let mut extra = BTreeMap::new();
        extra.insert("kind".to_string(), json!("line"));
        extra.insert(
            "points".to_string(),
            json!([
                [round2(tail[0]), round2(tail[1])],
                [round2(head[0]), round2(head[1])]
            ]),
        );
        if is_arrow {
            let mut arrow_head = BTreeMap::new();
            let cdxml_kind = cdxml_arrow_kind(node);
            arrow_head.insert("kind".to_string(), json!(cdxml_kind));
            arrow_head.insert(
                "head".to_string(),
                json!(canonical_arrow_endpoint(
                    node.attr("ArrowheadHead").unwrap_or(if head_enabled {
                        "Full"
                    } else {
                        "None"
                    }),
                )),
            );
            arrow_head.insert(
                "tail".to_string(),
                json!(canonical_arrow_endpoint(
                    node.attr("ArrowheadTail").unwrap_or(if tail_enabled {
                        "Full"
                    } else {
                        "None"
                    }),
                )),
            );
            if let Some(fill_type) = node.attr("FillType").map(canonical_arrow_fill_type) {
                arrow_head.insert("fillType".to_string(), json!(fill_type));
            }
            arrow_head.insert(
                "length".to_string(),
                json!(cdxml_arrow_size_for_render_scale(
                    parse_scaled_100(node.attr("HeadSize")),
                    crate::DEFAULT_ARROW_HEAD_LENGTH_RATIO,
                    defaults.line_width,
                    stroke_width,
                )),
            );
            arrow_head.insert(
                "centerLength".to_string(),
                json!(cdxml_arrow_size_for_render_scale(
                    parse_scaled_100(node.attr("ArrowheadCenterSize")).or_else(|| {
                        (!matches!(cdxml_kind, "equilibrium" | "unequal-equilibrium"))
                            .then(|| parse_scaled_100(node.attr("ArrowShaftSpacing")))
                            .flatten()
                    }),
                    crate::DEFAULT_ARROW_HEAD_LENGTH_RATIO * 0.875,
                    defaults.line_width,
                    stroke_width,
                )),
            );
            arrow_head.insert(
                "width".to_string(),
                json!(cdxml_arrow_size_for_render_scale(
                    parse_scaled_100(node.attr("ArrowheadWidth")),
                    crate::DEFAULT_ARROW_HEAD_LENGTH_RATIO * 0.25,
                    defaults.line_width,
                    stroke_width,
                )),
            );
            if matches!(cdxml_kind, "equilibrium" | "unequal-equilibrium") {
                arrow_head.insert(
                    "shaftSpacing".to_string(),
                    json!(cdxml_arrow_size_for_render_scale(
                        parse_scaled_100(node.attr("ArrowShaftSpacing")),
                        3.0,
                        defaults.line_width,
                        stroke_width,
                    )),
                );
                if let Some(ratio) = parse_scaled_100(node.attr("ArrowEquilibriumRatio"))
                    .filter(|value| *value > 1.0)
                {
                    arrow_head.insert("equilibriumRatio".to_string(), json!(ratio));
                }
            }
            if let Some(curve) =
                parse_f64(node.attr("AngularSize")).filter(|value| value.abs() > crate::EPSILON)
            {
                arrow_head.insert("curve".to_string(), json!(curve));
            }
            if let Some(no_go) = node
                .attr("NoGo")
                .map(str::trim)
                .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("None"))
            {
                arrow_head.insert("noGo".to_string(), json!(no_go.to_ascii_lowercase()));
            }
            if bold {
                arrow_head.insert("bold".to_string(), json!(true));
            }
            let mut arrow_geometry = BTreeMap::new();
            if let Some(bbox) = parse_bbox(node.attr("BoundingBox")) {
                arrow_geometry.insert(
                    "boundingBox".to_string(),
                    json!([
                        round2(bbox[0]),
                        round2(bbox[1]),
                        round2(bbox[2]),
                        round2(bbox[3])
                    ]),
                );
            }
            if let Some(center) = parse_xyz2(node.attr("Center3D")) {
                arrow_geometry.insert(
                    "center".to_string(),
                    json!([round2(center[0]), round2(center[1])]),
                );
            }
            if let Some(major) = parse_xyz2(node.attr("MajorAxisEnd3D")) {
                arrow_geometry.insert(
                    "majorAxisEnd".to_string(),
                    json!([round2(major[0]), round2(major[1])]),
                );
            }
            if let Some(minor) = parse_xyz2(node.attr("MinorAxisEnd3D")) {
                arrow_geometry.insert(
                    "minorAxisEnd".to_string(),
                    json!([round2(minor[0]), round2(minor[1])]),
                );
            }
            if !arrow_geometry.is_empty() {
                extra.insert("arrowGeometry".to_string(), json!(arrow_geometry));
            }
            extra.insert(
                "head".to_string(),
                json!(if head_enabled { "end" } else { "none" }),
            );
            extra.insert(
                "tail".to_string(),
                json!(if tail_enabled { "start" } else { "none" }),
            );
            extra.insert("arrowHead".to_string(), json!(arrow_head));
        }
        let style_ref = cdxml_line_style_ref(node, is_arrow, styles, defaults, colors);
        objects.push(SceneObject {
            id: format!("obj_line_{index:03}"),
            object_type: "line".to_string(),
            name: format!("line {index}"),
            visible: true,
            locked: false,
            z_index: parse_i32(node.attr("Z")).unwrap_or(if is_arrow { 20 } else { 18 }),
            transform: Transform::identity(),
            style_ref: Some(style_ref),
            meta: json!({"source": "cdxml", "graphicId": node.attr("id"), "import": {"cdxml": {"kind": if is_arrow { "arrow" } else { "line" }, "lineType": empty_as_null(node.attr("LineType"))}}}),
            payload: ObjectPayload {
                resource_ref: None,
                bbox: None,
                extra,
            },
            children: Vec::new(),
        });
        index += 1;
    }
}

fn cdxml_arrow_kind(node: &XmlNode) -> &'static str {
    let explicit_kind = node
        .attr("ArrowType")
        .or_else(|| node.attr("ArrowheadType"))
        .unwrap_or("Solid")
        .to_ascii_lowercase();
    if explicit_kind == "equilibrium" {
        return if parse_scaled_100(node.attr("ArrowEquilibriumRatio"))
            .is_some_and(|value| value > 1.0)
        {
            "unequal-equilibrium"
        } else {
            "equilibrium"
        };
    }
    let head = canonical_arrow_endpoint(node.attr("ArrowheadHead").unwrap_or("None"));
    let tail = canonical_arrow_endpoint(node.attr("ArrowheadTail").unwrap_or("None"));
    let has_equilibrium_spacing = node.attr("ArrowShaftSpacing").is_some();
    if explicit_kind == "solid"
        && has_equilibrium_spacing
        && head != "none"
        && head == tail
        && matches!(head, "half-left" | "half-right")
    {
        return if parse_scaled_100(node.attr("ArrowEquilibriumRatio"))
            .is_some_and(|value| value > 1.0)
        {
            "unequal-equilibrium"
        } else {
            "equilibrium"
        };
    }
    match explicit_kind.as_str() {
        "hollow" => "hollow",
        "angle" | "open" | "retrosynthetic" => "open",
        _ => "solid",
    }
}

fn canonical_arrow_endpoint(value: &str) -> &'static str {
    match value.to_ascii_lowercase().as_str() {
        "full" => "full",
        "halfleft" | "half-left" | "left" | "top" => "half-left",
        "halfright" | "half-right" | "right" | "bottom" => "half-right",
        _ => "none",
    }
}

fn canonical_arrow_fill_type(value: &str) -> &'static str {
    match value.to_ascii_lowercase().as_str() {
        "none" => "none",
        "solid" => "solid",
        "shaded" => "shaded",
        _ => "unknown",
    }
}

fn cdxml_arrow_size_for_render_scale(
    value: Option<f64>,
    fallback: f64,
    cdxml_line_width: f64,
    render_stroke_width: f64,
) -> f64 {
    let base = value.unwrap_or(fallback);
    if render_stroke_width.abs() <= crate::EPSILON {
        return base;
    }
    base * cdxml_line_width.max(crate::EPSILON) / render_stroke_width
}

fn cdxml_line_style_ref(
    node: &XmlNode,
    is_arrow: bool,
    styles: &mut BTreeMap<String, Value>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
) -> String {
    let line_type = node.attr("LineType").unwrap_or("");
    let bold = line_type.contains("Bold");
    let dashed = line_type.contains("Dashed");
    let color = colors.resolve(node.attr("color"));
    let base = if is_arrow { "arrow" } else { "line" };
    if !bold && !dashed && color == "#000000" {
        return format!("style_{base}_default");
    }
    let style_id = format!(
        "style_{base}_{}{}{}",
        if bold { "bold" } else { "regular" },
        if dashed { "_dashed" } else { "" },
        if color == "#000000" {
            String::new()
        } else {
            format!("_{}", color.trim_start_matches('#'))
        }
    );
    styles.entry(style_id.clone()).or_insert_with(|| {
        let line_cap = if dashed || is_arrow { "butt" } else { "round" };
        let line_join = if dashed || is_arrow { "miter" } else { "round" };
        json!({
            "kind": "stroke",
            "stroke": color,
            "strokeWidth": if bold { defaults.bold_width } else { defaults.line_width },
            "lineCap": line_cap,
            "lineJoin": line_join,
            "dashArray": if dashed { non_bond_dash_array(defaults) } else { json!([]) },
        })
    });
    style_id
}

pub(super) fn append_shape_objects(
    root: &XmlNode,
    objects: &mut Vec<SceneObject>,
    styles: &mut BTreeMap<String, Value>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
) {
    let mut index = 1;
    for node in descendants(root) {
        if !node.is("graphic") || node.attr("SupersededBy").is_some() {
            continue;
        }
        let graphic_type = node.attr("GraphicType").unwrap_or("");
        if !matches!(graphic_type, "Rectangle" | "Oval") {
            continue;
        }
        let Some(bbox) = parse_bbox(node.attr("BoundingBox")) else {
            continue;
        };
        let type_value = node
            .attr(if graphic_type == "Rectangle" {
                "RectangleType"
            } else {
                "OvalType"
            })
            .unwrap_or("");
        let color = colors.resolve(node.attr("color"));
        let filled = type_value.contains("Filled");
        let shaded = type_value.contains("Shaded");
        let shadow = type_value.contains("Shadow");
        let line_type = node.attr("LineType").unwrap_or("");
        let dashed = type_value.contains("Dashed") || line_type.contains("Dashed");
        let bold = line_type.contains("Bold");
        let stroke_width = parse_f64(node.attr("LineWidth")).unwrap_or(if bold {
            defaults.bold_width
        } else {
            defaults.line_width
        });
        let shadow_size = parse_scaled_100(node.attr("ShadowSize")).unwrap_or(4.0);
        let style_id = format!("style_shape_{index:03}");
        styles.insert(
            style_id.clone(),
            json!({
                "kind": "shape",
                "fill": if filled || shaded { json!(color) } else { Value::Null },
                "stroke": if filled { Value::Null } else { json!(color) },
                "strokeWidth": if filled { 0.0 } else { stroke_width },
                "dashArray": if dashed { non_bond_dash_array(defaults) } else { json!([]) },
                "shaded": if shaded { json!(true) } else { Value::Null },
                "shadow": if shadow { json!(true) } else { Value::Null },
                "shadowSize": if shadow { json!(shadow_size) } else { Value::Null },
            }),
        );
        let (transform, payload) = if graphic_type == "Oval" {
            let (Some(center), Some(major), Some(minor)) = (
                parse_xyz2(node.attr("Center3D")),
                parse_xyz2(node.attr("MajorAxisEnd3D")),
                parse_xyz2(node.attr("MinorAxisEnd3D")),
            ) else {
                continue;
            };
            let mut extra = BTreeMap::new();
            extra.insert(
                "kind".to_string(),
                json!(if type_value.contains("Circle") {
                    "circle"
                } else {
                    "ellipse"
                }),
            );
            extra.insert(
                "center".to_string(),
                json!([round2(center[0]), round2(center[1])]),
            );
            extra.insert(
                "majorAxisEnd".to_string(),
                json!([round2(major[0]), round2(major[1])]),
            );
            extra.insert(
                "minorAxisEnd".to_string(),
                json!([round2(minor[0]), round2(minor[1])]),
            );
            (
                Transform::identity(),
                ObjectPayload {
                    resource_ref: None,
                    bbox: Some([
                        round2(bbox[0]),
                        round2(bbox[1]),
                        round2(bbox[2] - bbox[0]),
                        round2(bbox[3] - bbox[1]),
                    ]),
                    extra,
                },
            )
        } else {
            let mut extra = BTreeMap::new();
            extra.insert(
                "kind".to_string(),
                json!(if type_value.contains("RoundEdge") {
                    "roundRect"
                } else {
                    "rect"
                }),
            );
            extra.insert(
                "cornerRadius".to_string(),
                json!(parse_scaled_100(node.attr("CornerRadius")).unwrap_or(0.0)),
            );
            (
                Transform {
                    translate: [round2(bbox[0]), round2(bbox[1])],
                    rotate: 0.0,
                    scale: [1.0, 1.0],
                },
                ObjectPayload {
                    resource_ref: None,
                    bbox: Some([
                        0.0,
                        0.0,
                        round2(bbox[2] - bbox[0]),
                        round2(bbox[3] - bbox[1]),
                    ]),
                    extra,
                },
            )
        };
        objects.push(SceneObject {
            id: format!("obj_shape_{index:03}"),
            object_type: "shape".to_string(),
            name: format!("shape {index}"),
            visible: true,
            locked: false,
            z_index: parse_i32(node.attr("Z")).unwrap_or(15),
            transform,
            style_ref: Some(style_id),
            meta: json!({"source": "cdxml", "graphicId": node.attr("id")}),
            payload,
            children: Vec::new(),
        });
        index += 1;
    }
}

pub(super) fn append_orbital_shape_objects(
    root: &XmlNode,
    objects: &mut Vec<SceneObject>,
    styles: &mut BTreeMap<String, Value>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
) {
    let mut index = 1;
    for node in descendants(root) {
        if !node.is("graphic")
            || node.attr("SupersededBy").is_some()
            || node.attr("GraphicType") != Some("Orbital")
        {
            continue;
        }
        let Some(orbital_type) = node.attr("OrbitalType") else {
            continue;
        };
        let Some((template, style, phase)) = cdxml_orbital_family(orbital_type) else {
            continue;
        };
        let color = colors.resolve(node.attr("color"));
        let style_id = format!("style_shape_orbital_{index:03}");
        styles.insert(
            style_id.clone(),
            json!({
                "kind": "shape",
                "fill": if style == "hollow" { Value::Null } else { json!(color.clone()) },
                "stroke": if style == "filled" { Value::Null } else { json!(color.clone()) },
                "strokeWidth": defaults.line_width,
                "dashArray": json!([]),
                "shaded": if style == "shaded" { json!(true) } else { Value::Null },
            }),
        );
        let mut extra = BTreeMap::new();
        extra.insert("kind".to_string(), json!("orbital"));
        extra.insert("orbitalTemplate".to_string(), json!(template));
        extra.insert("orbitalStyle".to_string(), json!(style));
        extra.insert("orbitalPhase".to_string(), json!(phase));
        extra.insert("orbitalColor".to_string(), json!(color.clone()));

        let (transform, payload_bbox) = if matches!(template, "s" | "oval") {
            let (Some(center), Some(major), Some(minor)) = (
                parse_xyz2(node.attr("Center3D")),
                parse_xyz2(node.attr("MajorAxisEnd3D")),
                parse_xyz2(node.attr("MinorAxisEnd3D")),
            ) else {
                continue;
            };
            extra.insert(
                "center".to_string(),
                json!([round2(center[0]), round2(center[1])]),
            );
            extra.insert(
                "majorAxisEnd".to_string(),
                json!([round2(major[0]), round2(major[1])]),
            );
            extra.insert(
                "minorAxisEnd".to_string(),
                json!([round2(minor[0]), round2(minor[1])]),
            );
            let rx = Point::new(center[0], center[1]).distance(Point::new(major[0], major[1]));
            let ry = Point::new(center[0], center[1]).distance(Point::new(minor[0], minor[1]));
            let bbox = [
                center[0] - rx,
                center[1] - ry,
                center[0] + rx,
                center[1] + ry,
            ];
            (
                Transform::identity(),
                Some([
                    round2(bbox[0].min(bbox[2])),
                    round2(bbox[1].min(bbox[3])),
                    round2((bbox[2] - bbox[0]).abs()),
                    round2((bbox[3] - bbox[1]).abs()),
                ]),
            )
        } else {
            let Some((anchor, tip)) = parse_orbital_axis_points(node.attr("BoundingBox")) else {
                continue;
            };
            extra.insert(
                "axisStart".to_string(),
                json!([round2(anchor[0]), round2(anchor[1])]),
            );
            extra.insert(
                "axisEnd".to_string(),
                json!([round2(tip[0]), round2(tip[1])]),
            );
            let padding = ((Point::new(anchor[0], anchor[1]).distance(Point::new(tip[0], tip[1]))
                * 0.75)
                .max(defaults.bond_length * 0.25))
            .max(6.0);
            let min_x = anchor[0].min(tip[0]) - padding;
            let min_y = anchor[1].min(tip[1]) - padding;
            let max_x = anchor[0].max(tip[0]) + padding;
            let max_y = anchor[1].max(tip[1]) + padding;
            (
                Transform::identity(),
                Some([
                    round2(min_x),
                    round2(min_y),
                    round2(max_x - min_x),
                    round2(max_y - min_y),
                ]),
            )
        };

        objects.push(SceneObject {
            id: format!("obj_shape_orbital_{index:03}"),
            object_type: "shape".to_string(),
            name: format!("orbital {index}"),
            visible: true,
            locked: false,
            z_index: parse_i32(node.attr("Z")).unwrap_or(15),
            transform,
            style_ref: Some(style_id),
            meta: json!({"source": "cdxml", "graphicId": node.attr("id"), "orbitalType": orbital_type}),
            payload: ObjectPayload {
                resource_ref: None,
                bbox: payload_bbox,
                extra,
            },
            children: Vec::new(),
        });
        index += 1;
    }
}

fn parse_orbital_axis_points(value: Option<&str>) -> Option<([f64; 2], [f64; 2])> {
    let nums: Vec<f64> = value?
        .split_whitespace()
        .take(4)
        .filter_map(|part| part.parse().ok())
        .collect();
    if nums.len() != 4 {
        return None;
    }
    Some(([nums[2], nums[3]], [nums[0], nums[1]]))
}

fn cdxml_orbital_family(value: &str) -> Option<(&'static str, &'static str, &'static str)> {
    match value {
        "s" => Some(("s", "hollow", "plus")),
        "sShaded" => Some(("s", "shaded", "plus")),
        "sFilled" => Some(("s", "filled", "plus")),
        "p" => Some(("p", "shaded", "plus")),
        "pFilled" => Some(("p", "filled", "plus")),
        "dxy" => Some(("dxy", "shaded", "plus")),
        "dxyFilled" => Some(("dxy", "filled", "plus")),
        "oval" => Some(("oval", "hollow", "plus")),
        "ovalShaded" => Some(("oval", "shaded", "plus")),
        "ovalFilled" => Some(("oval", "filled", "plus")),
        "hybridMinus" => Some(("hybrid", "shaded", "minus")),
        "hybridMinusFilled" => Some(("hybrid", "filled", "minus")),
        "hybridPlus" => Some(("hybrid", "shaded", "plus")),
        "hybridPlusFilled" => Some(("hybrid", "filled", "plus")),
        "dz2Minus" => Some(("dz2", "shaded", "minus")),
        "dz2MinusFilled" => Some(("dz2", "filled", "minus")),
        "dz2Plus" => Some(("dz2", "shaded", "plus")),
        "dz2PlusFilled" => Some(("dz2", "filled", "plus")),
        "lobe" => Some(("lobe", "hollow", "plus")),
        "lobeShaded" => Some(("lobe", "shaded", "plus")),
        "lobeFilled" => Some(("lobe", "filled", "plus")),
        _ => None,
    }
}

pub(super) fn append_table_shape_objects(
    root: &XmlNode,
    objects: &mut Vec<SceneObject>,
    styles: &mut BTreeMap<String, Value>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
) {
    let mut index = 1;
    for node in descendants(root) {
        if !node.is("table") || node.attr("SupersededBy").is_some() {
            continue;
        }
        let Some(bbox) = parse_bbox(node.attr("BoundingBox")) else {
            continue;
        };
        let color = colors.resolve(node.attr("color"));
        let style_id = format!("style_shape_table_{index:03}");
        styles.insert(
            style_id.clone(),
            json!({
                "kind": "shape",
                "fill": Value::Null,
                "stroke": color,
                "strokeWidth": defaults.line_width,
                "dashArray": json!([]),
            }),
        );
        let mut extra = BTreeMap::new();
        extra.insert("kind".to_string(), json!("crossTable"));
        objects.push(SceneObject {
            id: format!("obj_shape_table_{index:03}"),
            object_type: "shape".to_string(),
            name: format!("table shape {index}"),
            visible: true,
            locked: false,
            z_index: parse_i32(node.attr("Z")).unwrap_or(15),
            transform: Transform {
                translate: [round2(bbox[0]), round2(bbox[1])],
                rotate: 0.0,
                scale: [1.0, 1.0],
            },
            style_ref: Some(style_id),
            meta: json!({"source": "cdxml", "tableId": node.attr("id")}),
            payload: ObjectPayload {
                resource_ref: None,
                bbox: Some([
                    0.0,
                    0.0,
                    round2(bbox[2] - bbox[0]),
                    round2(bbox[3] - bbox[1]),
                ]),
                extra,
            },
            children: Vec::new(),
        });
        index += 1;
    }
}

pub(super) fn append_tlc_plate_shape_objects(
    root: &XmlNode,
    objects: &mut Vec<SceneObject>,
    styles: &mut BTreeMap<String, Value>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
) {
    let mut index = 1;
    for node in descendants(root) {
        if !node.is("tlcplate") || node.attr("SupersededBy").is_some() {
            continue;
        }
        let corners = [
            parse_xyz2(node.attr("TopLeft")),
            parse_xyz2(node.attr("TopRight")),
            parse_xyz2(node.attr("BottomRight")),
            parse_xyz2(node.attr("BottomLeft")),
        ];
        let plate_bbox = corners
            .iter()
            .flatten()
            .fold(None, |acc: Option<[f64; 4]>, point| {
                Some(match acc {
                    Some([left, top, right, bottom]) => [
                        left.min(point[0]),
                        top.min(point[1]),
                        right.max(point[0]),
                        bottom.max(point[1]),
                    ],
                    None => [point[0], point[1], point[0], point[1]],
                })
            })
            .or_else(|| parse_bbox(node.attr("BoundingBox")));
        let Some(bbox) = plate_bbox else {
            continue;
        };
        let color = colors.resolve(node.attr("color"));
        let style_id = format!("style_shape_tlc_{index:03}");
        styles.insert(
            style_id.clone(),
            json!({
                "kind": "shape",
                "fill": "#ffffff",
                "stroke": color,
                "strokeWidth": defaults.line_width,
                "dashArray": json!([]),
            }),
        );
        let lanes_xml: Vec<_> = node
            .children
            .iter()
            .filter(|child| child.is("tlclane"))
            .collect();
        let lane_count = lanes_xml.len().max(1);
        let lanes: Vec<_> = lanes_xml
            .iter()
            .enumerate()
            .map(|(lane_index, lane)| {
                let spots: Vec<_> = lane
                    .children
                    .iter()
                    .filter(|child| child.is("tlcspot"))
                    .map(|spot| {
                        let mut json_spot = serde_json::Map::new();
                        json_spot.insert(
                            "rf".to_string(),
                            json!(round2(parse_f64(spot.attr("Rf")).unwrap_or(0.15))),
                        );
                        if let Some(width) = parse_f64(spot.attr("Width")) {
                            json_spot.insert(
                                "width".to_string(),
                                json!(round2(normalize_tlc_spot_extent(width))),
                            );
                        }
                        if let Some(height) = parse_f64(spot.attr("Height")) {
                            json_spot.insert(
                                "height".to_string(),
                                json!(round2(normalize_tlc_spot_extent(height))),
                            );
                        }
                        if let Some(curve_type) = parse_i32(spot.attr("CurveType")) {
                            json_spot.insert("curveType".to_string(), json!(curve_type));
                        }
                        if let Some(tail) = parse_f64(spot.attr("Tail")) {
                            json_spot.insert("tail".to_string(), json!(tail));
                        }
                        Value::Object(json_spot)
                    })
                    .collect();
                json!({
                    "offset": round2((lane_index as f64 + 1.0) / (lane_count as f64 + 1.0)),
                    "spots": spots,
                })
            })
            .collect();
        let mut extra = BTreeMap::new();
        extra.insert("kind".to_string(), json!("tlcPlate"));
        extra.insert(
            "originFraction".to_string(),
            json!(round2(
                parse_f64(node.attr("OriginFraction")).unwrap_or(0.1)
            )),
        );
        extra.insert(
            "solventFrontFraction".to_string(),
            json!(round2(
                parse_f64(node.attr("SolventFrontFraction")).unwrap_or(0.1)
            )),
        );
        extra.insert(
            "showOrigin".to_string(),
            json!(node
                .attr("ShowOrigin")
                .is_none_or(|value| value.eq_ignore_ascii_case("yes"))),
        );
        extra.insert(
            "showSolventFront".to_string(),
            json!(node
                .attr("ShowSolventFront")
                .is_none_or(|value| value.eq_ignore_ascii_case("yes"))),
        );
        extra.insert(
            "showBorders".to_string(),
            json!(node
                .attr("ShowBorders")
                .is_none_or(|value| value.eq_ignore_ascii_case("yes"))),
        );
        extra.insert(
            "showSideTicks".to_string(),
            json!(node
                .attr("ShowSideTicks")
                .is_none_or(|value| value.eq_ignore_ascii_case("yes"))),
        );
        extra.insert(
            "dashSpacing".to_string(),
            json!(round2(defaults.hash_spacing)),
        );
        extra.insert("lanes".to_string(), json!(lanes));
        objects.push(SceneObject {
            id: format!("obj_shape_tlc_{index:03}"),
            object_type: "shape".to_string(),
            name: format!("tlc plate {index}"),
            visible: true,
            locked: false,
            z_index: parse_i32(node.attr("Z")).unwrap_or(15),
            transform: Transform {
                translate: [round2(bbox[0]), round2(bbox[1])],
                rotate: 0.0,
                scale: [1.0, 1.0],
            },
            style_ref: Some(style_id),
            meta: json!({"source": "cdxml", "tlcPlateId": node.attr("id")}),
            payload: ObjectPayload {
                resource_ref: None,
                bbox: Some([
                    0.0,
                    0.0,
                    round2(bbox[2] - bbox[0]),
                    round2(bbox[3] - bbox[1]),
                ]),
                extra,
            },
            children: Vec::new(),
        });
        index += 1;
    }
}

fn normalize_tlc_spot_extent(raw: f64) -> f64 {
    if raw.abs() > 1024.0 {
        raw / 65536.0
    } else {
        raw
    }
}

#[derive(Clone)]
struct PendingCdxmlBracket {
    kind: String,
    bbox: [f64; 4],
    z_index: i32,
    graphic_id: Option<String>,
    stroke: String,
}

pub(super) fn append_bracket_objects(
    root: &XmlNode,
    objects: &mut Vec<SceneObject>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
) {
    let mut brackets = Vec::new();
    let mut symbol_index = 1;
    for node in descendants(root) {
        if !node.is("graphic") || node.attr("SupersededBy").is_some() {
            continue;
        }
        match node.attr("GraphicType").unwrap_or("") {
            "Bracket" => {
                let Some(bbox) = parse_bbox(node.attr("BoundingBox")) else {
                    continue;
                };
                brackets.push(PendingCdxmlBracket {
                    kind: match node.attr("BracketType").unwrap_or("") {
                        "Square" => "square",
                        "Curly" => "curly",
                        _ => "round",
                    }
                    .to_string(),
                    bbox,
                    z_index: parse_i32(node.attr("Z")).unwrap_or(15),
                    graphic_id: node.attr("id").map(ToString::to_string),
                    stroke: colors.resolve(node.attr("color")),
                });
            }
            "Symbol" => {
                let symbol_type = node.attr("SymbolType").unwrap_or("");
                let Some(kind) = cdxml_symbol_kind(symbol_type) else {
                    continue;
                };
                let Some(raw_bbox) = parse_bbox(node.attr("BoundingBox")) else {
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
                    meta: json!({"source": "cdxml", "graphicId": node.attr("id")}),
                    payload: ObjectPayload {
                        resource_ref: None,
                        bbox: Some([0.0, 0.0, width, height]),
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
    let mut object_index = 1;
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
        let right = &brackets[right_index];
        let lb = normalized_bbox(left.bbox);
        let rb = normalized_bbox(right.bbox);
        let min_x = lb[0].min(rb[0]);
        let min_y = lb[1].min(rb[1]);
        let max_x = lb[2].max(rb[2]);
        let max_y = lb[3].max(rb[3]);
        let mut extra = BTreeMap::new();
        extra.insert("kind".to_string(), json!(left.kind));
        extra.insert("stroke".to_string(), json!(left.stroke.clone()));
        extra.insert("strokeWidth".to_string(), json!(1.0));
        extra.insert("lipSize".to_string(), json!(60));
        objects.push(SceneObject {
            id: format!("obj_bracket_{object_index:03}"),
            object_type: "bracket".to_string(),
            name: format!("bracket {object_index}"),
            visible: true,
            locked: false,
            z_index: left.z_index.min(right.z_index),
            transform: Transform {
                translate: [round2(min_x), round2(min_y)],
                rotate: 0.0,
                scale: [1.0, 1.0],
            },
            style_ref: None,
            meta: json!({
                "source": "cdxml",
                "graphicIds": [left.graphic_id, right.graphic_id],
            }),
            payload: ObjectPayload {
                resource_ref: None,
                bbox: Some([0.0, 0.0, round2(max_x - min_x), round2(max_y - min_y)]),
                extra,
            },
            children: Vec::new(),
        });
        object_index += 1;
    }
}

fn cdxml_symbol_center(kind: &str, bbox: [f64; 4]) -> [f64; 2] {
    let center_x = (bbox[0] + bbox[2]) * 0.5;
    let center_y = if kind == "electron" {
        // ChemDraw treats the top endpoint of an electron symbol's vertical anchor
        // as the dot center; the rest of the bbox stores the anchor height.
        bbox[1]
    } else {
        (bbox[1] + bbox[3]) * 0.5
    };
    [center_x, center_y]
}

fn normalized_bbox(bbox: [f64; 4]) -> [f64; 4] {
    [
        bbox[0].min(bbox[2]),
        bbox[1].min(bbox[3]),
        bbox[0].max(bbox[2]),
        bbox[1].max(bbox[3]),
    ]
}

fn center_x(bbox: [f64; 4]) -> f64 {
    (bbox[0] + bbox[2]) * 0.5
}

fn center_y(bbox: [f64; 4]) -> f64 {
    (bbox[1] + bbox[3]) * 0.5
}

fn height_of(bbox: [f64; 4]) -> f64 {
    bbox[3] - bbox[1]
}

fn cdxml_symbol_kind(symbol_type: &str) -> Option<&'static str> {
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

pub(super) fn append_text_objects(
    root: &XmlNode,
    objects: &mut Vec<SceneObject>,
    styles: &mut BTreeMap<String, Value>,
    colors: &CdxmlColorTable,
    fonts: &BTreeMap<String, String>,
    display_fragment_ids: &BTreeSet<String>,
    bonded_node_ids: &BTreeSet<String>,
) {
    let mut index = 1;
    append_text_objects_recursive(
        root,
        false,
        0,
        None,
        &mut index,
        objects,
        styles,
        colors,
        fonts,
        display_fragment_ids,
        bonded_node_ids,
    );
}

pub(super) fn append_text_objects_recursive(
    node: &XmlNode,
    skip_text: bool,
    placeholder_depth: usize,
    inherited_z: Option<i32>,
    index: &mut usize,
    objects: &mut Vec<SceneObject>,
    styles: &mut BTreeMap<String, Value>,
    colors: &CdxmlColorTable,
    fonts: &BTreeMap<String, String>,
    display_fragment_ids: &BTreeSet<String>,
    bonded_node_ids: &BTreeSet<String>,
) {
    let next_skip_text = skip_text
        || (node.is("objecttag") && node.attr("Name") == Some("bracketusage"))
        || (node.is("fragment")
            && node
                .attr("id")
                .is_some_and(|id| display_fragment_ids.contains(id)))
        || (node.is("n")
            && node.attr("Element").is_some()
            && node
                .attr("id")
                .map_or(true, |id| bonded_node_ids.contains(id)));
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
    let current_z = parse_i32(node.attr("Z")).or(inherited_z);
    if node.is("t") && !skip_text && placeholder_depth <= 1 {
        if let Some(object) =
            text_object(node, *index, current_z.unwrap_or(30), styles, colors, fonts)
        {
            objects.push(object);
            *index += 1;
        }
    }
    for child in &node.children {
        append_text_objects_recursive(
            child,
            next_skip_text,
            next_placeholder_depth,
            current_z,
            index,
            objects,
            styles,
            colors,
            fonts,
            display_fragment_ids,
            bonded_node_ids,
        );
    }
}

fn text_object(
    node: &XmlNode,
    index: usize,
    z_index: i32,
    styles: &mut BTreeMap<String, Value>,
    colors: &CdxmlColorTable,
    fonts: &BTreeMap<String, String>,
) -> Option<SceneObject> {
    let text = node
        .attr("UTF8Text")
        .map(ToString::to_string)
        .unwrap_or_else(|| node.full_text())
        .trim()
        .to_string();
    if text.is_empty() {
        return None;
    }
    let bbox = parse_bbox(node.attr("BoundingBox"));
    let point = parse_xy(node.attr("p")).or_else(|| bbox.map(|bbox| [bbox[0], bbox[1]]))?;
    let align = node
        .attr("Justification")
        .or_else(|| node.attr("LabelJustification"))
        .unwrap_or("Left")
        .to_ascii_lowercase();
    let font_id = node
        .attr("font")
        .or_else(|| node.direct_children("s").find_map(|run| run.attr("font")))
        .unwrap_or("3");
    let color_id = node
        .attr("color")
        .or_else(|| node.direct_children("s").find_map(|run| run.attr("color")))
        .unwrap_or("0");
    let font_size = parse_f64(node.attr("size")).unwrap_or_else(|| {
        node.direct_children("s")
            .find_map(|run| parse_f64(run.attr("size")))
            .unwrap_or(10.0)
    });
    let style_id = format!("style_text_{index:03}");
    styles.entry(style_id.clone()).or_insert_with(|| {
        json!({
            "kind": "text",
            "fontFamily": fonts.get(font_id).cloned().unwrap_or_else(|| "Arial".to_string()),
            "fontSize": font_size,
            "fontWeight": 400,
            "fill": colors.resolve(Some(color_id)),
            "stroke": null,
        })
    });
    let runs: Vec<LabelRun> = node
        .direct_children("s")
        .flat_map(|run| {
            let run_text = run.full_text();
            if run_text.is_empty() {
                Vec::new()
            } else {
                label_display_runs(
                    &run_text,
                    parse_u32(run.attr("face")).unwrap_or(0),
                    run.attr("font").unwrap_or(font_id),
                    run.attr("color").unwrap_or(color_id),
                    parse_f64(run.attr("size")).unwrap_or(font_size),
                    colors,
                    fonts,
                )
            }
        })
        .collect();
    let width = bbox
        .map(|bbox| (bbox[2] - bbox[0]).abs())
        .filter(|width| *width > crate::EPSILON)
        .unwrap_or_else(|| (text.chars().count() as f64 * font_size * 0.55).max(font_size));
    let height = bbox
        .map(|bbox| (bbox[3] - bbox[1]).abs())
        .filter(|height| *height > crate::EPSILON)
        .unwrap_or(font_size * 1.4);
    let translate = if let Some(bbox) = bbox {
        let x = match align.as_str() {
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
    let box_x = match align.as_str() {
        "center" => -width * 0.5,
        "right" => -width,
        _ => 0.0,
    };
    extra.insert(
        "box".to_string(),
        json!([round2(box_x), 0.0, round2(width), round2(height)]),
    );
    extra.insert("align".to_string(), json!(align));
    extra.insert("valign".to_string(), json!("top"));
    extra.insert(
        "lineHeight".to_string(),
        json!(round2(cdxml_text_line_height(node, font_size))),
    );
    extra.insert("fontSize".to_string(), json!(round2(font_size)));
    if let Some(point) = parse_xy(node.attr("p")) {
        extra.insert(
            "baselineOffset".to_string(),
            json!(round2(point[1] - translate[1])),
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
        visible: true,
        locked: false,
        z_index,
        transform: Transform {
            translate,
            rotate: 0.0,
            scale: [1.0, 1.0],
        },
        style_ref: Some(style_id),
        meta: json!({"source": "cdxml", "role": "free_text", "textId": node.attr("id")}),
        payload: ObjectPayload {
            resource_ref: None,
            bbox: None,
            extra,
        },
        children: Vec::new(),
    })
}

fn cdxml_text_line_height(node: &XmlNode, font_size: f64) -> f64 {
    match node.attr("LineHeight").map(str::trim) {
        Some(value) if !value.eq_ignore_ascii_case("auto") => {
            parse_f64(Some(value)).unwrap_or(font_size * 1.2)
        }
        Some(_) => cdxml_auto_text_line_height(node, font_size),
        None => font_size * 1.2,
    }
}

fn cdxml_auto_text_line_height(node: &XmlNode, font_size: f64) -> f64 {
    let mut has_bold = false;
    let mut has_manual_subscript = false;
    let mut has_manual_superscript = false;

    for run in node.direct_children("s") {
        let face = parse_u32(run.attr("face")).unwrap_or(0);
        has_bold |= face & 1 != 0;
        let has_subscript = face & 32 != 0;
        let has_superscript = face & 64 != 0;
        if has_subscript && !has_superscript {
            has_manual_subscript = true;
        }
        if has_superscript && !has_subscript {
            has_manual_superscript = true;
        }
    }

    let ratio = if has_manual_superscript {
        if has_bold {
            1.445
        } else {
            1.415
        }
    } else if has_manual_subscript {
        if has_bold {
            1.345
        } else {
            1.315
        }
    } else if has_bold {
        1.175
    } else {
        1.15
    };
    font_size * ratio
}
