use super::*;

impl Engine {
    pub fn tlc_spot_hit_test(&self, point: Point) -> Option<TlcSpotHit> {
        let mut best: Option<(f64, TlcSpotHit)> = None;
        for object in self.state.document.scene_objects() {
            if let Some(geometry) = tlc_plate_geometry(object) {
                for (lane_index, lane_x) in geometry.lane_centers.iter().enumerate() {
                    let Some(spots) = geometry.spots.get(lane_index) else {
                        continue;
                    };
                    for (spot_index, rf) in spots.iter().enumerate() {
                        let local_center = Point::new(
                            *lane_x,
                            geometry.origin_y - (geometry.origin_y - geometry.solvent_y) * *rf,
                        );
                        let center = crate::rotate_point_around(
                            local_center,
                            geometry.center,
                            geometry.rotate,
                        );
                        let distance = center.distance(point);
                        if distance > geometry.spot_radius + px_to_pt(6.0) {
                            continue;
                        }
                        let hit = TlcSpotHit {
                            object_id: object.id.clone(),
                            lane_index,
                            spot_index,
                            rf: round2(*rf),
                            value_kind: "rf".to_string(),
                            center,
                            guide_points: tlc_lane_guide_points(&geometry, lane_index),
                        };
                        match &best {
                            Some((best_distance, _)) if *best_distance <= distance => {}
                            _ => best = Some((distance, hit)),
                        }
                    }
                }
            }
            if let Some(geometry) = gel_plate_geometry(object) {
                for (lane_index, bands) in geometry.bands.iter().enumerate() {
                    let Some(&lane_x) = geometry.lane_centers.get(lane_index) else {
                        continue;
                    };
                    for (spot_index, band) in bands.iter().enumerate() {
                        if !band.visible {
                            continue;
                        }
                        let local_center = Point::new(lane_x, gel_band_y(&geometry, band.value));
                        let center = crate::rotate_point_around(
                            local_center,
                            geometry.center,
                            geometry.rotate,
                        );
                        let distance = center.distance(point);
                        if distance > band.width.max(band.height) * 0.5 + px_to_pt(6.0) {
                            continue;
                        }
                        let hit = TlcSpotHit {
                            object_id: object.id.clone(),
                            lane_index,
                            spot_index,
                            rf: round2(band.value),
                            value_kind: "band-value".to_string(),
                            center,
                            guide_points: gel_lane_guide_points(&geometry, lane_index),
                        };
                        match &best {
                            Some((best_distance, _)) if *best_distance <= distance => {}
                            _ => best = Some((distance, hit)),
                        }
                    }
                }
            }
        }
        best.map(|(_, hit)| hit)
    }

    pub fn begin_tlc_spot_drag(&mut self, point: Point) -> Option<TlcSpotHit> {
        let hit = self.tlc_spot_hit_test(point)?;
        self.tlc_spot_drag = Some(TlcSpotDragState {
            initial_rf: hit.rf,
            hit: hit.clone(),
            changed: false,
            undo_pushed: false,
        });
        Some(hit)
    }

    pub fn update_tlc_spot_drag(&mut self, point: Point) -> Option<TlcSpotHit> {
        let command = self.tlc_spot_drag_command()?;
        let mut next = None;
        self.with_transient_command(command, |engine| {
            next = engine.update_tlc_spot_drag_untracked(point);
            next.is_some()
        });
        next
    }

    pub(super) fn update_tlc_spot_drag_untracked(&mut self, point: Point) -> Option<TlcSpotHit> {
        let drag = self.tlc_spot_drag.clone()?;
        let next_rf = self.tlc_spot_rf_at_point(&drag.hit.object_id, drag.hit.lane_index, point)?;
        let changed = (drag.hit.rf - next_rf).abs() > 0.0001;
        if changed && !drag.undo_pushed {
            self.push_undo_snapshot();
        }
        let next = self.update_tlc_spot_to_point(
            &drag.hit.object_id,
            drag.hit.lane_index,
            drag.hit.spot_index,
            point,
        )?;
        if let Some(active_drag) = &mut self.tlc_spot_drag {
            active_drag.changed |= changed;
            active_drag.undo_pushed |= changed;
            active_drag.hit = next.clone();
        }
        Some(next)
    }

    pub fn finish_tlc_spot_drag(&mut self, point: Point) -> Option<TlcSpotHit> {
        let had_drag = self.tlc_spot_drag.is_some();
        let next = if had_drag {
            self.update_tlc_spot_drag(point)
        } else {
            None
        };
        let changed = self.tlc_spot_drag.as_ref().is_some_and(|drag| drag.changed);
        let undo_pushed = self
            .tlc_spot_drag
            .as_ref()
            .is_some_and(|drag| drag.undo_pushed);
        self.tlc_spot_drag = None;
        if had_drag && undo_pushed && !changed {
            self.undo_stack.pop();
        }
        next
    }

    pub(super) fn tlc_spot_drag_command(&self) -> Option<EditorCommand> {
        let drag = self.tlc_spot_drag.as_ref()?;
        Some(EditorCommand::MoveChromatographyMark {
            object_id: drag.hit.object_id.clone(),
            lane_index: drag.hit.lane_index,
            spot_index: drag.hit.spot_index,
            before_value: drag.initial_rf,
        })
    }

