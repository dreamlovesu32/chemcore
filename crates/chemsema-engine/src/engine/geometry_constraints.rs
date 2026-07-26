use super::Engine;
use crate::{
    AnnotationPropertiesPatch, ConstraintData, ConstraintType, GeometryData, GeometryFeature,
    LinkEndpoint, LinkPolicy, LinkRelation, ObjectPayload, SceneObject, SelectionState, Transform,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Clone, Copy, PartialEq, Eq)]
enum AnnotationBasisKind {
    Point,
    Line,
    Plane,
}

impl Engine {
    pub fn annotation_dialog_json(&self, annotation: &str) -> String {
        let selected = if annotation == "selected" {
            self.state
                .selection
                .arrow_objects
                .first()
                .and_then(|id| self.state.document.find_scene_object(id))
                .filter(|object| {
                    self.state.selection.arrow_objects.len() == 1
                        && matches!(
                            object.kind(),
                            crate::SceneObjectKind::Geometry | crate::SceneObjectKind::Constraint
                        )
                })
        } else {
            None
        };
        let annotation = selected
            .map(annotation_name_for_object)
            .unwrap_or(annotation);
        let object_id = selected.map(|object| object.id.clone());
        let mut fields = Vec::new();
        if matches!(
            annotation,
            "point-distance" | "point-percentage" | "point-normal-distance"
        ) {
            let value = selected
                .and_then(|object| object.payload.geometry.as_ref())
                .and_then(|geometry| geometry.relation_value)
                .unwrap_or(if annotation == "point-percentage" {
                    50.0
                } else {
                    1.0
                });
            fields.push(json!({
                "key": "relationValue",
                "label": if annotation == "point-percentage" { "Percentage" } else { "Distance" },
                "value": value,
                "unit": if annotation == "point-percentage" { "%" } else { "Å" },
                "valueKind": "number"
            }));
        }
        if matches!(
            annotation,
            "distance" | "angle" | "dihedral" | "exclusion-sphere"
        ) {
            let selected_constraint =
                selected.and_then(|object| object.payload.constraint.as_ref());
            let default_value = match annotation {
                "distance" => self
                    .selected_point_distance_angstrom(&self.annotation_basis_entity_ids())
                    .unwrap_or(0.0),
                "angle" | "dihedral" => self
                    .selected_angle_degrees(&self.annotation_basis_entity_ids())
                    .unwrap_or(0.0),
                "exclusion-sphere" => 1.0,
                _ => 0.0,
            };
            let unit = if matches!(annotation, "angle" | "dihedral") {
                "°"
            } else {
                "Å"
            };
            fields.extend([
                json!({
                    "key": "minimum",
                    "label": "Minimum",
                    "value": selected_constraint.and_then(|constraint| constraint.minimum).unwrap_or(default_value),
                    "unit": unit,
                    "valueKind": "number"
                }),
                json!({
                    "key": "maximum",
                    "label": "Maximum",
                    "value": selected_constraint.and_then(|constraint| constraint.maximum).unwrap_or(default_value),
                    "unit": unit,
                    "valueKind": "number"
                }),
            ]);
            let display = selected_constraint
                .map(|constraint| constraint.display.clone())
                .unwrap_or_default();
            let automatic_text = selected_constraint
                .and_then(crate::geometry_constraints::constraint_value_text)
                .unwrap_or_else(|| {
                    let unit = if matches!(annotation, "angle" | "dihedral") {
                        "\u{00B0}"
                    } else {
                        "\u{00C5}"
                    };
                    format!("{default_value} {unit}")
                });
            let position = selected
                .and_then(|object| self.annotation_default_label_position(object))
                .or_else(|| {
                    self.selected_annotation_centroid()
                        .map(|point| crate::Point::new(point.x, point.y - 3.0))
                })
                .unwrap_or_else(|| crate::Point::new(0.0, 0.0));
            let explicit_position = display
                .position
                .map(|point| crate::Point::new(point[0], point[1]))
                .unwrap_or(position);
            let offset = display.positioning_offset.unwrap_or([0.0, 0.0]);
            fields.extend([
                json!({
                    "key": "autoValue",
                    "label": "Update value automatically",
                    "value": display.auto_value,
                    "valueKind": "boolean"
                }),
                json!({
                    "key": "textOverride",
                    "label": "Displayed text",
                    "value": display.text_override.unwrap_or(automatic_text),
                    "valueKind": "text"
                }),
                json!({
                    "key": "positioningType",
                    "label": "Position",
                    "value": display.positioning_type.as_cdxml(),
                    "valueKind": "choice",
                    "options": [
                        {"value": "auto", "label": "Automatic"},
                        {"value": "absolute", "label": "Absolute"},
                        {"value": "offset", "label": "Offset"},
                        {"value": "angle", "label": "Angle"}
                    ]
                }),
                json!({"key": "positionX", "label": "X", "value": explicit_position.x, "unit": "pt", "valueKind": "number"}),
                json!({"key": "positionY", "label": "Y", "value": explicit_position.y, "unit": "pt", "valueKind": "number"}),
                json!({"key": "positioningOffsetX", "label": "Offset X", "value": offset[0], "unit": "pt", "valueKind": "number"}),
                json!({"key": "positioningOffsetY", "label": "Offset Y", "value": offset[1], "unit": "pt", "valueKind": "number"}),
                json!({"key": "positioningAngle", "label": "Position angle", "value": display.positioning_angle.unwrap_or(0.0), "unit": "\u{00B0}", "valueKind": "number"}),
                json!({
                    "key": "indicatorVisible",
                    "label": "Show indicator",
                    "value": display.indicator_visible,
                    "valueKind": "boolean"
                }),
                json!({"key": "fontFamily", "label": "Font", "value": display.font_family.unwrap_or_else(|| "Arial".to_string()), "valueKind": "text"}),
                json!({"key": "fontSize", "label": "Font size", "value": display.font_size.unwrap_or(7.5), "unit": "pt", "valueKind": "number", "minimum": 0.1}),
                json!({"key": "fill", "label": "Color", "value": display.fill.unwrap_or_else(|| "#000000".to_string()), "valueKind": "text"}),
                json!({"key": "bold", "label": "Bold", "value": display.font_weight >= 600, "valueKind": "boolean"}),
                json!({"key": "italic", "label": "Italic", "value": display.italic, "valueKind": "boolean"}),
                json!({"key": "underline", "label": "Underline", "value": display.underline, "valueKind": "boolean"}),
            ]);
        }
        if matches!(annotation, "point-distance" | "point-normal-distance") {
            let value = selected
                .and_then(|object| object.payload.geometry.as_ref())
                .is_some_and(|geometry| geometry.point_is_directed);
            fields.push(json!({
                "key": "pointIsDirected",
                "label": "Directed point",
                "value": value,
                "valueKind": "boolean"
            }));
        }
        if matches!(annotation, "distance" | "angle" | "dihedral") {
            let value = selected
                .and_then(|object| object.payload.constraint.as_ref())
                .is_some_and(|constraint| constraint.ignore_unconnected_atoms);
            fields.push(json!({
                "key": "ignoreUnconnectedAtoms",
                "label": "Ignore unconnected atoms",
                "value": value,
                "valueKind": "boolean"
            }));
        }
        if annotation == "dihedral" {
            let value = selected
                .and_then(|object| object.payload.constraint.as_ref())
                .is_some_and(|constraint| constraint.dihedral_is_chiral);
            fields.push(json!({
                "key": "dihedralIsChiral",
                "label": "Chiral dihedral",
                "value": value,
                "valueKind": "boolean"
            }));
        }
        json!({
            "kind": "annotation-properties",
            "title": if object_id.is_some() { "Annotation Properties" } else { "Create Annotation" },
            "annotation": annotation,
            "objectId": object_id,
            "fields": fields
        })
        .to_string()
    }

