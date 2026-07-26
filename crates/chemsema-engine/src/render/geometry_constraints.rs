use super::*;

struct AnnotationPaint {
    object_id: Option<String>,
    stroke: String,
    stroke_width: f64,
    dash_array: Vec<f64>,
    offset: Vector,
}

pub(super) fn render_geometry_constraint_object(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemSemaDocument,
    object: &SceneObject,
) {
    let evaluated = match crate::geometry_constraints::evaluate_annotation(document, object) {
        Ok(evaluated) => evaluated,
        Err(error) => {
            render_invalid_annotation(out, object, &error);
            return;
        }
    };
    let mut points = evaluated_points(&evaluated);
    let annotation_offset =
        Vector::new(object.transform.translate[0], object.transform.translate[1]);
    for point in &mut points {
        *point = point.translated(annotation_offset);
    }
    if points.is_empty() {
        return;
    }
    let object_id = Some(object.id.clone());
    let style = object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref));
    let stroke = style
        .and_then(|style| style_string(style, "stroke"))
        .or_else(|| {
            object
                .payload
                .constraint
                .as_ref()
                .and_then(|constraint| constraint.display.fill.clone())
        })
        .unwrap_or_else(|| "#000000".to_string());
    let stroke_width = style
        .and_then(|style| style_number(style, "strokeWidth"))
        .unwrap_or_else(|| {
            document
                .style
                .defaults
                .get("graphicLineWidth")
                .copied()
                .unwrap_or(crate::DEFAULT_BOND_STROKE)
        });
    let dash_array = style
        .and_then(|style| style_number_array(style, "dashArray"))
        .unwrap_or_default();
    let paint = AnnotationPaint {
        object_id,
        stroke,
        stroke_width,
        dash_array,
        offset: annotation_offset,
    };
    if let Some(geometry) = object.payload.geometry.as_ref() {
        render_geometry(out, geometry, points, paint);
        return;
    }
    let Some(constraint) = object.payload.constraint.as_ref() else {
        return;
    };
    render_constraint(out, document, constraint, evaluated, points, paint);
}

fn evaluated_points(evaluated: &crate::geometry_constraints::EvaluatedAnnotation) -> Vec<Point> {
    match evaluated {
        crate::geometry_constraints::EvaluatedAnnotation::Geometry(geometry) => match geometry {
            crate::geometry_constraints::EvaluatedGeometry::Point(point) => vec![*point],
            crate::geometry_constraints::EvaluatedGeometry::Line { start, end, .. } => {
                vec![*start, *end]
            }
            crate::geometry_constraints::EvaluatedGeometry::Plane { boundary, .. } => {
                boundary.clone()
            }
        },
        crate::geometry_constraints::EvaluatedAnnotation::Distance { start, end, .. } => {
            vec![*start, *end]
        }
        crate::geometry_constraints::EvaluatedAnnotation::Angle { points, .. } => points.clone(),
        crate::geometry_constraints::EvaluatedAnnotation::ExclusionSphere { center, .. } => {
            vec![*center]
        }
    }
}

