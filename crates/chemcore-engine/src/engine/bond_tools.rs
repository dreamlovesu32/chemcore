use super::*;

fn refresh_attached_label_geometry_for_bond_endpoints(
    fragment: &mut crate::MoleculeFragment,
    object_translate: [f64; 2],
    stroke_width: f64,
    begin_id: &str,
    end_id: &str,
) {
    refresh_attached_node_label_geometry_for_node(
        fragment,
        object_translate,
        begin_id,
        stroke_width,
    );
    if end_id != begin_id {
        refresh_attached_node_label_geometry_for_node(
            fragment,
            object_translate,
            end_id,
            stroke_width,
        );
    }
}

impl Engine {
    pub fn text_format_icon_svg(kind: &str) -> String {
        let runs = match kind {
            "bold" => vec![text_icon_run("B", 16.0, Some(700), None, false, "normal")],
            "italic" => vec![text_icon_run(
                "I",
                16.0,
                Some(400),
                Some("italic"),
                false,
                "normal",
            )],
            "underline" => vec![text_icon_run("U", 16.0, Some(400), None, true, "normal")],
            "chemical" => vec![
                text_icon_run("CH", 14.0, Some(400), None, false, "normal"),
                text_icon_run("2", 14.0, Some(400), None, false, "subscript"),
            ],
            "subscript" => vec![
                text_icon_run("X", 16.0, Some(400), None, false, "normal"),
                text_icon_run("2", 16.0, Some(400), None, false, "subscript"),
            ],
            "superscript" => vec![
                text_icon_run("X", 16.0, Some(400), None, false, "normal"),
                text_icon_run("2", 16.0, Some(400), None, false, "superscript"),
            ],
            _ => return String::new(),
        };
        let fallback_font_size = runs
            .first()
            .and_then(|run| run.font_size)
            .unwrap_or(crate::DEFAULT_TEXT_FONT_SIZE_PT);
        let y = if kind == "superscript" { 16.8 } else { 17.0 };
        let primitive = crate::RenderPrimitive::Text {
            role: crate::RenderRole::DocumentText,
            object_id: Some("__text_format_icon".to_string()),
            node_id: None,
            x: 12.0,
            y,
            baseline_offset: Some(fallback_font_size * 0.82),
            dominant_baseline: None,
            text: String::new(),
            font_size: fallback_font_size,
            font_family: Some("Arial".to_string()),
            fill: Some("#000000".to_string()),
            text_anchor: Some("middle".to_string()),
            line_height: None,
            preserve_lines: false,
            box_width: None,
            runs,
            rotate: 0.0,
            rotate_center: None,
        };
        let class_name = if matches!(kind, "chemical" | "subscript" | "superscript") {
            "chemcore-icon cc-text-format-icon cc-script-icon"
        } else {
            "chemcore-icon cc-text-format-icon"
        };
        crate::primitives_to_svg_viewbox(&[primitive], [0.0, 0.0, 24.0, 24.0], Some(class_name))
            .replace("#000000", "currentColor")
    }