    pub(super) fn annotation_menu_values(&self) -> Vec<(&'static str, &'static str)> {
        if self.state.selection.region {
            return Vec::new();
        }
        let basis = self.annotation_basis_entity_ids();
        let kinds = basis
            .iter()
            .map(|id| self.annotation_basis_kind(id))
            .collect::<Option<Vec<_>>>();
        let Some(kinds) = kinds else {
            return Vec::new();
        };
        let mut values = Vec::new();
        if kinds.iter().all(|kind| *kind == AnnotationBasisKind::Point) {
            match kinds.len() {
                1 => values.push(("Exclusion Sphere", "exclusion-sphere")),
                2 => values.extend([
                    ("Distance", "distance"),
                    ("Point at Distance", "point-distance"),
                    ("Point at Percentage", "point-percentage"),
                    ("Best-fit Line", "line"),
                    ("Centroid", "centroid"),
                    ("Exclusion Sphere", "exclusion-sphere"),
                ]),
                3 => values.extend([
                    ("Angle", "angle"),
                    ("Best-fit Line", "line"),
                    ("Best-fit Plane", "plane"),
                    ("Centroid", "centroid"),
                    ("Exclusion Sphere", "exclusion-sphere"),
                ]),
                4.. => values.extend([
                    ("Dihedral", "dihedral"),
                    ("Best-fit Line", "line"),
                    ("Best-fit Plane", "plane"),
                    ("Centroid", "centroid"),
                    ("Exclusion Sphere", "exclusion-sphere"),
                ]),
                _ => {}
            }
            return values;
        }
        match kinds.as_slice() {
            [AnnotationBasisKind::Point, AnnotationBasisKind::Line] => {
                values.push(("Plane from Point and Line", "plane-point-line"));
                if basis.get(1).is_some_and(|id| {
                    self.state
                        .document
                        .find_scene_object(id)
                        .and_then(|object| object.payload.geometry.as_ref())
                        .is_some_and(|geometry| {
                            geometry.feature == GeometryFeature::NormalFromPointPlane
                        })
                }) {
                    values.push(("Point at Normal Distance", "point-normal-distance"));
                }
            }
            [AnnotationBasisKind::Point, AnnotationBasisKind::Plane] => {
                values.push(("Normal from Point and Plane", "normal"));
            }
            [AnnotationBasisKind::Line, AnnotationBasisKind::Line]
            | [AnnotationBasisKind::Line, AnnotationBasisKind::Plane]
            | [AnnotationBasisKind::Plane, AnnotationBasisKind::Line]
            | [AnnotationBasisKind::Plane, AnnotationBasisKind::Plane] => {
                values.push(("Angle", "angle"));
            }
            _ => {}
        }
        values
    }

