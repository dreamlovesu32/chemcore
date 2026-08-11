use super::*;

pub(crate) fn render_curve_object(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemSemaDocument,
    object: &SceneObject,
) {
    let Some(values) = object
        .payload
        .extra
        .get("curvePoints")
        .and_then(JsonValue::as_array)
    else {
        return;
    };
    let points: Vec<_> = values
        .iter()
        .filter_map(|value| {
            let pair = value.as_array()?;
            Some(Point::new(
                pair.first()?.as_f64()? + object.transform.translate[0],
                pair.get(1)?.as_f64()? + object.transform.translate[1],
            ))
        })
        .collect();
    if points.len() < 6 || (points.len() - 3) % 3 != 0 {
        return;
    }
    let body = &points[1..points.len() - 1];
    let style = object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref));
    let stroke = style
        .and_then(|value| style_string(value, "stroke"))
        .unwrap_or_else(|| "#000000".to_string());
    let stroke_width = style
        .and_then(|value| style_number(value, "strokeWidth"))
        .unwrap_or(crate::DEFAULT_BOND_STROKE);
    let dash_array = style
        .and_then(|value| style_number_array(value, "dashArray"))
        .unwrap_or_default();
    let mut d = format!("M {:.4} {:.4}", body[0].x, body[0].y);
    for segment in body[1..].chunks_exact(3) {
        d.push_str(&format!(
            " C {:.4} {:.4} {:.4} {:.4} {:.4} {:.4}",
            segment[0].x, segment[0].y, segment[1].x, segment[1].y, segment[2].x, segment[2].y,
        ));
    }
    let closed = payload_bool(&object.payload, "closed").unwrap_or(false);
    let mut rendered_points = body.to_vec();
    if closed {
        let closing_control_1 = points[points.len() - 1];
        let closing_control_2 = points[0];
        let closing_end = body[0];
        d.push_str(&format!(
            " C {:.4} {:.4} {:.4} {:.4} {:.4} {:.4}",
            closing_control_1.x,
            closing_control_1.y,
            closing_control_2.x,
            closing_control_2.y,
            closing_end.x,
            closing_end.y,
        ));
        rendered_points.extend([closing_control_1, closing_control_2]);
    }
    out.push(RenderPrimitive::Path {
        role: RenderRole::DocumentGraphic,
        object_id: Some(object.id.clone()),
        bond_id: None,
        d,
        points: rendered_points,
        stroke: stroke.clone(),
        stroke_width,
        dash_array,
        line_cap: Some("butt".to_string()),
        line_join: Some("round".to_string()),
        rotate: object.transform.rotate,
        rotate_center: None,
    });
    if !payload_string(&object.payload, "arrowheadType")
        .unwrap_or_else(|| "Solid".to_string())
        .eq_ignore_ascii_case("solid")
    {
        return;
    }
    let length = payload_number(&object.payload, "headLength")
        .unwrap_or(crate::DEFAULT_CURVE_ARROW_HEAD_LENGTH_RATIO * stroke_width);
    let center_length = payload_number(&object.payload, "headCenterLength")
        .unwrap_or(crate::DEFAULT_CURVE_ARROW_CENTER_LENGTH_RATIO * stroke_width);
    let width = payload_number(&object.payload, "headWidth")
        .unwrap_or(crate::DEFAULT_CURVE_ARROW_WIDTH_RATIO * stroke_width);
    let head = payload_string(&object.payload, "head").unwrap_or_else(|| "none".to_string());
    let head_style = curve_endpoint_style(&head);
    if head_style.enabled() {
        let end = *body.last().unwrap_or(&body[0]);
        let outer_guide = points[points.len() - 1];
        super::arrows::render_curve_solid_arrow_head(
            out,
            end,
            outer_guide,
            length,
            center_length,
            width,
            head_style,
            stroke_width,
            &stroke,
            Some(object.id.clone()),
        );
    }
    let tail = payload_string(&object.payload, "tail").unwrap_or_else(|| "none".to_string());
    let tail_style = curve_endpoint_style(&tail);
    if tail_style.enabled() {
        let start = body[0];
        let outer_guide = points[0];
        super::arrows::render_curve_solid_arrow_head(
            out,
            start,
            outer_guide,
            length,
            center_length,
            width,
            tail_style,
            stroke_width,
            &stroke,
            Some(object.id.clone()),
        );
    }
}

fn curve_endpoint_style(value: &str) -> super::arrows::RenderArrowEndpointStyle {
    match value.to_ascii_lowercase().as_str() {
        "full" => super::arrows::RenderArrowEndpointStyle::Full,
        "half" | "half-left" | "halfleft" | "left" | "top" => {
            super::arrows::RenderArrowEndpointStyle::Left
        }
        "half-right" | "halfright" | "right" | "bottom" => {
            super::arrows::RenderArrowEndpointStyle::Right
        }
        _ => super::arrows::RenderArrowEndpointStyle::None,
    }
}
