use super::*;

pub(in crate::cdxml) fn append_shape_objects(
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
        let Some(raw_bbox) = parse_ordered_bbox(node.attr("BoundingBox")) else {
            continue;
        };
        let bbox = [
            raw_bbox[0].min(raw_bbox[2]),
            raw_bbox[1].min(raw_bbox[3]),
            raw_bbox[0].max(raw_bbox[2]),
            raw_bbox[1].max(raw_bbox[3]),
        ];
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
        let bold = type_value.contains("Bold") || line_type.contains("Bold");
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
            let axes = match (
                parse_xyz2(node.attr("Center3D")),
                parse_xyz2(node.attr("MajorAxisEnd3D")),
                parse_xyz2(node.attr("MinorAxisEnd3D")),
            ) {
                (Some(center), Some(major), Some(minor)) => Some((center, major, minor)),
                // Older CDX circle graphics use the two ordered BoundingBox
                // points as a radial endpoint followed by the center.  The
                // official BoundingBox documentation explicitly says Graphic
                // objects overload the rectangle as a pair of defining points.
                // Preserve that representation when the later 3D-axis fields
                // are absent.
                _ if type_value.contains("Circle") => {
                    let center = [raw_bbox[2], raw_bbox[3]];
                    let major = [raw_bbox[0], raw_bbox[1]];
                    let dx = major[0] - center[0];
                    let dy = major[1] - center[1];
                    (dx.hypot(dy) > crate::EPSILON).then_some((
                        center,
                        major,
                        [center[0] - dy, center[1] + dx],
                    ))
                }
                _ => None,
            };
            let Some((center, major, minor)) = axes else {
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
                    spectrum: None,
                    geometry: None,
                    constraint: None,
                    table: None,
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
                    spectrum: None,
                    geometry: None,
                    constraint: None,
                    table: None,
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
            link_policy: Default::default(),
            meta: json!({"source": "cdxml", "graphicId": node.attr("id")}),
            payload,
            children: Vec::new(),
        });
        index += 1;
    }
}

pub(in crate::cdxml) fn append_orbital_shape_objects(
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
            link_policy: Default::default(),
            meta: json!({"source": "cdxml", "graphicId": node.attr("id"), "orbitalType": orbital_type}),
            payload: ObjectPayload {
                resource_ref: None,
                bbox: payload_bbox,
                spectrum: None,
                geometry: None,
                constraint: None,
                table: None,
                extra,
            },
            children: Vec::new(),
        });
        index += 1;
    }
}

