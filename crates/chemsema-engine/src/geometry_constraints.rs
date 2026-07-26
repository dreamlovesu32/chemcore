use crate::{
    ChemSemaDocument, ConstraintData, ConstraintType, GeometryData, GeometryFeature, Point,
    SceneObject, SceneObjectKind, Vector, EPSILON,
};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum EvaluatedGeometry {
    Point(Point),
    Line {
        origin: Point,
        direction: Vector,
        start: Point,
        end: Point,
    },
    Plane {
        origin: Point,
        boundary: Vec<Point>,
    },
}

impl EvaluatedGeometry {
    fn kind_name(&self) -> &'static str {
        match self {
            Self::Point(_) => "point",
            Self::Line { .. } => "line",
            Self::Plane { .. } => "plane",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum EvaluatedAnnotation {
    Geometry(EvaluatedGeometry),
    Distance {
        start: Point,
        end: Point,
        measured: f64,
    },
    Angle {
        points: Vec<Point>,
        measured_degrees: f64,
    },
    ExclusionSphere {
        center: Point,
        radius_angstrom: f64,
    },
}

pub(crate) fn constraint_value_text(data: &ConstraintData) -> Option<String> {
    if !data.display.auto_value {
        return data.display.text_override.clone();
    }
    let unit = match data.constraint_type {
        ConstraintType::Distance | ConstraintType::ExclusionSphere => "\u{00C5}",
        ConstraintType::Angle => "\u{00B0}",
    };
    match (data.minimum, data.maximum) {
        (Some(minimum), Some(maximum)) if (minimum - maximum).abs() > EPSILON => Some(format!(
            "{}\u{2013}{} {unit}",
            crate::round2(minimum),
            crate::round2(maximum)
        )),
        (Some(value), _) | (_, Some(value)) => Some(format!("{} {unit}", crate::round2(value))),
        (None, None) => None,
    }
}

pub(crate) fn evaluate_annotation(
    document: &ChemSemaDocument,
    object: &SceneObject,
) -> Result<EvaluatedAnnotation, String> {
    let mut visiting = BTreeSet::new();
    evaluate_annotation_inner(document, object, &mut visiting)
}

fn evaluate_annotation_inner(
    document: &ChemSemaDocument,
    object: &SceneObject,
    visiting: &mut BTreeSet<String>,
) -> Result<EvaluatedAnnotation, String> {
    if !visiting.insert(object.id.clone()) {
        return Err(format!(
            "geometry/constraint basis cycle reaches '{}'",
            object.id
        ));
    }
    let result = if let Some(geometry) = object.payload.geometry.as_ref() {
        evaluate_geometry(document, object, geometry, visiting).map(EvaluatedAnnotation::Geometry)
    } else if let Some(constraint) = object.payload.constraint.as_ref() {
        evaluate_constraint(document, object, constraint, visiting)
    } else {
        Err(format!(
            "{} object '{}' has no matching native payload",
            object.object_type, object.id
        ))
    };
    visiting.remove(&object.id);
    result
}

fn evaluate_geometry(
    document: &ChemSemaDocument,
    object: &SceneObject,
    data: &GeometryData,
    visiting: &mut BTreeSet<String>,
) -> Result<EvaluatedGeometry, String> {
    ensure_resolved(object, &data.unresolved_basis_ids)?;
    let basis = resolve_basis(document, object, &data.basis_entity_ids, visiting)?;
    match data.feature {
        GeometryFeature::PointFromPointPointDistance => {
            require_signature(object, &basis, &["point", "point"])?;
            let [first, second] = point_pair(&basis)?;
            let distance = required_relation_value(object, data)?;
            let direction = direction_between(object, first, second)?;
            Ok(EvaluatedGeometry::Point(Point::new(
                first.x + direction.x * distance * points_per_angstrom(document),
                first.y + direction.y * distance * points_per_angstrom(document),
            )))
        }
        GeometryFeature::PointFromPointPointPercentage => {
            require_signature(object, &basis, &["point", "point"])?;
            let [first, second] = point_pair(&basis)?;
            let percentage = required_relation_value(object, data)? / 100.0;
            Ok(EvaluatedGeometry::Point(Point::new(
                first.x + (second.x - first.x) * percentage,
                first.y + (second.y - first.y) * percentage,
            )))
        }
        GeometryFeature::PointFromPointNormalDistance => {
            require_signature(object, &basis, &["point", "line"])?;
            let point = expect_point(object, &basis[0], 0)?;
            let direction = expect_line(object, &basis[1], 1)?.1;
            let distance = required_relation_value(object, data)? * points_per_angstrom(document);
            Ok(EvaluatedGeometry::Point(Point::new(
                point.x + direction.x * distance,
                point.y + direction.y * distance,
            )))
        }
        GeometryFeature::LineFromPoints => {
            require_all_points(object, &basis, 2)?;
            best_fit_line(
                object,
                &basis
                    .iter()
                    .map(|entity| expect_point(object, entity, 0))
                    .collect::<Result<Vec<_>, _>>()?,
            )
        }
        GeometryFeature::PlaneFromPoints => {
            require_all_points(object, &basis, 3)?;
            plane_from_points(
                object,
                &basis
                    .iter()
                    .map(|entity| expect_point(object, entity, 0))
                    .collect::<Result<Vec<_>, _>>()?,
            )
        }
        GeometryFeature::PlaneFromPointLine => {
            require_signature(object, &basis, &["point", "line"])?;
            let point = expect_point(object, &basis[0], 0)?;
            let (_, _, start, end) = expect_line(object, &basis[1], 1)?;
            plane_from_points(object, &[point, start, end])
        }
        GeometryFeature::CentroidFromPoints => {
            require_all_points(object, &basis, 1)?;
            let points = basis
                .iter()
                .map(|entity| expect_point(object, entity, 0))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(EvaluatedGeometry::Point(centroid(&points)))
        }
        GeometryFeature::NormalFromPointPlane => {
            require_signature(object, &basis, &["point", "plane"])?;
            let point = expect_point(object, &basis[0], 0)?;
            let plane = expect_plane(object, &basis[1], 1)?;
            let direction = direction_between(object, plane.0, point)?;
            let length = plane
                .1
                .iter()
                .map(|boundary| boundary.distance(plane.0))
                .fold(0.0, f64::max)
                .max(3.0);
            Ok(EvaluatedGeometry::Line {
                origin: point,
                direction,
                start: point,
                end: Point::new(
                    point.x + direction.x * length,
                    point.y + direction.y * length,
                ),
            })
        }
    }
}

fn evaluate_constraint(
    document: &ChemSemaDocument,
    object: &SceneObject,
    data: &ConstraintData,
    visiting: &mut BTreeSet<String>,
) -> Result<EvaluatedAnnotation, String> {
    ensure_resolved(object, &data.unresolved_basis_ids)?;
    let basis = resolve_basis(document, object, &data.basis_entity_ids, visiting)?;
    match data.constraint_type {
        ConstraintType::Distance => {
            require_signature(object, &basis, &["point", "point"])?;
            let [start, end] = point_pair(&basis)?;
            Ok(EvaluatedAnnotation::Distance {
                start,
                end,
                measured: start.distance(end) / points_per_angstrom(document),
            })
        }
        ConstraintType::Angle => {
            if basis.len() == 3 && basis.iter().all(|entity| entity.kind_name() == "point") {
                let points = basis
                    .iter()
                    .map(|entity| expect_point(object, entity, 0))
                    .collect::<Result<Vec<_>, _>>()?;
                let first = direction_between(object, points[1], points[0])?;
                let second = direction_between(object, points[1], points[2])?;
                Ok(EvaluatedAnnotation::Angle {
                    measured_degrees: smaller_angle(first, second),
                    points,
                })
            } else if basis.len() == 4 && basis.iter().all(|entity| entity.kind_name() == "point") {
                let points = basis
                    .iter()
                    .map(|entity| expect_point(object, entity, 0))
                    .collect::<Result<Vec<_>, _>>()?;
                let first = direction_between(object, points[0], points[1])?;
                let second = direction_between(object, points[2], points[3])?;
                Ok(EvaluatedAnnotation::Angle {
                    measured_degrees: smaller_angle(first, second),
                    points,
                })
            } else if basis.len() == 2
                && basis
                    .iter()
                    .all(|entity| matches!(entity.kind_name(), "line" | "plane"))
            {
                let (first_direction, mut points) =
                    annotation_direction_and_points(object, &basis[0], 0)?;
                let (second_direction, second_points) =
                    annotation_direction_and_points(object, &basis[1], 1)?;
                points.extend(second_points);
                Ok(EvaluatedAnnotation::Angle {
                    measured_degrees: smaller_angle(first_direction, second_direction),
                    points,
                })
            } else {
                Err(format!(
                    "constraint '{}' angle basis must be 3/4 points or 2 lines",
                    object.id
                ))
            }
        }
        ConstraintType::ExclusionSphere => {
            require_all_points(object, &basis, 1)?;
            let points = basis
                .iter()
                .map(|entity| expect_point(object, entity, 0))
                .collect::<Result<Vec<_>, _>>()?;
            let radius_angstrom = data
                .maximum
                .or(data.minimum)
                .ok_or_else(|| {
                    format!(
                        "constraint '{}' exclusion sphere requires a radius bound",
                        object.id
                    )
                })?
                .abs();
            Ok(EvaluatedAnnotation::ExclusionSphere {
                center: centroid(&points),
                radius_angstrom,
            })
        }
    }
}

fn resolve_basis(
    document: &ChemSemaDocument,
    owner: &SceneObject,
    ids: &[String],
    visiting: &mut BTreeSet<String>,
) -> Result<Vec<EvaluatedGeometry>, String> {
    ids.iter()
        .map(|id| resolve_entity(document, owner, id, visiting))
        .collect()
}

fn resolve_entity(
    document: &ChemSemaDocument,
    owner: &SceneObject,
    id: &str,
    visiting: &mut BTreeSet<String>,
) -> Result<EvaluatedGeometry, String> {
    for entry in document.editable_fragments() {
        if let Some(node) = entry.fragment.nodes.iter().find(|node| node.id == id) {
            return Ok(EvaluatedGeometry::Point(entry.world_point_for_node(node)));
        }
        if let Some(bond) = entry.fragment.bonds.iter().find(|bond| bond.id == id) {
            let begin = entry
                .fragment
                .nodes
                .iter()
                .find(|node| node.id == bond.begin)
                .ok_or_else(|| format!("bond '{}' has no begin node", bond.id))?;
            let end = entry
                .fragment
                .nodes
                .iter()
                .find(|node| node.id == bond.end)
                .ok_or_else(|| format!("bond '{}' has no end node", bond.id))?;
            let start = entry.world_point_for_node(begin);
            let finish = entry.world_point_for_node(end);
            return line_from_pair(owner, start, finish);
        }
    }
    let object = document.find_scene_object(id).ok_or_else(|| {
        format!(
            "geometry/constraint '{}' references missing basis entity '{}'",
            owner.id, id
        )
    })?;
    match object.kind() {
        SceneObjectKind::Geometry => match evaluate_annotation_inner(document, object, visiting)? {
            EvaluatedAnnotation::Geometry(geometry) => Ok(geometry),
            _ => Err(format!("basis object '{}' is not geometry", id)),
        },
        _ => Err(format!(
            "geometry/constraint '{}' cannot use {} object '{}' as basis",
            owner.id, object.object_type, id
        )),
    }
}

fn required_relation_value(owner: &SceneObject, data: &GeometryData) -> Result<f64, String> {
    data.relation_value.ok_or_else(|| {
        format!(
            "geometry '{}' feature {} requires relationValue",
            owner.id,
            data.feature.as_cdxml()
        )
    })
}

fn ensure_resolved(owner: &SceneObject, unresolved: &[String]) -> Result<(), String> {
    if unresolved.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "geometry/constraint '{}' has unresolved basis ids: {}",
            owner.id,
            unresolved.join(", ")
        ))
    }
}