    pub(super) fn create_annotation_untracked(
        &mut self,
        annotation: &str,
        properties: Option<AnnotationPropertiesPatch>,
    ) -> Result<bool, String> {
        let allowed = self
            .annotation_menu_values()
            .iter()
            .any(|(_, value)| *value == annotation);
        if !allowed {
            return Err(format!(
                "annotation '{annotation}' is not valid for the ordered selection"
            ));
        }
        let basis_entity_ids = self.annotation_basis_entity_ids();
        let link_basis_entity_ids = basis_entity_ids.clone();
        let mut geometry = match annotation {
            "point-distance" => Some(GeometryData {
                feature: GeometryFeature::PointFromPointPointDistance,
                basis_entity_ids: basis_entity_ids.clone(),
                unresolved_basis_ids: Vec::new(),
                relation_value: Some(1.0),
                point_is_directed: false,
            }),
            "point-percentage" => Some(GeometryData {
                feature: GeometryFeature::PointFromPointPointPercentage,
                basis_entity_ids: basis_entity_ids.clone(),
                unresolved_basis_ids: Vec::new(),
                relation_value: Some(50.0),
                point_is_directed: false,
            }),
            "point-normal-distance" => Some(GeometryData {
                feature: GeometryFeature::PointFromPointNormalDistance,
                basis_entity_ids: basis_entity_ids.clone(),
                unresolved_basis_ids: Vec::new(),
                relation_value: Some(1.0),
                point_is_directed: false,
            }),
            "line" => Some(GeometryData {
                feature: GeometryFeature::LineFromPoints,
                basis_entity_ids: basis_entity_ids.clone(),
                unresolved_basis_ids: Vec::new(),
                relation_value: None,
                point_is_directed: false,
            }),
            "plane" => Some(GeometryData {
                feature: GeometryFeature::PlaneFromPoints,
                basis_entity_ids: basis_entity_ids.clone(),
                unresolved_basis_ids: Vec::new(),
                relation_value: None,
                point_is_directed: false,
            }),
            "plane-point-line" => Some(GeometryData {
                feature: GeometryFeature::PlaneFromPointLine,
                basis_entity_ids: basis_entity_ids.clone(),
                unresolved_basis_ids: Vec::new(),
                relation_value: None,
                point_is_directed: false,
            }),
            "normal" => Some(GeometryData {
                feature: GeometryFeature::NormalFromPointPlane,
                basis_entity_ids: basis_entity_ids.clone(),
                unresolved_basis_ids: Vec::new(),
                relation_value: None,
                point_is_directed: false,
            }),
            "centroid" => Some(GeometryData {
                feature: GeometryFeature::CentroidFromPoints,
                basis_entity_ids: basis_entity_ids.clone(),
                unresolved_basis_ids: Vec::new(),
                relation_value: None,
                point_is_directed: false,
            }),
            _ => None,
        };
        let constraint_type = match annotation {
            "distance" => Some(ConstraintType::Distance),
            "angle" | "dihedral" => Some(ConstraintType::Angle),
            "exclusion-sphere" => Some(ConstraintType::ExclusionSphere),
            _ => None,
        };
        let mut constraint = constraint_type.map(|constraint_type| {
            let default_value = match constraint_type {
                ConstraintType::Distance => {
                    self.selected_point_distance_angstrom(&basis_entity_ids)
                }
                ConstraintType::Angle => self.selected_angle_degrees(&basis_entity_ids),
                ConstraintType::ExclusionSphere => Some(1.0),
            };
            ConstraintData {
                constraint_type,
                basis_entity_ids,
                unresolved_basis_ids: Vec::new(),
                minimum: default_value,
                maximum: default_value,
                ignore_unconnected_atoms: false,
                dihedral_is_chiral: annotation == "dihedral",
                point_is_directed: false,
                display: crate::AnnotationDisplay::default(),
            }
        });
        if let Some(properties) = properties {
            validate_annotation_properties(&properties)?;
            if let Some(geometry) = geometry.as_mut() {
                if let Some(value) = properties.relation_value {
                    geometry.relation_value = Some(value);
                }
                if let Some(value) = properties.point_is_directed {
                    geometry.point_is_directed = value;
                }
            }
            if let Some(constraint) = constraint.as_mut() {
                if let Some(value) = properties.minimum {
                    constraint.minimum = Some(value);
                }
                if let Some(value) = properties.maximum {
                    constraint.maximum = Some(value);
                }
                if let Some(value) = properties.point_is_directed {
                    constraint.point_is_directed = value;
                }
                if let Some(value) = properties.ignore_unconnected_atoms {
                    constraint.ignore_unconnected_atoms = value;
                }
                if let Some(value) = properties.dihedral_is_chiral {
                    constraint.dihedral_is_chiral = value;
                }
                apply_annotation_display_patch(&mut constraint.display, &properties)?;
            }
        }
        let kind = if geometry.is_some() {
            crate::SceneObjectKind::Geometry
        } else {
            crate::SceneObjectKind::Constraint
        };
        let id = self.next_id(kind.as_str());
        let z_index = self
            .state
            .document
            .objects
            .iter()
            .map(|object| object.z_index)
            .max()
            .unwrap_or(0)
            + 1;
        self.state.document.objects.push(SceneObject {
            id: id.clone(),
            object_type: kind.as_str().to_string(),
            name: annotation.to_string(),
            visible: true,
            locked: false,
            z_index,
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
        });
        let link_id = self.next_id("link");
        self.state.document.links.push(LinkRelation {
            id: link_id,
            kind: "annotation-basis".to_string(),
            endpoints: std::iter::once(LinkEndpoint {
                entity_id: id.clone(),
                role: "annotation".to_string(),
            })
            .chain(
                link_basis_entity_ids
                    .into_iter()
                    .map(|basis_id| LinkEndpoint {
                        entity_id: basis_id,
                        role: "basis".to_string(),
                    }),
            )
            .collect(),
            data: Value::Null,
        });
        self.state.selection = SelectionState {
            arrow_objects: vec![id],
            ..SelectionState::default()
        };
        Ok(true)
    }