    pub fn tlc_lane_guide_hit_test(&self, point: Point) -> Option<TlcSpotHit> {
        if self.tlc_spot_hit_test(point).is_some() {
            return None;
        }
        for object in self.state.document.scene_objects() {
            if let Some(geometry) = tlc_plate_geometry(object) {
                for (lane_index, spots) in geometry.spots.iter().enumerate() {
                    let guide_points = tlc_lane_guide_points(&geometry, lane_index);
                    if !point_in_polygon(point, &guide_points) {
                        continue;
                    }
                    let rf = spots.first().copied().unwrap_or(0.15);
                    let lane_x = *geometry.lane_centers.get(lane_index)?;
                    let local_center = Point::new(
                        lane_x,
                        geometry.origin_y - (geometry.origin_y - geometry.solvent_y) * rf,
                    );
                    return Some(TlcSpotHit {
                        object_id: object.id.clone(),
                        lane_index,
                        spot_index: 0,
                        rf: round2(rf),
                        value_kind: "rf".to_string(),
                        center: crate::rotate_point_around(
                            local_center,
                            geometry.center,
                            geometry.rotate,
                        ),
                        guide_points,
                    });
                }
            }
            if let Some(geometry) = gel_plate_geometry(object) {
                for (lane_index, bands) in geometry.bands.iter().enumerate() {
                    let guide_points = gel_lane_guide_points(&geometry, lane_index);
                    if !point_in_polygon(point, &guide_points) {
                        continue;
                    }
                    let value = bands
                        .iter()
                        .find(|band| band.visible)
                        .map(|band| band.value)
                        .unwrap_or((geometry.start_range + geometry.end_range) * 0.5);
                    let local_center = Point::new(
                        *geometry.lane_centers.get(lane_index)?,
                        gel_band_y(&geometry, value),
                    );
                    return Some(TlcSpotHit {
                        object_id: object.id.clone(),
                        lane_index,
                        spot_index: 0,
                        rf: round2(value),
                        value_kind: "band-value".to_string(),
                        center: crate::rotate_point_around(
                            local_center,
                            geometry.center,
                            geometry.rotate,
                        ),
                        guide_points,
                    });
                }
            }
        }
        None
    }

    pub(super) fn update_tlc_spot_to_point(
        &mut self,
        object_id: &str,
        lane_index: usize,
        spot_index: usize,
        point: Point,
    ) -> Option<TlcSpotHit> {
        let object = self.state.document.find_scene_object_mut(object_id)?;
        if object.payload.gel_electrophoresis.is_some() {
            let geometry = gel_plate_geometry(object)?;
            let local_point = crate::rotate_point_around(point, geometry.center, -geometry.rotate);
            let fraction = ((local_point.y - geometry.top) / geometry.height).clamp(0.0, 1.0);
            let value = match geometry.axis_direction {
                GelAxisDirection::HigherAtTop => {
                    geometry.start_range + geometry.range * (1.0 - fraction)
                }
                GelAxisDirection::HigherAtBottom => {
                    geometry.start_range + geometry.range * fraction
                }
            };
            let lane_x = *geometry.lane_centers.get(lane_index)?;
            object
                .payload
                .gel_electrophoresis
                .as_mut()?
                .lanes
                .get_mut(lane_index)?
                .bands
                .get_mut(spot_index)?
                .value = round2(value);
            let local_center = Point::new(lane_x, gel_band_y(&geometry, value));
            return Some(TlcSpotHit {
                object_id: object_id.to_string(),
                lane_index,
                spot_index,
                rf: round2(value),
                value_kind: "band-value".to_string(),
                center: crate::rotate_point_around(local_center, geometry.center, geometry.rotate),
                guide_points: gel_lane_guide_points(&geometry, lane_index),
            });
        }
        let geometry = tlc_plate_geometry(object)?;
        let local_point = crate::rotate_point_around(point, geometry.center, -geometry.rotate);
        let denominator = (geometry.origin_y - geometry.solvent_y).abs();
        if denominator <= crate::EPSILON {
            return None;
        }
        let rf = ((geometry.origin_y - local_point.y) / (geometry.origin_y - geometry.solvent_y))
            .clamp(0.0, 1.0);
        let lanes = object.payload.extra.get_mut("lanes")?.as_array_mut()?;
        let lane = lanes.get_mut(lane_index)?.as_object_mut()?;
        let spots = lane.get_mut("spots")?.as_array_mut()?;
        let spot = spots.get_mut(spot_index)?.as_object_mut()?;
        spot.insert("rf".to_string(), json!(round2(rf)));
        let lane_x = *geometry.lane_centers.get(lane_index)?;
        let local_center = Point::new(
            lane_x,
            geometry.origin_y - (geometry.origin_y - geometry.solvent_y) * rf,
        );
        Some(TlcSpotHit {
            object_id: object_id.to_string(),
            lane_index,
            spot_index,
            rf: round2(rf),
            value_kind: "rf".to_string(),
            center: crate::rotate_point_around(local_center, geometry.center, geometry.rotate),
            guide_points: tlc_lane_guide_points(&geometry, lane_index),
        })
    }