fn require_signature(
    owner: &SceneObject,
    basis: &[EvaluatedGeometry],
    expected: &[&str],
) -> Result<(), String> {
    let actual = basis
        .iter()
        .map(EvaluatedGeometry::kind_name)
        .collect::<Vec<_>>();
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "geometry/constraint '{}' requires basis [{}], got [{}]",
            owner.id,
            expected.join(", "),
            actual.join(", ")
        ))
    }
}

fn require_all_points(
    owner: &SceneObject,
    basis: &[EvaluatedGeometry],
    minimum: usize,
) -> Result<(), String> {
    if basis.len() >= minimum && basis.iter().all(|entity| entity.kind_name() == "point") {
        Ok(())
    } else {
        Err(format!(
            "geometry/constraint '{}' requires at least {minimum} point basis objects",
            owner.id
        ))
    }
}

fn point_pair(basis: &[EvaluatedGeometry]) -> Result<[Point; 2], String> {
    match basis {
        [EvaluatedGeometry::Point(first), EvaluatedGeometry::Point(second)] => {
            Ok([*first, *second])
        }
        _ => Err("point pair signature was not validated".to_string()),
    }
}

fn expect_point(
    owner: &SceneObject,
    entity: &EvaluatedGeometry,
    index: usize,
) -> Result<Point, String> {
    if let EvaluatedGeometry::Point(point) = entity {
        Ok(*point)
    } else {
        Err(format!(
            "geometry/constraint '{}' basis {index} must be a point",
            owner.id
        ))
    }
}