    pub(super) fn update_annotation_untracked(
        &mut self,
        object_id: &str,
        properties: AnnotationPropertiesPatch,
    ) -> Result<bool, String> {
        validate_annotation_properties(&properties)?;
        let object = self
            .state
            .document
            .find_scene_object_mut(object_id)
            .ok_or_else(|| format!("annotation object '{object_id}' does not exist"))?;
        let mut changed = false;
        if let Some(geometry) = object.payload.geometry.as_mut() {
            if let Some(value) = properties.relation_value {
                changed |= geometry.relation_value != Some(value);
                geometry.relation_value = Some(value);
            }
            if let Some(value) = properties.point_is_directed {
                changed |= geometry.point_is_directed != value;
                geometry.point_is_directed = value;
            }
        } else if let Some(constraint) = object.payload.constraint.as_mut() {
            let previous = constraint.clone();
            let mut next = previous.clone();
            if let Some(value) = properties.minimum {
                next.minimum = Some(value);
            }
            if let Some(value) = properties.maximum {
                next.maximum = Some(value);
            }
            if let Some(value) = properties.point_is_directed {
                next.point_is_directed = value;
            }
            if let Some(value) = properties.ignore_unconnected_atoms {
                next.ignore_unconnected_atoms = value;
            }
            if let Some(value) = properties.dihedral_is_chiral {
                next.dihedral_is_chiral = value;
            }
            apply_annotation_display_patch(&mut next.display, &properties)?;
            changed |= next != previous;
            *constraint = next;
        } else {
            return Err(format!("object '{object_id}' is not a native annotation"));
        }
        Ok(changed)
    }