fn render_geometry(
    out: &mut Vec<RenderPrimitive>,
    geometry: &crate::GeometryData,
    points: Vec<Point>,
    paint: AnnotationPaint,
) {
    use crate::GeometryFeature;
    match geometry.feature {
        GeometryFeature::CentroidFromPoints => {
            render_annotation_cross(
                out,
                paint.object_id,
                annotation_centroid(&points),
                paint.stroke,
                paint.stroke_width,
            );
        }
        GeometryFeature::LineFromPoints => {
            if let (Some(first), Some(last)) = (points.first(), points.last()) {
                out.push(RenderPrimitive::Line {
                    role: RenderRole::DocumentGraphic,
                    object_id: paint.object_id,
                    bond_id: None,
                    from: *first,
                    to: *last,
                    stroke: paint.stroke,
                    stroke_width: paint.stroke_width,
                    dash_array: paint.dash_array,
                });
            }
        }
        GeometryFeature::PlaneFromPoints | GeometryFeature::PlaneFromPointLine => {
            if points.len() >= 3 {
                out.push(RenderPrimitive::Polygon {
                    role: RenderRole::DocumentGraphic,
                    object_id: paint.object_id,
                    node_id: None,
                    bond_id: None,
                    points,
                    fill: "#7c3aed20".to_string(),
                    stroke: paint.stroke,
                    stroke_width: paint.stroke_width,
                });
            }
        }
        GeometryFeature::NormalFromPointPlane => {
            if points.len() == 2 {
                out.push(RenderPrimitive::Line {
                    role: RenderRole::DocumentGraphic,
                    object_id: paint.object_id,
                    bond_id: None,
                    from: points[0],
                    to: points[1],
                    stroke: paint.stroke,
                    stroke_width: paint.stroke_width,
                    dash_array: paint.dash_array,
                });
            }
        }
        GeometryFeature::PointFromPointPointDistance
        | GeometryFeature::PointFromPointPointPercentage
        | GeometryFeature::PointFromPointNormalDistance => {
            render_annotation_cross(
                out,
                paint.object_id,
                points[0],
                paint.stroke,
                paint.stroke_width,
            );
        }
    }
}

fn render_constraint(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemSemaDocument,
    constraint: &crate::ConstraintData,
    evaluated: crate::geometry_constraints::EvaluatedAnnotation,
    points: Vec<Point>,
    paint: AnnotationPaint,
) {
    match constraint.constraint_type {
        crate::ConstraintType::Distance if points.len() >= 2 => {
            if constraint.display.indicator_visible {
                out.push(RenderPrimitive::Line {
                    role: RenderRole::DocumentGraphic,
                    object_id: paint.object_id.clone(),
                    bond_id: None,
                    from: points[0],
                    to: points[1],
                    stroke: paint.stroke.clone(),
                    stroke_width: paint.stroke_width,
                    dash_array: paint.dash_array,
                });
            }
            render_constraint_value(
                out,
                paint.object_id,
                annotation_centroid(&points[..2]),
                constraint,
                paint.stroke,
                paint.offset,
            );
        }
        crate::ConstraintType::Angle if points.len() >= 3 => {
            if constraint.display.indicator_visible {
                out.push(RenderPrimitive::Polyline {
                    role: RenderRole::DocumentGraphic,
                    object_id: paint.object_id.clone(),
                    bond_id: None,
                    points: if points.len() == 3 {
                        vec![points[0], points[1], points[2]]
                    } else {
                        points.clone()
                    },
                    stroke: paint.stroke.clone(),
                    stroke_width: paint.stroke_width,
                    dash_array: paint.dash_array,
                    line_cap: Some("round".to_string()),
                    line_join: Some("round".to_string()),
                });
            }
            render_constraint_value(
                out,
                paint.object_id,
                points[1],
                constraint,
                paint.stroke,
                paint.offset,
            );
        }
        crate::ConstraintType::ExclusionSphere => {
            let points_per_angstrom = document
                .style
                .defaults
                .get("bondLength")
                .copied()
                .unwrap_or(crate::DEFAULT_BOND_LENGTH_PT)
                / 1.5;
            let radius = match evaluated {
                crate::geometry_constraints::EvaluatedAnnotation::ExclusionSphere {
                    radius_angstrom,
                    ..
                } => radius_angstrom * points_per_angstrom,
                _ => return,
            };
            out.push(RenderPrimitive::Circle {
                role: RenderRole::DocumentGraphic,
                object_id: paint.object_id,
                node_id: None,
                center: points[0],
                radius,
                fill: "#7c3aed18".to_string(),
                stroke: paint.stroke,
                stroke_width: paint.stroke_width,
            });
        }
        _ => {}
    }
}