fn expect_line(
    owner: &SceneObject,
    entity: &EvaluatedGeometry,
    index: usize,
) -> Result<(Point, Vector, Point, Point), String> {
    if let EvaluatedGeometry::Line {
        origin,
        direction,
        start,
        end,
    } = entity
    {
        Ok((*origin, *direction, *start, *end))
    } else {
        Err(format!(
            "geometry/constraint '{}' basis {index} must be a line",
            owner.id
        ))
    }
}

fn expect_plane(
    owner: &SceneObject,
    entity: &EvaluatedGeometry,
    index: usize,
) -> Result<(Point, Vec<Point>), String> {
    if let EvaluatedGeometry::Plane { origin, boundary } = entity {
        Ok((*origin, boundary.clone()))
    } else {
        Err(format!(
            "geometry/constraint '{}' basis {index} must be a plane",
            owner.id
        ))
    }
}

fn annotation_direction_and_points(
    owner: &SceneObject,
    entity: &EvaluatedGeometry,
    index: usize,
) -> Result<(Vector, Vec<Point>), String> {
    match entity {
        EvaluatedGeometry::Line {
            direction,
            start,
            end,
            ..
        } => Ok((*direction, vec![*start, *end])),
        EvaluatedGeometry::Plane { boundary, .. } => {
            let Some((&first, &second)) = boundary.first().zip(boundary.get(1)) else {
                return Err(format!(
                    "geometry/constraint '{}' plane basis {index} has no projected edge",
                    owner.id
                ));
            };
            Ok((direction_between(owner, first, second)?, boundary.clone()))
        }
        EvaluatedGeometry::Point(_) => Err(format!(
            "geometry/constraint '{}' basis {index} must be a line or plane",
            owner.id
        )),
    }
}

