use super::*;

impl Engine {
    pub(super) fn set_bio_shape_direct(
        &mut self,
        object_id: &str,
        data: crate::BioShapeData,
    ) -> bool {
        let Some(original) = self
            .state
            .document
            .find_scene_object(object_id)
            .filter(|object| object.payload.bio_shape.is_some())
            .cloned()
        else {
            return false;
        };
        if original.payload.bio_shape.as_ref() == Some(&data) {
            return false;
        }
        self.push_undo_snapshot();
        let Some(object) = self.state.document.find_scene_object_mut(object_id) else {
            return false;
        };
        object.payload.bbox = Some(bio_shape_local_bbox(&data));
        object.payload.bio_shape = Some(data);
        true
    }
}

pub(super) fn bio_shape_local_bbox(data: &crate::BioShapeData) -> [f64; 4] {
    let center = Point::new(data.center[0], data.center[1]);
    let major = center.distance(Point::new(data.major_axis_end[0], data.major_axis_end[1]));
    let minor = center.distance(Point::new(data.minor_axis_end[0], data.minor_axis_end[1]));
    let padding = data
        .bold_width
        .max(data.line_width)
        .max(data.margin_width)
        .max(1.0);
    [
        round2(center.x - major - padding),
        round2(center.y - minor - padding),
        round2((major + padding) * 2.0),
        round2((minor + padding) * 2.0),
    ]
}