pub(super) fn parse_orbital_axis_points(value: Option<&str>) -> Option<([f64; 2], [f64; 2])> {
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

pub(super) fn cdxml_orbital_family(
    value: &str,
) -> Option<(&'static str, &'static str, &'static str)> {
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

pub(in crate::cdxml) fn append_table_shape_objects(
    root: &XmlNode,
    objects: &mut Vec<SceneObject>,
    _styles: &mut BTreeMap<String, Value>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
) {
    let mut index = 1;
    for node in descendants(root) {
        if !node.is("table") || node.attr("SupersededBy").is_some() {
            continue;
        }
        let pages = node.direct_children("page").collect::<Vec<_>>();
        if pages.is_empty() {
            continue;
        }
        let page_bounds = pages
            .iter()
            .filter_map(|page| {
                parse_ordered_bbox(
                    page.attr("BoundsInParent")
                        .or_else(|| page.attr("BoundingBox")),
                )
            })
            .map(|bounds| {
                [
                    bounds[0].min(bounds[2]),
                    bounds[1].min(bounds[3]),
                    bounds[0].max(bounds[2]),
                    bounds[1].max(bounds[3]),
                ]
            })
            .collect::<Vec<_>>();
        if page_bounds.len() != pages.len() {
            continue;
        }
        let mut column_guides = page_bounds
            .iter()
            .flat_map(|bounds| [bounds[0], bounds[2]])
            .collect::<Vec<_>>();
        let mut row_guides = page_bounds
            .iter()
            .flat_map(|bounds| [bounds[1], bounds[3]])
            .collect::<Vec<_>>();
        column_guides.sort_by(f64::total_cmp);
        row_guides.sort_by(f64::total_cmp);
        column_guides.dedup_by(|left, right| (*left - *right).abs() <= crate::EPSILON);
        row_guides.dedup_by(|left, right| (*left - *right).abs() <= crate::EPSILON);
        if column_guides.len() < 2 || row_guides.len() < 2 {
            continue;
        }
        let left = column_guides[0];
        let top = row_guides[0];
        let right = *column_guides.last().unwrap_or(&left);
        let bottom = *row_guides.last().unwrap_or(&top);
        let default_border = crate::TableBorder {
            visible: parse_f64(node.attr("LineWidth")).unwrap_or(defaults.line_width)
                > crate::EPSILON,
            line_style: table_line_style(node.attr("LineType")),
            width: parse_f64(node.attr("LineWidth")).unwrap_or(defaults.line_width),
            color: colors.resolve(node.attr("color")),
        };
        let rows = row_guides.len() - 1;
        let columns = column_guides.len() - 1;
        let cells = pages
            .iter()
            .zip(page_bounds.iter())
            .filter_map(|(page, bounds)| {
                let row = row_guides
                    .iter()
                    .position(|guide| (*guide - bounds[1]).abs() <= crate::EPSILON)?;
                let column = column_guides
                    .iter()
                    .position(|guide| (*guide - bounds[0]).abs() <= crate::EPSILON)?;
                let mut borders = crate::TableCellBorders::default();
                for border_node in page.direct_children("border") {
                    let width =
                        parse_f64(border_node.attr("LineWidth")).unwrap_or(default_border.width);
                    let border = crate::TableBorder {
                        visible: width > crate::EPSILON,
                        line_style: table_line_style(border_node.attr("LineType")),
                        width,
                        color: colors
                            .resolve(border_node.attr("color").or_else(|| node.attr("color"))),
                    };
                    match border_node.attr("Side").unwrap_or("undefined") {
                        "top" => borders.top = Some(border),
                        "left" => borders.left = Some(border),
                        "bottom" => borders.bottom = Some(border),
                        "right" => borders.right = Some(border),
                        _ => {}
                    }
                }
                Some(crate::TableCell {
                    id: page
                        .attr("id")
                        .map(|id| format!("cell_cdxml_{id}"))
                        .unwrap_or_else(|| format!("cell_{index}_{row}_{column}")),
                    row,
                    column,
                    content_object_ids: Vec::new(),
                    borders,
                    horizontal_alignment: Default::default(),
                    vertical_alignment: Default::default(),
                })
            })
            .collect::<Vec<_>>();
        if cells.len() != rows * columns {
            continue;
        }
        objects.push(SceneObject {
            id: format!("obj_table_{index:03}"),
            object_type: "table".to_string(),
            name: format!("table {index}"),
            visible: true,
            locked: false,
            z_index: parse_i32(node.attr("Z")).unwrap_or(15),
            transform: Transform {
                translate: [round2(left), round2(top)],
                rotate: 0.0,
                scale: [1.0, 1.0],
            },
            style_ref: None,
            link_policy: Default::default(),
            meta: json!({"source": "cdxml", "tableId": node.attr("id")}),
            payload: ObjectPayload {
                resource_ref: None,
                bbox: Some([0.0, 0.0, round2(right - left), round2(bottom - top)]),
                spectrum: None,
                geometry: None,
                constraint: None,
                table: Some(crate::TableData {
                    rows,
                    columns,
                    row_guides: row_guides
                        .into_iter()
                        .map(|guide| round2(guide - top))
                        .collect(),
                    column_guides: column_guides
                        .into_iter()
                        .map(|guide| round2(guide - left))
                        .collect(),
                    cells,
                    default_border,
                }),
                extra: BTreeMap::new(),
            },
            children: Vec::new(),
        });
        index += 1;
    }
}

fn table_line_style(value: Option<&str>) -> crate::TableLineStyle {
    match value.unwrap_or("Solid") {
        value if value.contains("Dashed") => crate::TableLineStyle::Dashed,
        value if value.contains("Bold") => crate::TableLineStyle::Bold,
        value if value.contains("Wavy") => crate::TableLineStyle::Wavy,
        _ => crate::TableLineStyle::Solid,
    }
}

pub(in crate::cdxml) fn associate_table_cell_contents(root: &XmlNode, objects: &mut [SceneObject]) {
    let mut page_sources = BTreeMap::<String, BTreeSet<String>>::new();
    for table in descendants(root)
        .into_iter()
        .filter(|node| node.is("table"))
    {
        for page in table.direct_children("page") {
            let Some(page_id) = page.attr("id") else {
                continue;
            };
            let sources = descendants(page)
                .into_iter()
                .filter(|node| !matches!(node.name.as_str(), "page" | "border"))
                .filter_map(|node| node.attr("id").map(ToString::to_string))
                .collect();
            page_sources.insert(page_id.to_string(), sources);
        }
    }
    let object_sources = objects
        .iter()
        .filter(|object| object.object_type != "table")
        .map(|object| {
            let mut values = BTreeSet::new();
            collect_id_meta_values(&object.meta, &mut values);
            (object.id.clone(), values)
        })
        .collect::<Vec<_>>();
    for object in objects
        .iter_mut()
        .filter(|object| object.object_type == "table")
    {
        let Some(table) = object.payload.table.as_mut() else {
            continue;
        };
        for cell in &mut table.cells {
            let Some(page_id) = cell.id.strip_prefix("cell_cdxml_") else {
                continue;
            };
            let Some(sources) = page_sources.get(page_id) else {
                continue;
            };
            cell.content_object_ids = object_sources
                .iter()
                .filter(|(_, object_ids)| !object_ids.is_disjoint(sources))
                .map(|(object_id, _)| object_id.clone())
                .collect();
        }
    }
}

fn collect_id_meta_values(value: &Value, values: &mut BTreeSet<String>) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                if key.ends_with("Id") {
                    if let Some(value) = value.as_str() {
                        values.insert(value.to_string());
                    }
                }
                collect_id_meta_values(value, values);
            }
        }
        Value::Array(array) => {
            for value in array {
                collect_id_meta_values(value, values);
            }
        }
        _ => {}
    }
}

pub(in crate::cdxml) fn append_tlc_plate_shape_objects(
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
            link_policy: Default::default(),
            meta: json!({"source": "cdxml", "tlcPlateId": node.attr("id")}),
            payload: ObjectPayload {
                resource_ref: None,
                bbox: Some([
                    0.0,
                    0.0,
                    round2(bbox[2] - bbox[0]),
                    round2(bbox[3] - bbox[1]),
                ]),
                spectrum: None,
                geometry: None,
                constraint: None,
                table: None,
                extra,
            },
            children: Vec::new(),
        });
        index += 1;
    }
}

pub(super) fn normalize_tlc_spot_extent(raw: f64) -> f64 {
    if raw.abs() > 1024.0 {
        raw / 65536.0
    } else {
        raw
    }
}