fn best_fit_line(owner: &SceneObject, points: &[Point]) -> Result<EvaluatedGeometry, String> {
    let origin = centroid(points);
    let mut xx = 0.0;
    let mut xy = 0.0;
    let mut yy = 0.0;
    for point in points {
        let dx = point.x - origin.x;
        let dy = point.y - origin.y;
        xx += dx * dx;
        xy += dx * dy;
        yy += dy * dy;
    }
    if xx + yy <= EPSILON {
        return Err(format!(
            "geometry '{}' best-fit line requires distinct points",
            owner.id
        ));
    }
    let angle = 0.5 * (2.0 * xy).atan2(xx - yy);
    let direction = Vector::new(angle.cos(), angle.sin());
    let mut minimum = f64::INFINITY;
    let mut maximum = f64::NEG_INFINITY;
    for point in points {
        let projection = (point.x - origin.x) * direction.x + (point.y - origin.y) * direction.y;
        minimum = minimum.min(projection);
        maximum = maximum.max(projection);
    }
    Ok(EvaluatedGeometry::Line {
        origin,
        direction,
        start: Point::new(
            origin.x + direction.x * minimum,
            origin.y + direction.y * minimum,
        ),
        end: Point::new(
            origin.x + direction.x * maximum,
            origin.y + direction.y * maximum,
        ),
    })
}