    pub(super) fn tlc_spot_rf_at_point(
        &self,
        object_id: &str,
        lane_index: usize,
        point: Point,
    ) -> Option<f64> {
        let object = self.state.document.find_scene_object(object_id)?;
        if let Some(geometry) = gel_plate_geometry(object) {
            let local_point = crate::rotate_point_around(point, geometry.center, -geometry.rotate);
            geometry.lane_centers.get(lane_index)?;
            let fraction = ((local_point.y - geometry.top) / geometry.height).clamp(0.0, 1.0);
            let value = match geometry.axis_direction {
                GelAxisDirection::HigherAtTop => {
                    geometry.start_range + geometry.range * (1.0 - fraction)
                }
                GelAxisDirection::HigherAtBottom => {
                    geometry.start_range + geometry.range * fraction
                }
            };
            return Some(round2(value));
        }
        let geometry = tlc_plate_geometry(object)?;
        let local_point = crate::rotate_point_around(point, geometry.center, -geometry.rotate);
        let denominator = (geometry.origin_y - geometry.solvent_y).abs();
        if denominator <= crate::EPSILON {
            return None;
        }
        geometry.lane_centers.get(lane_index)?;
        Some(round2(
            ((geometry.origin_y - local_point.y) / (geometry.origin_y - geometry.solvent_y))
                .clamp(0.0, 1.0),
        ))
    }
}

#[derive(Debug, Clone)]
struct GelBandGeometry {
    value: f64,
    width: f64,
    height: f64,
    visible: bool,
}

#[derive(Debug, Clone)]
struct GelPlateGeometry {
    center: Point,
    rotate: f64,
    left: f64,
    right: f64,
    top: f64,
    height: f64,
    start_range: f64,
    end_range: f64,
    range: f64,
    axis_direction: GelAxisDirection,
    lane_centers: Vec<f64>,
    bands: Vec<Vec<GelBandGeometry>>,
}

fn gel_plate_geometry(object: &SceneObject) -> Option<GelPlateGeometry> {
    let [x, y, width, height] = object.payload.bbox?;
    let gel = object.payload.gel_electrophoresis.as_ref()?;
    if width <= crate::EPSILON || height <= crate::EPSILON {
        return None;
    }
    let left = object.transform.translate[0] + x;
    let top = object.transform.translate[1] + y;
    let range = gel.end_range - gel.start_range;
    if range.abs() <= crate::EPSILON {
        return None;
    }
    let lane_centers = (0..gel.lanes.len())
        .map(|index| left + width * (index as f64 + 1.0) / (gel.lanes.len() as f64 + 1.0))
        .collect();
    let bands = gel
        .lanes
        .iter()
        .map(|lane| {
            if !lane.visible {
                return Vec::new();
            }
            lane.bands
                .iter()
                .map(|band| GelBandGeometry {
                    value: band.value,
                    width: band.width,
                    height: band.height,
                    visible: band.visible,
                })
                .collect()
        })
        .collect();
    Some(GelPlateGeometry {
        center: Point::new(left + width * 0.5, top + height * 0.5),
        rotate: object.transform.rotate,
        left,
        right: left + width,
        top,
        height,
        start_range: gel.start_range,
        end_range: gel.end_range,
        range,
        axis_direction: match gel.unit_id {
            0..=2 => GelAxisDirection::HigherAtTop,
            3 => GelAxisDirection::HigherAtBottom,
            _ => return None,
        },
        lane_centers,
        bands,
    })
}

fn gel_band_y(geometry: &GelPlateGeometry, value: f64) -> f64 {
    let fraction = ((value - geometry.start_range) / geometry.range).clamp(0.0, 1.0);
    match geometry.axis_direction {
        GelAxisDirection::HigherAtTop => geometry.top + geometry.height * (1.0 - fraction),
        GelAxisDirection::HigherAtBottom => geometry.top + geometry.height * fraction,
    }
}

#[derive(Debug, Clone, Copy)]
enum GelAxisDirection {
    HigherAtTop,
    HigherAtBottom,
}

fn gel_lane_guide_points(geometry: &GelPlateGeometry, lane_index: usize) -> Vec<Point> {
    let Some(&lane_x) = geometry.lane_centers.get(lane_index) else {
        return Vec::new();
    };
    let left = if lane_index == 0 {
        (geometry.left + lane_x) * 0.5
    } else {
        (geometry.lane_centers[lane_index - 1] + lane_x) * 0.5
    };
    let right = if lane_index + 1 == geometry.lane_centers.len() {
        (lane_x + geometry.right) * 0.5
    } else {
        (lane_x + geometry.lane_centers[lane_index + 1]) * 0.5
    };
    [
        Point::new(left, geometry.top),
        Point::new(right, geometry.top),
        Point::new(right, geometry.top + geometry.height),
        Point::new(left, geometry.top + geometry.height),
    ]
    .into_iter()
    .map(|point| crate::rotate_point_around(point, geometry.center, geometry.rotate))
    .collect()
}
