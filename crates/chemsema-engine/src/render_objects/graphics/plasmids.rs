use super::*;

pub(super) fn render_plasmid_map_object(
    out: &mut Vec<RenderPrimitive>,
    object: &SceneObject,
    style: ShapeStyleSpec,
) {
    let (Some([x, y, width, height]), Some(plasmid)) =
        (object.payload.bbox, object.payload.plasmid_map.as_ref())
    else {
        return;
    };
    let center = Point::new(
        object.transform.translate[0] + x + width * 0.5,
        object.transform.translate[1] + y + height * 0.5,
    );
    let rotate = object.transform.rotate;
    let stroke = if plasmid.color.is_empty() {
        style.base_color().to_string()
    } else {
        plasmid.color.clone()
    };
    out.push(RenderPrimitive::Ellipse {
        role: RenderRole::DocumentGraphic,
        object_id: Some(object.id.clone()),
        center,
        rx: plasmid.radius,
        ry: plasmid.radius,
        rotate,
        fill: None,
        stroke: Some(stroke.clone()),
        stroke_width: plasmid.line_width.max(0.05),
        dash_array: Vec::new(),
        fill_gradient: None,
    });

    for region in &plasmid.regions {
        let start = plasmid.angle_degrees(region.start);
        let end = plasmid.angle_degrees(region.end);
        let sweep = (end - start).rem_euclid(360.0);
        if sweep <= crate::EPSILON {
            continue;
        }
        let radius = plasmid.radius + region.offset;
        let half_width = region.width * 0.5;
        let segments = ((sweep / 6.0).ceil() as usize).clamp(4, 96);
        let mut points = Vec::with_capacity(segments * 2 + 4);
        for index in 0..=segments {
            let angle = start + sweep * index as f64 / segments as f64;
            points.push(plasmid_point(center, radius + half_width, angle, rotate));
        }
        if region.arrow_at_end {
            points.push(plasmid_point(
                center,
                radius,
                end + region.width.max(4.0),
                rotate,
            ));
        }
        for index in (0..=segments).rev() {
            let angle = start + sweep * index as f64 / segments as f64;
            points.push(plasmid_point(center, radius - half_width, angle, rotate));
        }
        if region.arrow_at_start {
            points.push(plasmid_point(
                center,
                radius,
                start - region.width.max(4.0),
                rotate,
            ));
        }
        let fill = if region.filled {
            color_with_alpha(&region.color, region.alpha)
        } else if region.shaded {
            "#d0d0d0".to_string()
        } else if region.faded {
            color_with_alpha(&region.color, region.alpha * 0.35)
        } else {
            "#ffffff".to_string()
        };
        out.push(RenderPrimitive::Polygon {
            role: RenderRole::DocumentGraphic,
            object_id: Some(object.id.clone()),
            node_id: None,
            bond_id: None,
            points,
            fill,
            stroke: region.color.clone(),
            stroke_width: plasmid.line_width.max(0.05),
        });
    }

    if plasmid.show_base_pairs {
        out.push(RenderPrimitive::Text {
            role: RenderRole::DocumentText,
            object_id: Some(object.id.clone()),
            node_id: None,
            bond_id: None,
            x: center.x,
            y: center.y,
            baseline_offset: None,
            dominant_baseline: Some("central".to_string()),
            text: format!("{} bp", plasmid.number_base_pairs),
            font_size: plasmid.label_size,
            font_family: Some("Arial".to_string()),
            fill: Some(stroke.clone()),
            text_anchor: Some("middle".to_string()),
            line_height: None,
            preserve_lines: false,
            box_width: None,
            runs: Vec::new(),
            rotate,
            rotate_center: Some(center),
        });
    }

    for marker in &plasmid.markers {
        let position_angle = plasmid.angle_degrees(marker.position);
        let label_angle = marker.label_angle.unwrap_or(position_angle);
        let ring_point = plasmid_point(center, plasmid.radius, position_angle, rotate);
        let label_point = plasmid_point(
            center,
            plasmid.radius + marker.offset.max(plasmid.margin_width),
            label_angle,
            rotate,
        );
        out.push(RenderPrimitive::Line {
            role: RenderRole::DocumentGraphic,
            object_id: Some(object.id.clone()),
            bond_id: None,
            from: ring_point,
            to: label_point,
            stroke: marker.color.clone(),
            stroke_width: plasmid.line_width.max(0.05),
            dash_array: Vec::new(),
        });
        out.push(RenderPrimitive::Text {
            role: RenderRole::DocumentText,
            object_id: Some(object.id.clone()),
            node_id: None,
            bond_id: None,
            x: label_point.x,
            y: label_point.y,
            baseline_offset: None,
            dominant_baseline: Some("central".to_string()),
            text: marker.label.clone(),
            font_size: plasmid.label_size,
            font_family: Some("Arial".to_string()),
            fill: Some(marker.color.clone()),
            text_anchor: Some(if label_point.x < center.x {
                "end".to_string()
            } else {
                "start".to_string()
            }),
            line_height: None,
            preserve_lines: false,
            box_width: None,
            runs: Vec::new(),
            rotate,
            rotate_center: Some(label_point),
        });
    }
}

fn plasmid_point(center: Point, radius: f64, angle: f64, rotate: f64) -> Point {
    let radians = (angle + rotate).to_radians();
    Point::new(
        center.x + radius * radians.sin(),
        center.y - radius * radians.cos(),
    )
}

fn color_with_alpha(color: &str, alpha: f64) -> String {
    let alpha = alpha.clamp(0.0, 1.0);
    if alpha >= 0.999 {
        color.to_string()
    } else {
        let color = color.trim_start_matches('#');
        if color.len() == 6 {
            let (Ok(r), Ok(g), Ok(b)) = (
                u8::from_str_radix(&color[0..2], 16),
                u8::from_str_radix(&color[2..4], 16),
                u8::from_str_radix(&color[4..6], 16),
            ) else {
                return format!("#{color}");
            };
            format!("rgba({r},{g},{b},{alpha:.4})")
        } else {
            color.to_string()
        }
    }
}
