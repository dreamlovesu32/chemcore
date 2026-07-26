use super::*;

impl Engine {
    pub(super) fn set_plasmid_map_direct(
        &mut self,
        object_id: &str,
        data: crate::PlasmidMapData,
    ) -> bool {
        let Some(original) = self
            .state
            .document
            .find_scene_object(object_id)
            .filter(|object| object.payload.plasmid_map.is_some())
            .cloned()
        else {
            return false;
        };
        if original.payload.plasmid_map.as_ref() == Some(&data) {
            return false;
        }
        let Some([x, y, width, height]) = original.payload.bbox else {
            return false;
        };
        let center = Point::new(
            original.transform.translate[0] + x + width * 0.5,
            original.transform.translate[1] + y + height * 0.5,
        );
        let label_extent = data
            .markers
            .iter()
            .map(|marker| marker.offset.max(0.0) + data.label_size * 2.0)
            .fold(data.label_size, f64::max);
        let extent = data.radius + label_extent;
        self.push_undo_snapshot();
        let Some(object) = self.state.document.find_scene_object_mut(object_id) else {
            return false;
        };
        object.transform.translate = [center.x - extent, center.y - extent];
        object.payload.bbox = Some([0.0, 0.0, extent * 2.0, extent * 2.0]);
        object.payload.plasmid_map = Some(data);
        true
    }
}