    pub fn bond_tool_icon_svg(variant: BondVariant, stroke_width: f64, bold_width: f64) -> String {
        let mut engine = Engine::new();
        engine.set_document_style_preset(ACS_DOCUMENT_1996_PRESET);
        let icon_stroke_width = stroke_width.max(0.1);
        let geometry_stroke_width = crate::DEFAULT_BOND_STROKE.max(0.1);
        engine.options.bond_stroke_width = if variant == BondVariant::Wavy {
            geometry_stroke_width
        } else {
            icon_stroke_width
        };
        engine.options.graphic_stroke_width = icon_stroke_width;
        engine.options.bold_bond_width = bold_width.max(engine.options.bond_stroke_width);
        let mut tool = engine.state.tool.clone();
        tool.active_tool = Tool::Bond;
        tool.bond_variant = variant;
        engine.set_tool_state(tool);

        let bond_length = engine.options.bond_length_world_pt().value();
        let angle = std::f64::consts::PI - std::f64::consts::FRAC_PI_6;
        let half_axis = Point::new(
            angle.cos() * bond_length * 0.5,
            angle.sin() * bond_length * 0.5,
        );
        let center = Point::new(12.0, 12.0);
        let anchor = BondAnchor {
            node_id: None,
            point: Point::new(center.x - half_axis.x, center.y - half_axis.y),
            label_anchor: None,
        };
        let end = BondAnchor {
            node_id: None,
            point: Point::new(center.x + half_axis.x, center.y + half_axis.y),
            label_anchor: None,
        };
        let order = match variant {
            BondVariant::Double | BondVariant::DashedDouble => 2,
            BondVariant::Triple => 3,
            _ => 1,
        };
        let Some(document) = engine.document_with_preview_bond(&anchor, &end, order) else {
            return String::new();
        };
        let mut primitives = crate::render_document(&document);
        if variant == BondVariant::Wavy {
            for primitive in &mut primitives {
                if let crate::RenderPrimitive::Path {
                    bond_id: Some(bond_id),
                    stroke_width,
                    ..
                } = primitive
                {
                    if bond_id == "__preview_bond" {
                        *stroke_width = icon_stroke_width;
                    }
                }
            }
        }
        crate::primitives_to_svg_viewbox(
            &primitives,
            [0.0, 0.0, 24.0, 24.0],
            Some("chemcore-icon cc-bond-icon"),
        )
        .replace("#000000", "currentColor")
    }

    pub(super) fn document_with_preview_bond(
        &self,
        anchor: &BondAnchor,
        end: &BondAnchor,
        order: u8,
    ) -> Option<ChemcoreDocument> {
        let mut document = self.state.document.clone();
        if let (Some(begin_id), Some(end_id)) = (&anchor.node_id, &end.node_id) {
            if begin_id == end_id || self.bond_exists_in_document(&document, begin_id, end_id) {
                return None;
            }
        }
        let mut entry = document.editable_fragment_mut()?;
        let begin_id = match &anchor.node_id {
            Some(node_id) => node_id.clone(),
            None => {
                let local = entry.local_point(anchor.point);
                let node_id = "__preview_node_begin".to_string();
                entry
                    .fragment
                    .nodes
                    .push(crate::Node::carbon(node_id.clone(), local));
                node_id
            }
        };
        let end_id = match &end.node_id {
            Some(node_id) => node_id.clone(),
            None => {
                let local = entry.local_point(end.point);
                let node_id = "__preview_node_end".to_string();
                entry
                    .fragment
                    .nodes
                    .push(crate::Node::carbon(node_id.clone(), local));
                node_id
            }
        };
        if begin_id == end_id || self.bond_exists_in_fragment(entry.fragment, &begin_id, &end_id) {
            return None;
        }
        entry.fragment.bonds.push(Bond {
            id: "__preview_bond".to_string(),
            begin: begin_id.clone(),
            end: end_id.clone(),
            order: order.max(1),
            double: self.pending_double_state_for_new_bond(&begin_id, &end_id, order.max(1)),
            stereo: self.pending_bond_stereo(),
            stroke_width: self.options.bond_stroke_world_pt().value(),
            stroke: None,
            bold_width: Some(self.options.bold_bond_width_world_pt().value()),
            wedge_width: Some(self.options.wedge_width_world_pt().value()),
            label_clip_margin: Some(self.options.label_clip_margin_world_pt().value()),
            hash_spacing: Some(self.options.hash_spacing_world_pt().value()),
            bond_spacing: Some(self.options.bond_spacing_percent()),
            margin_width: Some(self.options.margin_width_world_pt().value()),
            line_styles: self.pending_line_styles(),
            line_weights: self.pending_line_weights(),
            meta: serde_json::Value::Null,
        });
        update_terminal_double_bond_placement_after_new_attachment(
            entry.fragment,
            &begin_id,
            "__preview_bond",
        );
        update_terminal_double_bond_placement_after_new_attachment(
            entry.fragment,
            &end_id,
            "__preview_bond",
        );
        refresh_attached_label_geometry_for_bond_endpoints(
            entry.fragment,
            entry.object.transform.translate,
            self.options.bond_stroke_world_pt().value(),
            &begin_id,
            &end_id,
        );
        entry.update_bounds();
        Some(document)
    }