fn line_from_pair(
    owner: &SceneObject,
    start: Point,
    end: Point,
) -> Result<EvaluatedGeometry, String> {
    let direction = direction_between(owner, start, end)?;
    Ok(EvaluatedGeometry::Line {
        origin: Point::new((start.x + end.x) * 0.5, (start.y + end.y) * 0.5),
        direction,
        start,
        end,
    })
}

fn plane_from_points(owner: &SceneObject, points: &[Point]) -> Result<EvaluatedGeometry, String> {
    let boundary = convex_hull(points);
    if boundary.len() < 3 || polygon_area(&boundary).abs() <= EPSILON {
        return Err(format!(
            "geometry '{}' plane requires three non-collinear projected points",
            owner.id
        ));
    }
    Ok(EvaluatedGeometry::Plane {
        origin: centroid(points),
        boundary,
    })
}

fn direction_between(owner: &SceneObject, start: Point, end: Point) -> Result<Vector, String> {
    let vector = Vector::new(end.x - start.x, end.y - start.y);
    if vector.length() <= EPSILON {
        Err(format!(
            "geometry/constraint '{}' requires distinct basis positions",
            owner.id
        ))
    } else {
        Ok(vector.normalized())
    }
}

fn smaller_angle(first: Vector, second: Vector) -> f64 {
    let dot = (first.x * second.x + first.y * second.y).clamp(-1.0, 1.0);
    dot.acos().to_degrees()
}

fn points_per_angstrom(document: &ChemSemaDocument) -> f64 {
    document
        .style
        .defaults
        .get("bondLength")
        .copied()
        .unwrap_or(crate::DEFAULT_BOND_LENGTH_PT)
        / 1.5
}

fn centroid(points: &[Point]) -> Point {
    let (x, y) = points
        .iter()
        .fold((0.0, 0.0), |(x, y), point| (x + point.x, y + point.y));
    Point::new(x / points.len() as f64, y / points.len() as f64)
}

