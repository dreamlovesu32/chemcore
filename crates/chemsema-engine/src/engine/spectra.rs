use super::*;

impl Engine {
    pub(super) fn set_spectrum_data_direct(
        &mut self,
        object_id: &str,
        spectrum: crate::SpectrumData,
    ) -> bool {
        if spectrum.validate().is_err() {
            return false;
        }
        let Some(object) = self
            .state
            .document
            .find_scene_object_mut(object_id)
            .filter(|object| object.object_type == "spectrum" && !object.locked)
        else {
            return false;
        };
        if object.payload.spectrum.as_ref() == Some(&spectrum) {
            return false;
        }
        object.payload.spectrum = Some(spectrum);
        true
    }
}