    fn annotation_basis_entity_ids(&self) -> Vec<String> {
        if !self.state.selection.ordered_entities.is_empty() {
            return self.state.selection.ordered_entities.clone();
        }
        self.state
            .selection
            .nodes
            .iter()
            .chain(self.state.selection.label_nodes.iter())
            .chain(self.state.selection.bonds.iter())
            .chain(self.state.selection.arrow_objects.iter().filter(|id| {
                self.state
                    .document
                    .find_scene_object(id)
                    .is_some_and(|object| object.kind() == crate::SceneObjectKind::Geometry)
            }))
            .cloned()
            .collect()
    }

    fn annotation_basis_kind(&self, id: &str) -> Option<AnnotationBasisKind> {
        for entry in self.state.document.editable_fragments() {
            if entry.fragment.nodes.iter().any(|node| node.id == id) {
                return Some(AnnotationBasisKind::Point);
            }
            if entry.fragment.bonds.iter().any(|bond| bond.id == id) {
                return Some(AnnotationBasisKind::Line);
            }
        }
        let geometry = self
            .state
            .document
            .find_scene_object(id)?
            .payload
            .geometry
            .as_ref()?;
        Some(match geometry.feature {
            GeometryFeature::PointFromPointPointDistance
            | GeometryFeature::PointFromPointPointPercentage
            | GeometryFeature::PointFromPointNormalDistance
            | GeometryFeature::CentroidFromPoints => AnnotationBasisKind::Point,
            GeometryFeature::LineFromPoints | GeometryFeature::NormalFromPointPlane => {
                AnnotationBasisKind::Line
            }
            GeometryFeature::PlaneFromPoints | GeometryFeature::PlaneFromPointLine => {
                AnnotationBasisKind::Plane
            }
        })
    }