    pub fn cycle_bond_center_style(&mut self, bond_id: &str) -> bool {
        self.with_command(
            EditorCommand::CycleBondStyle {
                bond_id: bond_id.to_string(),
                variant: self.state.tool.bond_variant,
            },
            |engine| engine.cycle_bond_center_style_untracked(bond_id),
        )
    }

    pub(super) fn cycle_bond_center_style_untracked(&mut self, bond_id: &str) -> bool {
        let (current_order, was_double_before) = self
            .state
            .document
            .editable_fragment()
            .and_then(|entry| entry.fragment.bonds.iter().find(|bond| bond.id == bond_id))
            .map(|bond| (bond.order, bond.order == 2 && bond.double.is_some()))
            .unwrap_or((1, false));
        let default_side = self
            .preferred_double_bond_side(bond_id)
            .unwrap_or(DoubleBondPlacement::Right);
        let default_placement =
            if current_order == 1 && self.should_default_center_double_bond(bond_id) {
                DoubleBondPlacement::Center
            } else {
                default_side
            };
        let should_freeze_after_change = was_double_before;
        self.push_undo_snapshot();
        let Some(mut entry) = self.state.document.editable_fragment_mut() else {
            self.undo_stack.pop();
            return false;
        };
        let Some(bond) = entry
            .fragment
            .bonds
            .iter_mut()
            .find(|bond| bond.id == bond_id)
        else {
            self.undo_stack.pop();
            return false;
        };
        let changed = match self.state.tool.bond_variant {
            BondVariant::Single => apply_single_tool_center_style(bond, default_placement),
            BondVariant::Double => apply_double_tool_center_style(bond, default_placement),
            BondVariant::Triple => replace_with_plain_triple_bond_style(bond),
            BondVariant::Dashed => cycle_dashed_bond_center_style(bond, default_placement),
            BondVariant::DashedDouble => {
                cycle_dashed_double_bond_tool_center_style(bond, default_placement)
            }
            BondVariant::Bold => cycle_bold_bond_center_style(bond, default_placement),
            BondVariant::BoldDashed => replace_with_bold_dashed_bond_style(bond),
            BondVariant::Wavy => replace_with_plain_wavy_bond_style(bond),
            BondVariant::Wedge | BondVariant::HashedWedge | BondVariant::HollowWedge => {
                replace_with_stereo_bond_style(bond, self.state.tool.bond_variant)
            }
        };
        if !changed {
            self.undo_stack.pop();
            return false;
        }
        if let Some(double) = bond.double.as_mut() {
            double.frozen = should_freeze_after_change;
        }
        let Some((begin_id, end_id)) = entry
            .fragment
            .bonds
            .iter()
            .find(|bond| bond.id == bond_id)
            .map(|bond| (bond.begin.clone(), bond.end.clone()))
        else {
            self.undo_stack.pop();
            return false;
        };
        refresh_attached_label_geometry_for_bond_endpoints(
            entry.fragment,
            entry.object.transform.translate,
            self.options.bond_stroke_world_pt().value(),
            &begin_id,
            &end_id,
        );
        entry.update_bounds();
        self.state.selection = SelectionState::default();
        self.clear_interaction();
        self.note_pending_select_target(PendingSelectTarget::MoleculeBond(bond_id.to_string()));
        true
    }
}

fn text_icon_run(
    text: &str,
    font_size: f64,
    font_weight: Option<u32>,
    font_style: Option<&str>,
    underline: bool,
    script: &str,
) -> crate::LabelRun {
    crate::LabelRun {
        text: text.to_string(),
        font_family: Some("Arial".to_string()),
        font_size: Some(font_size),
        fill: Some("#000000".to_string()),
        font_weight,
        font_style: font_style.map(ToString::to_string),
        underline: Some(underline),
        script: Some(script.to_string()),
    }
}