fn render_invalid_annotation(out: &mut Vec<RenderPrimitive>, object: &SceneObject, error: &str) {
    let Some([x, y, width, height]) = object.payload.bbox else {
        return;
    };
    out.push(RenderPrimitive::Text {
        role: RenderRole::DocumentDiagnostic,
        object_id: Some(object.id.clone()),
        node_id: None,
        x: x + width * 0.5,
        y: y + height * 0.5,
        baseline_offset: None,
        dominant_baseline: Some("middle".to_string()),
        text: format!("Invalid annotation: {error}"),
        font_size: 7.5,
        font_family: Some("Arial".to_string()),
        fill: Some("#c62828".to_string()),
        text_anchor: Some("middle".to_string()),
        line_height: None,
        preserve_lines: false,
        box_width: None,
        runs: Vec::new(),
        rotate: 0.0,
        rotate_center: None,
    });
}

fn annotation_centroid(points: &[Point]) -> Point {
    let (x, y) = points
        .iter()
        .fold((0.0, 0.0), |(x, y), point| (x + point.x, y + point.y));
    Point::new(x / points.len() as f64, y / points.len() as f64)
}

fn render_annotation_cross(
    out: &mut Vec<RenderPrimitive>,
    object_id: Option<String>,
    center: Point,
    stroke: String,
    stroke_width: f64,
) {
    for (from, to) in [
        (
            Point::new(center.x - 3.0, center.y),
            Point::new(center.x + 3.0, center.y),
        ),
        (
            Point::new(center.x, center.y - 3.0),
            Point::new(center.x, center.y + 3.0),
        ),
    ] {
        out.push(RenderPrimitive::Line {
            role: RenderRole::DocumentGraphic,
            object_id: object_id.clone(),
            bond_id: None,
            from,
            to,
            stroke: stroke.clone(),
            stroke_width,
            dash_array: Vec::new(),
        });
    }
}

fn render_constraint_value(
    out: &mut Vec<RenderPrimitive>,
    object_id: Option<String>,
    position: Point,
    constraint: &crate::ConstraintData,
    fill: String,
    annotation_offset: Vector,
) {
    let Some(text) = crate::geometry_constraints::constraint_value_text(constraint) else {
        return;
    };
    let position = match constraint.display.positioning_type {
        crate::AnnotationPositioningType::Auto => position,
        crate::AnnotationPositioningType::Absolute | crate::AnnotationPositioningType::Angle => {
            let Some(position) = constraint.display.position else {
                return;
            };
            Point::new(position[0], position[1]).translated(annotation_offset)
        }
        crate::AnnotationPositioningType::Offset => {
            let Some([dx, dy]) = constraint.display.positioning_offset else {
                return;
            };
            Point::new(position.x + dx, position.y + dy)
        }
    };
    let run_text = text.clone();
    out.push(RenderPrimitive::Text {
        role: RenderRole::DocumentText,
        object_id,
        node_id: None,
        x: position.x,
        y: position.y - 3.0,
        baseline_offset: None,
        dominant_baseline: Some("auto".to_string()),
        text,
        font_size: constraint.display.font_size.unwrap_or(7.5),
        font_family: Some(
            constraint
                .display
                .font_family
                .clone()
                .unwrap_or_else(|| "Arial".to_string()),
        ),
        fill: Some(constraint.display.fill.clone().unwrap_or(fill)),
        text_anchor: Some("middle".to_string()),
        line_height: None,
        preserve_lines: false,
        box_width: None,
        runs: vec![crate::LabelRun {
            text: run_text,
            font_family: constraint.display.font_family.clone(),
            font_size: constraint.display.font_size,
            fill: constraint.display.fill.clone(),
            font_weight: Some(constraint.display.font_weight),
            font_style: constraint.display.italic.then(|| "italic".to_string()),
            underline: Some(constraint.display.underline),
            outline: Some(false),
            shadow: Some(false),
            script: Some("normal".to_string()),
        }],
        rotate: 0.0,
        rotate_center: None,
    });
}