fn convex_hull(points: &[Point]) -> Vec<Point> {
    let mut points = points.to_vec();
    points.sort_by(|left, right| {
        left.x
            .total_cmp(&right.x)
            .then_with(|| left.y.total_cmp(&right.y))
    });
    points.dedup_by(|left, right| {
        (left.x - right.x).abs() <= EPSILON && (left.y - right.y).abs() <= EPSILON
    });
    if points.len() <= 2 {
        return points;
    }
    let mut lower = Vec::new();
    for point in &points {
        while lower.len() >= 2
            && cross(lower[lower.len() - 2], lower[lower.len() - 1], *point) <= EPSILON
        {
            lower.pop();
        }
        lower.push(*point);
    }
    let mut upper = Vec::new();
    for point in points.iter().rev() {
        while upper.len() >= 2
            && cross(upper[upper.len() - 2], upper[upper.len() - 1], *point) <= EPSILON
        {
            upper.pop();
        }
        upper.push(*point);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

fn cross(origin: Point, first: Point, second: Point) -> f64 {
    (first.x - origin.x) * (second.y - origin.y) - (first.y - origin.y) * (second.x - origin.x)
}

fn polygon_area(points: &[Point]) -> f64 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
        .map(|(first, second)| first.x * second.y - second.x * first.y)
        .sum::<f64>()
        * 0.5
}

pub(crate) fn validate_annotation_graph(document: &ChemSemaDocument) -> Result<(), String> {
    for object in document.scene_objects().into_iter().filter(|object| {
        matches!(
            object.kind(),
            SceneObjectKind::Geometry | SceneObjectKind::Constraint
        )
    }) {
        let unresolved = object
            .payload
            .geometry
            .as_ref()
            .map(|data| !data.unresolved_basis_ids.is_empty())
            .or_else(|| {
                object
                    .payload
                    .constraint
                    .as_ref()
                    .map(|data| !data.unresolved_basis_ids.is_empty())
            })
            .unwrap_or(false);
        if unresolved {
            continue;
        }
        evaluate_annotation(document, object)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LinkPolicy, ObjectPayload, Transform};
    use serde_json::Value;
    use std::collections::BTreeMap;

    fn annotation(
        id: &str,
        geometry: Option<GeometryData>,
        constraint: Option<ConstraintData>,
    ) -> SceneObject {
        SceneObject {
            id: id.to_string(),
            object_type: if geometry.is_some() {
                "geometry"
            } else {
                "constraint"
            }
            .to_string(),
            name: String::new(),
            visible: true,
            locked: false,
            z_index: 1,
            transform: Transform::identity(),
            style_ref: None,
            link_policy: LinkPolicy::Linked,
            meta: Value::Null,
            payload: ObjectPayload {
                resource_ref: None,
                bbox: None,
                spectrum: None,
                geometry,
                constraint,
                extra: BTreeMap::new(),
            },
            children: Vec::new(),
        }
    }

    #[test]
    fn best_fit_line_uses_all_points() {
        let owner = annotation(
            "g1",
            Some(GeometryData {
                feature: GeometryFeature::LineFromPoints,
                basis_entity_ids: vec!["n1".into(), "n2".into(), "n3".into()],
                unresolved_basis_ids: Vec::new(),
                relation_value: None,
                point_is_directed: false,
            }),
            None,
        );
        let geometry = best_fit_line(
            &owner,
            &[
                Point::new(-10.0, -1.0),
                Point::new(0.0, 0.0),
                Point::new(10.0, 1.0),
            ],
        )
        .unwrap();
        let EvaluatedGeometry::Line { direction, .. } = geometry else {
            panic!("expected line");
        };
        assert!(direction.x.abs() > 0.99);
        assert!(direction.y.abs() < 0.11);
    }

    #[test]
    fn plane_rejects_collinear_projection() {
        let owner = annotation(
            "g1",
            Some(GeometryData {
                feature: GeometryFeature::PlaneFromPoints,
                basis_entity_ids: vec!["n1".into(), "n2".into(), "n3".into()],
                unresolved_basis_ids: Vec::new(),
                relation_value: None,
                point_is_directed: false,
            }),
            None,
        );
        let error = plane_from_points(
            &owner,
            &[
                Point::new(0.0, 0.0),
                Point::new(1.0, 1.0),
                Point::new(2.0, 2.0),
            ],
        )
        .unwrap_err();
        assert!(error.contains("non-collinear"));
    }
}