    fn selected_point_distance_angstrom(&self, basis: &[String]) -> Option<f64> {
        let [first, second] = basis else {
            return None;
        };
        let first = self.annotation_node_point(first)?;
        let second = self.annotation_node_point(second)?;
        Some(crate::round2(
            first.distance(second) / (self.options.bond_length_world_pt().value() / 1.5),
        ))
    }

    fn selected_angle_degrees(&self, basis: &[String]) -> Option<f64> {
        let [first, vertex, third] = basis else {
            return None;
        };
        let first = self.annotation_node_point(first)?;
        let vertex = self.annotation_node_point(vertex)?;
        let third = self.annotation_node_point(third)?;
        let left = crate::direction_from_angle(crate::angle_between(vertex, first));
        let right = crate::direction_from_angle(crate::angle_between(vertex, third));
        Some(crate::round2(
            (left.x * right.x + left.y * right.y)
                .clamp(-1.0, 1.0)
                .acos()
                .to_degrees(),
        ))
    }

    fn annotation_node_point(&self, id: &str) -> Option<crate::Point> {
        self.state
            .document
            .editable_fragments()
            .into_iter()
            .find_map(|entry| {
                entry
                    .fragment
                    .nodes
                    .iter()
                    .find(|node| node.id == id)
                    .map(|node| entry.world_point_for_node(node))
            })
    }

    fn selected_annotation_centroid(&self) -> Option<crate::Point> {
        let points = self
            .annotation_basis_entity_ids()
            .iter()
            .filter_map(|id| self.annotation_node_point(id))
            .collect::<Vec<_>>();
        (!points.is_empty()).then(|| {
            let (x, y) = points
                .iter()
                .fold((0.0, 0.0), |(x, y), point| (x + point.x, y + point.y));
            crate::Point::new(x / points.len() as f64, y / points.len() as f64)
        })
    }

    fn annotation_default_label_position(&self, object: &SceneObject) -> Option<crate::Point> {
        match crate::geometry_constraints::evaluate_annotation(&self.state.document, object).ok()? {
            crate::geometry_constraints::EvaluatedAnnotation::Distance { start, end, .. } => Some(
                crate::Point::new((start.x + end.x) * 0.5, (start.y + end.y) * 0.5 - 3.0),
            ),
            crate::geometry_constraints::EvaluatedAnnotation::Angle { points, .. } => {
                points.get(1).copied()
            }
            crate::geometry_constraints::EvaluatedAnnotation::ExclusionSphere {
                center,
                radius_angstrom,
            } => Some(crate::Point::new(
                center.x,
                center.y - radius_angstrom * self.options.bond_length_world_pt().value() / 1.5,
            )),
            crate::geometry_constraints::EvaluatedAnnotation::Geometry(_) => None,
        }
    }
}

fn validate_annotation_properties(properties: &AnnotationPropertiesPatch) -> Result<(), String> {
    for value in [
        properties.relation_value,
        properties.minimum,
        properties.maximum,
        properties.position_x,
        properties.position_y,
        properties.positioning_angle,
        properties.positioning_offset_x,
        properties.positioning_offset_y,
        properties.font_size,
    ]
    .into_iter()
    .flatten()
    {
        if !value.is_finite() {
            return Err("annotation values must be finite".to_string());
        }
    }
    if properties.font_size.is_some_and(|value| value <= 0.0) {
        return Err("annotation font size must be positive".to_string());
    }
    if properties
        .minimum
        .zip(properties.maximum)
        .is_some_and(|(minimum, maximum)| minimum > maximum)
    {
        return Err("annotation minimum cannot exceed maximum".to_string());
    }
    Ok(())
}

