use super::*;

pub(super) fn non_bond_dash_array(defaults: CdxmlDefaults) -> Value {
    json!([defaults
        .hash_spacing
        .max(crate::DEFAULT_HASH_SPACING_PT.value() * 0.25)])
}

pub(in crate::cdxml) fn append_curve_objects(
    root: &XmlNode,
    objects: &mut Vec<SceneObject>,
    styles: &mut BTreeMap<String, Value>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
) {
    let mut index = 1;
    for node in descendants(root) {
        if !node.is("curve") || node.attr("SupersededBy").is_some() {
            continue;
        }
        let Some(points) = parse_cdxml_curve_points(node.attr("CurvePoints")) else {
            continue;
        };
        let curve_type = parse_i32(node.attr("CurveType")).unwrap_or(0);
        let dashed = curve_type & 0x0002 != 0
            || node
                .attr("LineType")
                .is_some_and(|value| value.contains("Dashed"));
        let bold = curve_type & 0x0004 != 0
            || node
                .attr("LineType")
                .is_some_and(|value| value.contains("Bold"));
        let stroke = colors.resolve(node.attr("color"));
        let stroke_width = parse_f64(node.attr("LineWidth")).unwrap_or(if bold {
            defaults.bold_width
        } else {
            defaults.line_width
        });
        let style_id = format!("style_curve_{index:03}");
        styles.insert(
            style_id.clone(),
            json!({
                "kind": "stroke",
                "stroke": stroke,
                "strokeWidth": stroke_width,
                "lineCap": "butt",
                "lineJoin": "round",
                "dashArray": if dashed { non_bond_dash_array(defaults) } else { json!([]) },
            }),
        );

        let mut extra = BTreeMap::new();
        extra.insert("kind".to_string(), json!("bezier-curve"));
        extra.insert("curvePoints".to_string(), json!(points));
        extra.insert("curveType".to_string(), json!(curve_type));
        extra.insert("closed".to_string(), json!(curve_type & 0x0001 != 0));
        let explicit_head = node.attr("ArrowheadHead").map(canonical_arrow_endpoint);
        let explicit_tail = node.attr("ArrowheadTail").map(canonical_arrow_endpoint);
        extra.insert(
            "head".to_string(),
            json!(explicit_head.unwrap_or(if curve_type & 0x0008 != 0 {
                "full"
            } else if curve_type & 0x0020 != 0 {
                "half"
            } else {
                "none"
            })),
        );
        extra.insert(
            "tail".to_string(),
            json!(explicit_tail.unwrap_or(if curve_type & 0x0010 != 0 {
                "full"
            } else if curve_type & 0x0040 != 0 {
                "half"
            } else {
                "none"
            })),
        );
        extra.insert(
            "arrowheadType".to_string(),
            json!(node.attr("ArrowheadType").unwrap_or("Solid")),
        );
        extra.insert(
            "headLength".to_string(),
            json!(cdxml_arrow_size_for_render_scale(
                parse_scaled_100(node.attr("HeadSize")),
                crate::DEFAULT_ARROW_HEAD_LENGTH_RATIO,
            )),
        );
        extra.insert(
            "headCenterLength".to_string(),
            json!(cdxml_arrow_size_for_render_scale(
                parse_scaled_100(node.attr("ArrowheadCenterSize")),
                crate::DEFAULT_ARROW_HEAD_LENGTH_RATIO * 0.875,
            )),
        );
        extra.insert(
            "headWidth".to_string(),
            json!(cdxml_arrow_size_for_render_scale(
                parse_scaled_100(node.attr("ArrowheadWidth")),
                crate::DEFAULT_ARROW_HEAD_LENGTH_RATIO * 0.25,
            )),
        );
        let (min_x, min_y, max_x, max_y) = points.iter().fold(
            (
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ),
            |(min_x, min_y, max_x, max_y), point| {
                (
                    min_x.min(point[0]),
                    min_y.min(point[1]),
                    max_x.max(point[0]),
                    max_y.max(point[1]),
                )
            },
        );
        objects.push(SceneObject {
            id: format!("obj_curve_{index:03}"),
            object_type: "curve".to_string(),
            name: format!("curve {index}"),
            visible: true,
            locked: false,
            z_index: parse_i32(node.attr("Z")).unwrap_or(18),
            transform: Transform::identity(),
            style_ref: Some(style_id),
            link_policy: Default::default(),
            meta: json!({
                "source": "cdxml",
                "curveId": node.attr("id"),
                "curveFingerprint": node.attr("CurvePoints"),
            }),
            payload: ObjectPayload {
                resource_ref: None,
                bbox: Some([min_x, min_y, max_x - min_x, max_y - min_y]),
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

pub(in crate::cdxml) fn parse_cdxml_curve_points(value: Option<&str>) -> Option<Vec<[f64; 2]>> {
    let values: Vec<_> = value?
        .split_whitespace()
        .filter_map(|value| value.parse::<f64>().ok())
        .collect();
    if values.len() < 12 || values.len() % 2 != 0 {
        return None;
    }
    let points: Vec<_> = values
        .chunks_exact(2)
        .map(|pair| [pair[0], pair[1]])
        .collect();
    // ChemDraw stores one endpoint guide on each side of the drawable
    // spline.  The body is points[1..len-1]: P0 followed by C1, C2, P for
    // every cubic segment.  The two outer guides determine endpoint tangent
    // and arrow geometry but are not part of the stroked path.
    ((points.len() - 3) % 3 == 0).then_some(points)
}

pub(in crate::cdxml) fn append_line_objects(
    root: &XmlNode,
    objects: &mut Vec<SceneObject>,
    styles: &mut BTreeMap<String, Value>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
) {
    let mut index = 1;
    for node in descendants(root) {
        if !(node.is("arrow")
            || (node.is("graphic") && matches!(node.attr("GraphicType"), Some("Line" | "Arc"))))
        {
            continue;
        }
        if node.attr("SupersededBy").is_some() {
            continue;
        }
        // CDXML permits plain line graphics (and legacy line-shaped arrows) to
        // store their two endpoints only in BoundingBox. For these objects the
        // first pair is the head and the second pair is the tail; unlike a
        // rectangular extent, their authored order must be preserved.
        let legacy_arc = cdxml_legacy_arc_geometry(node);
        let bbox_endpoints = cdxml_line_bbox_endpoints(node.attr("BoundingBox"));
        let head = parse_xyz2(node.attr("Head3D"))
            .or_else(|| legacy_arc.map(|geometry| geometry.head))
            .or_else(|| bbox_endpoints.map(|points| points.0));
        let tail = parse_xyz2(node.attr("Tail3D"))
            .or_else(|| legacy_arc.map(|geometry| geometry.tail))
            .or_else(|| bbox_endpoints.map(|points| points.1));
        let (Some(tail), Some(head)) = (tail, head) else {
            continue;
        };
        let is_arrow = node.is("arrow") || has_arrow_attrs(node);
        let line_type = node.attr("LineType").unwrap_or("");
        let bold = line_type.contains("Bold");
        let head_endpoint = cdxml_arrow_endpoint(node, true);
        let tail_endpoint = cdxml_arrow_endpoint(node, false);
        let head_enabled = head_endpoint != "none";
        let tail_enabled = tail_endpoint != "none";
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
            arrow_head.insert("head".to_string(), json!(head_endpoint));
            arrow_head.insert("tail".to_string(), json!(tail_endpoint));
            if let Some(fill_type) = node.attr("FillType").map(canonical_arrow_fill_type) {
                arrow_head.insert("fillType".to_string(), json!(fill_type));
            }
            arrow_head.insert(
                "length".to_string(),
                json!(cdxml_arrow_size_for_render_scale(
                    parse_scaled_100(node.attr("HeadSize")),
                    crate::DEFAULT_ARROW_HEAD_LENGTH_RATIO
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
                    crate::DEFAULT_ARROW_HEAD_LENGTH_RATIO * 0.875
                )),
            );
            arrow_head.insert(
                "width".to_string(),
                json!(cdxml_arrow_size_for_render_scale(
                    parse_scaled_100(node.attr("ArrowheadWidth")),
                    crate::DEFAULT_ARROW_HEAD_LENGTH_RATIO * 0.25
                )),
            );
            if !matches!(cdxml_kind, "equilibrium" | "unequal-equilibrium") {
                if let Some(shaft_spacing) = parse_scaled_100(node.attr("ArrowShaftSpacing")) {
                    arrow_head.insert("shaftSpacing".to_string(), json!(shaft_spacing));
                }
            }
            if matches!(cdxml_kind, "equilibrium" | "unequal-equilibrium") {
                arrow_head.insert(
                    "shaftSpacing".to_string(),
                    json!(cdxml_arrow_size_for_render_scale(
                        parse_scaled_100(node.attr("ArrowShaftSpacing")),
                        3.0
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
            } else if legacy_arrow_type_has(node, "NoGo") {
                arrow_head.insert("noGo".to_string(), json!("cross"));
            }
            if node.attr("Dipole").is_some_and(cdxml_boolean_enabled)
                || legacy_arrow_type_has(node, "Dipole")
            {
                arrow_head.insert("dipole".to_string(), json!(true));
            }
            if let Some(curve_spacing) = parse_scaled_100(node.attr("CurveSpacing")) {
                arrow_head.insert("curveSpacing".to_string(), json!(curve_spacing));
            }
            if node.attr("Closed").is_some_and(cdxml_boolean_enabled) {
                arrow_head.insert("closed".to_string(), json!(true));
            }
            if let Some(source) = node.attr("ArrowSource") {
                arrow_head.insert("source".to_string(), json!(source));
            }
            if let Some(target) = node.attr("ArrowTarget") {
                arrow_head.insert("target".to_string(), json!(target));
            }
            if bold {
                arrow_head.insert("bold".to_string(), json!(true));
            }
            let mut arrow_geometry = BTreeMap::new();
            if legacy_arc.is_none() {
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
            }
            if let Some(center) = parse_xyz2(node.attr("Center3D")) {
                arrow_geometry.insert(
                    "center".to_string(),
                    json!([round2(center[0]), round2(center[1])]),
                );
            } else if let Some(geometry) = legacy_arc {
                arrow_geometry.insert(
                    "center".to_string(),
                    json!([round2(geometry.center[0]), round2(geometry.center[1])]),
                );
                arrow_geometry.insert(
                    "majorAxisEnd".to_string(),
                    json!([
                        round2(geometry.center[0] + geometry.radius),
                        round2(geometry.center[1])
                    ]),
                );
                arrow_geometry.insert(
                    "minorAxisEnd".to_string(),
                    json!([
                        round2(geometry.center[0]),
                        round2(geometry.center[1] + geometry.radius)
                    ]),
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
            link_policy: Default::default(),
            meta: json!({"source": "cdxml", "graphicId": node.attr("id"), "import": {"cdxml": {"kind": if is_arrow { "arrow" } else { "line" }, "lineType": empty_as_null(node.attr("LineType"))}}}),
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

pub(super) fn cdxml_line_bbox_endpoints(value: Option<&str>) -> Option<([f64; 2], [f64; 2])> {
    let mut parts = value?.split_whitespace();
    let head = [parts.next()?.parse().ok()?, parts.next()?.parse().ok()?];
    let tail = [parts.next()?.parse().ok()?, parts.next()?.parse().ok()?];
    Some((head, tail))
}

pub(super) fn cdxml_legacy_arc_geometry(node: &XmlNode) -> Option<LegacyArcGeometry> {
    if !node.is("graphic") || node.attr("GraphicType") != Some("Arc") {
        return None;
    }
    // For legacy Arc graphics ChemDraw stores the authored head followed by
    // the circle center in BoundingBox. AngularSize is the signed sweep from
    // the head to the tail (screen coordinates, positive clockwise).
    let (head, center) = cdxml_line_bbox_endpoints(node.attr("BoundingBox"))?;
    let sweep = parse_f64(node.attr("AngularSize"))?.to_radians();
    let delta_x = head[0] - center[0];
    let delta_y = head[1] - center[1];
    let radius = delta_x.hypot(delta_y);
    if radius <= crate::EPSILON {
        return None;
    }
    let tail = [
        center[0] + delta_x * sweep.cos() - delta_y * sweep.sin(),
        center[1] + delta_x * sweep.sin() + delta_y * sweep.cos(),
    ];
    Some(LegacyArcGeometry {
        head,
        tail,
        center,
        radius,
    })
}

pub(super) fn cdxml_arrow_kind(node: &XmlNode) -> &'static str {
    if !has_modern_arrow_fields(node) {
        if legacy_arrow_type_has(node, "Equilibrium") {
            return if parse_scaled_100(node.attr("ArrowEquilibriumRatio"))
                .is_some_and(|value| value > 1.0)
            {
                "unequal-equilibrium"
            } else {
                "equilibrium"
            };
        }
        if legacy_arrow_type_has(node, "Hollow") {
            return "hollow";
        }
        if legacy_arrow_type_has(node, "RetroSynthetic") {
            return "open";
        }
    }
    let explicit_kind = node
        .attr("ArrowheadType")
        .or_else(|| node.attr("ArrowType"))
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

pub(super) fn legacy_arrow_type_has(node: &XmlNode, expected: &str) -> bool {
    node.attr("ArrowType").is_some_and(|value| {
        value
            .split_whitespace()
            .any(|token| token.eq_ignore_ascii_case(expected))
    })
}

pub(super) fn has_modern_arrow_fields(node: &XmlNode) -> bool {
    ["ArrowheadHead", "ArrowheadTail", "ArrowheadType"]
        .into_iter()
        .any(|name| node.attr(name).is_some())
}

pub(super) fn cdxml_boolean_enabled(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "yes" | "true" | "1"
    )
}

pub(super) fn cdxml_arrow_endpoint(node: &XmlNode, at_head: bool) -> &'static str {
    let modern_name = if at_head {
        "ArrowheadHead"
    } else {
        "ArrowheadTail"
    };
    if let Some(value) = node.attr(modern_name) {
        return canonical_arrow_endpoint(value);
    }
    if has_modern_arrow_fields(node) {
        return "none";
    }

    if legacy_arrow_type_has(node, "Equilibrium") {
        return "half-left";
    }
    if legacy_arrow_type_has(node, "Resonance") {
        return "full";
    }
    if !at_head {
        return "none";
    }
    if legacy_arrow_type_has(node, "HalfHead") {
        return if parse_f64(node.attr("HeadSize")).is_some_and(|value| value < 0.0) {
            "half-right"
        } else {
            "half-left"
        };
    }
    if ["FullHead", "Hollow", "RetroSynthetic", "Dipole"]
        .into_iter()
        .any(|kind| legacy_arrow_type_has(node, kind))
    {
        return "full";
    }
    "none"
}

pub(super) fn canonical_arrow_endpoint(value: &str) -> &'static str {
    match value.to_ascii_lowercase().as_str() {
        "full" | "fullhead" => "full",
        "halfleft" | "half-left" | "left" | "top" => "half-left",
        "halfright" | "half-right" | "right" | "bottom" => "half-right",
        _ => "none",
    }
}

pub(super) fn canonical_arrow_fill_type(value: &str) -> &'static str {
    match value.to_ascii_lowercase().as_str() {
        "none" => "none",
        "solid" => "solid",
        "shaded" => "shaded",
        _ => "unknown",
    }
}

pub(super) fn cdxml_arrow_size_for_render_scale(value: Option<f64>, default_value: f64) -> f64 {
    value.unwrap_or(default_value)
}

pub(super) fn cdxml_line_style_ref(
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
    let stroke_width = parse_f64(node.attr("LineWidth")).unwrap_or(if bold {
        defaults.bold_width
    } else {
        defaults.line_width
    });
    let default_stroke_width = if bold {
        defaults.bold_width
    } else {
        defaults.line_width
    };
    let base = if is_arrow { "arrow" } else { "line" };
    let default_width = (stroke_width - default_stroke_width).abs() <= crate::EPSILON;
    if !bold && !dashed && color == "#000000" && default_width {
        return format!("style_{base}_default");
    }
    let style_id = format!(
        "style_{base}_{}{}{}{}",
        if bold { "bold" } else { "regular" },
        if dashed { "_dashed" } else { "" },
        if default_width {
            String::new()
        } else {
            format!("_w{}", cdxml_style_number(stroke_width))
        },
        if color == "#000000" {
            String::new()
        } else {
            format!("_{}", color.trim_start_matches('#'))
        }
    );
    styles.entry(style_id.clone()).or_insert_with(|| {
        // ChemDraw emits CDXML GraphicType="Line" shafts as square-ended
        // geometry, just like arrow shafts.  This is independent of whether
        // LineType contains Dashed or Bold; round caps are a native editor
        // default and must not be introduced at the CDXML import boundary.
        let line_cap = "butt";
        let line_join = "miter";
        json!({
            "kind": "stroke",
            "stroke": color,
            "strokeWidth": stroke_width,
            "lineCap": line_cap,
            "lineJoin": line_join,
            "dashArray": if dashed { non_bond_dash_array(defaults) } else { json!([]) },
        })
    });
    style_id
}

pub(super) fn cdxml_style_number(value: f64) -> String {
    let mut text = format!("{:.3}", value.abs());
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    let text = text.replace('.', "p");
    if value < 0.0 {
        format!("m{text}")
    } else {
        text
    }
}