fn apply_annotation_display_patch(
    display: &mut crate::AnnotationDisplay,
    properties: &AnnotationPropertiesPatch,
) -> Result<(), String> {
    if let Some(value) = properties.auto_value {
        display.auto_value = value;
    }
    if let Some(value) = &properties.text_override {
        display.text_override = Some(value.clone());
    }
    if let Some(value) = properties.positioning_type {
        display.positioning_type = value;
    }
    if properties.position_x.is_some() || properties.position_y.is_some() {
        let existing = display.position.unwrap_or([0.0, 0.0]);
        display.position = Some([
            properties.position_x.unwrap_or(existing[0]),
            properties.position_y.unwrap_or(existing[1]),
        ]);
    }
    if let Some(value) = properties.positioning_angle {
        display.positioning_angle = Some(value);
    }
    if properties.positioning_offset_x.is_some() || properties.positioning_offset_y.is_some() {
        let existing = display.positioning_offset.unwrap_or([0.0, 0.0]);
        display.positioning_offset = Some([
            properties.positioning_offset_x.unwrap_or(existing[0]),
            properties.positioning_offset_y.unwrap_or(existing[1]),
        ]);
    }
    if let Some(value) = properties.indicator_visible {
        display.indicator_visible = value;
    }
    if let Some(value) = &properties.font_family {
        if value.trim().is_empty() {
            return Err("annotation font family cannot be empty".to_string());
        }
        display.font_family = Some(value.clone());
    }
    if let Some(value) = properties.font_size {
        display.font_size = Some(value);
    }
    if let Some(value) = &properties.fill {
        if value.trim().is_empty() {
            return Err("annotation color cannot be empty".to_string());
        }
        display.fill = Some(value.clone());
    }
    if let Some(value) = properties.bold {
        display.font_weight = if value { 700 } else { 400 };
    }
    if let Some(value) = properties.italic {
        display.italic = value;
    }
    if let Some(value) = properties.underline {
        display.underline = value;
    }
    match display.positioning_type {
        crate::AnnotationPositioningType::Auto => {}
        crate::AnnotationPositioningType::Absolute if display.position.is_none() => {
            return Err("absolute annotation positioning requires x and y".to_string());
        }
        crate::AnnotationPositioningType::Offset if display.positioning_offset.is_none() => {
            return Err("offset annotation positioning requires x and y offsets".to_string());
        }
        crate::AnnotationPositioningType::Angle if display.positioning_angle.is_none() => {
            return Err("angle annotation positioning requires an angle".to_string());
        }
        _ => {}
    }
    Ok(())
}

fn annotation_name_for_object(object: &SceneObject) -> &'static str {
    if let Some(geometry) = object.payload.geometry.as_ref() {
        return match geometry.feature {
            GeometryFeature::PointFromPointPointDistance => "point-distance",
            GeometryFeature::PointFromPointPointPercentage => "point-percentage",
            GeometryFeature::PointFromPointNormalDistance => "point-normal-distance",
            GeometryFeature::LineFromPoints => "line",
            GeometryFeature::PlaneFromPoints => "plane",
            GeometryFeature::PlaneFromPointLine => "plane-point-line",
            GeometryFeature::CentroidFromPoints => "centroid",
            GeometryFeature::NormalFromPointPlane => "normal",
        };
    }
    object
        .payload
        .constraint
        .as_ref()
        .map(|constraint| match constraint.constraint_type {
            ConstraintType::Distance => "distance",
            ConstraintType::Angle if constraint.dihedral_is_chiral => "dihedral",
            ConstraintType::Angle => "angle",
            ConstraintType::ExclusionSphere => "exclusion-sphere",
        })
        .unwrap_or("")
}
