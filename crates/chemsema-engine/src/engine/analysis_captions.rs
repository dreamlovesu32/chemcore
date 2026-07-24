use super::select::object_selection_bounds_for_render;
use super::text_edit::make_text_object;
use super::{EditorCommand, Engine, TextEditSession, TextEditTarget};
use crate::{LabelRun, LinkEndpoint, LinkPolicy, LinkRelation, SelectionState};
use serde_json::{json, Value};
use std::collections::BTreeSet;

const CAPTION_GAP: f64 = 12.0;
const FONT_SIZE: f64 = 10.0;
const LINE_HEIGHT: f64 = 12.0;

impl Engine {
    pub fn paste_selection_analysis_caption(&mut self, digits: u8) -> bool {
        self.with_command(
            EditorCommand::PasteAnalysisCaption {
                digits: digits.min(8),
            },
            |engine| engine.paste_selection_analysis_caption_untracked(digits.min(8)),
        )
    }

    fn paste_selection_analysis_caption_untracked(&mut self, digits: u8) -> bool {
        let Ok((source, _)) = self.selected_single_molecule_fragment() else {
            return false;
        };
        let source_id = source.id.clone();
        let Some(summary) = self.chemistry_summary_for_molecule_object(&source_id) else {
            return false;
        };
        let Some(bounds) = self
            .state
            .document
            .find_scene_object(&source_id)
            .and_then(|object| object_selection_bounds_for_render(&self.state.document, object))
        else {
            return false;
        };
        let values = analysis_values(&summary, digits);
        let text = analysis_text(&values);
        let width = estimated_caption_width(&text);
        let height = LINE_HEIGHT * 3.0;
        let x = (bounds[0] + bounds[2]) * 0.5 - width * 0.5;
        let y = bounds[3] + CAPTION_GAP;
        let object_id = self.next_id("obj_analysis");
        let relation_id = self.next_id("link");
        let run = LabelRun {
            text: text.clone(),
            font_family: Some("Arial".to_string()),
            font_size: Some(FONT_SIZE),
            fill: Some("#000000".to_string()),
            ..LabelRun::default()
        };
        let session = TextEditSession {
            target: TextEditTarget::TextObject {
                object_id: None,
                x,
                y,
            },
            text: text.clone(),
            source_runs: vec![run.clone()],
            font_family: Some("Arial".to_string()),
            font_size: Some(FONT_SIZE),
            fill: Some("#000000".to_string()),
            align: Some("left".to_string()),
            line_height: Some(LINE_HEIGHT),
            box_value: Some([0.0, 0.0, width, height]),
            anchor_offset: None,
            text_position: None,
            glyph_polygons: Vec::new(),
            preserve_lines: true,
            default_chemical: false,
            display_mode: None,
        };
        let z_index = self
            .state
            .document
            .scene_objects()
            .into_iter()
            .map(|object| object.z_index)
            .max()
            .unwrap_or_default()
            + 1;
        self.push_undo_snapshot();
        let mut object = make_text_object(
            &object_id,
            x,
            y,
            &text,
            vec![run.clone()],
            vec![run],
            &session,
            width,
            height,
            z_index,
        );
        object.name = "analysis-caption".to_string();
        object.link_policy = LinkPolicy::Linked;
        object.payload.extra.insert(
            "analysisCaption".to_string(),
            json!({
                "version": 1,
                "digits": digits,
                "anchorMode": "follow",
                "generatedValues": values,
            }),
        );
        if let Some(source) = self.state.document.find_scene_object_mut(&source_id) {
            source.link_policy = LinkPolicy::Linked;
        }
        self.state.document.objects.push(object);
        self.state.document.links.push(LinkRelation {
            id: relation_id,
            kind: "analysis-caption".to_string(),
            endpoints: vec![
                LinkEndpoint {
                    entity_id: source_id,
                    role: "source".to_string(),
                },
                LinkEndpoint {
                    entity_id: object_id.clone(),
                    role: "caption".to_string(),
                },
            ],
            data: Value::Null,
        });
        self.state.selection = SelectionState {
            text_objects: vec![object_id],
            ..SelectionState::default()
        };
        true
    }

    pub(super) fn refresh_analysis_captions(&mut self) -> bool {
        let pairs = self
            .state
            .document
            .links
            .iter()
            .filter(|relation| relation.kind == "analysis-caption")
            .filter_map(|relation| {
                let source = relation
                    .endpoints
                    .iter()
                    .find(|endpoint| endpoint.role == "source")?;
                let caption = relation
                    .endpoints
                    .iter()
                    .find(|endpoint| endpoint.role == "caption")?;
                Some((source.entity_id.clone(), caption.entity_id.clone()))
            })
            .collect::<Vec<_>>();
        let mut changed = false;
        for (source_id, caption_id) in pairs {
            let Some(summary) = self.chemistry_summary_for_molecule_object(&source_id) else {
                continue;
            };
            let (digits, old_values, anchor_mode) = self
                .state
                .document
                .find_scene_object(&caption_id)
                .and_then(|object| object.payload.extra.get("analysisCaption"))
                .map(|data| {
                    (
                        data.get("digits").and_then(Value::as_u64).unwrap_or(2) as u8,
                        data.get("generatedValues").cloned().unwrap_or(Value::Null),
                        data.get("anchorMode")
                            .and_then(Value::as_str)
                            .unwrap_or("follow")
                            .to_string(),
                    )
                })
                .unwrap_or((2, Value::Null, "fixed".to_string()));
            let values = analysis_values(&summary, digits);
            let old_text = self
                .state
                .document
                .find_scene_object(&caption_id)
                .and_then(|object| object.payload.extra.get("text"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let next_text = replace_generated_values(&old_text, &old_values, &values)
                .unwrap_or_else(|| analysis_text(&values));
            let source_bounds =
                self.state
                    .document
                    .find_scene_object(&source_id)
                    .and_then(|object| {
                        object_selection_bounds_for_render(&self.state.document, object)
                    });
            if let Some(caption) = self.state.document.find_scene_object_mut(&caption_id) {
                if caption.payload.extra.get("text").and_then(Value::as_str)
                    != Some(next_text.as_str())
                {
                    caption
                        .payload
                        .extra
                        .insert("text".to_string(), json!(next_text.clone()));
                    for key in ["runs", "sourceRuns"] {
                        caption.payload.extra.insert(
                            key.to_string(),
                            json!([LabelRun {
                                text: next_text.clone(),
                                font_family: Some("Arial".to_string()),
                                font_size: Some(FONT_SIZE),
                                fill: Some("#000000".to_string()),
                                ..LabelRun::default()
                            }]),
                        );
                    }
                    changed = true;
                }
                if let Some(data) = caption
                    .payload
                    .extra
                    .get_mut("analysisCaption")
                    .and_then(Value::as_object_mut)
                {
                    if data.get("generatedValues") != Some(&values) {
                        data.insert("generatedValues".to_string(), values.clone());
                        changed = true;
                    }
                }
                let width = estimated_caption_width(&next_text);
                let next_box = json!([0.0, 0.0, width, LINE_HEIGHT * 3.0]);
                if caption.payload.extra.get("box") != Some(&next_box) {
                    caption.payload.extra.insert("box".to_string(), next_box);
                    caption.payload.bbox = Some([0.0, 0.0, width, LINE_HEIGHT * 3.0]);
                    changed = true;
                }
                if anchor_mode == "follow" {
                    if let Some(bounds) = source_bounds {
                        let next = [
                            crate::round2((bounds[0] + bounds[2]) * 0.5 - width * 0.5),
                            crate::round2(bounds[3] + CAPTION_GAP),
                        ];
                        if caption.transform.translate != next {
                            caption.transform.translate = next;
                            changed = true;
                        }
                    }
                }
            }
        }
        changed
    }

    pub(super) fn mark_moved_analysis_captions_fixed(&mut self, command: &EditorCommand) {
        let moved = match command {
            EditorCommand::MoveTargets { targets, .. } => targets
                .objects
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            EditorCommand::MoveSelection => self
                .state
                .selection
                .text_objects
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            _ => BTreeSet::new(),
        };
        if moved.is_empty() {
            return;
        }
        let caption_ids = self
            .state
            .document
            .links
            .iter()
            .filter(|relation| relation.kind == "analysis-caption")
            .flat_map(|relation| relation.endpoints.iter())
            .filter(|endpoint| {
                endpoint.role == "caption" && moved.contains(endpoint.entity_id.as_str())
            })
            .map(|endpoint| endpoint.entity_id.clone())
            .collect::<Vec<_>>();
        for caption_id in caption_ids {
            if let Some(data) = self
                .state
                .document
                .find_scene_object_mut(&caption_id)
                .and_then(|object| object.payload.extra.get_mut("analysisCaption"))
                .and_then(Value::as_object_mut)
            {
                data.insert("anchorMode".to_string(), json!("fixed"));
            }
        }
    }

    pub fn take_pending_dialog_json(&mut self) -> String {
        self.pending_dialog
            .take()
            .unwrap_or(Value::Null)
            .to_string()
    }
}

fn analysis_values(
    summary: &super::selection_summary::SelectionChemistrySummary,
    digits: u8,
) -> Value {
    json!({
        "formula": summary.formula,
        "formulaWeight": format!("{:.*}", usize::from(digits), summary.formula_weight),
        "exactMass": format!("{:.*}", usize::from(digits), summary.exact_mass),
    })
}

fn analysis_text(values: &Value) -> String {
    format!(
        "Formula: {}\nFormula Weight: {}\nExact Mass: {}",
        values.get("formula").and_then(Value::as_str).unwrap_or(""),
        values
            .get("formulaWeight")
            .and_then(Value::as_str)
            .unwrap_or(""),
        values
            .get("exactMass")
            .and_then(Value::as_str)
            .unwrap_or("")
    )
}

fn replace_generated_values(text: &str, old: &Value, next: &Value) -> Option<String> {
    let mut result = text.to_string();
    for key in ["formula", "formulaWeight", "exactMass"] {
        let old_value = old.get(key)?.as_str()?;
        let next_value = next.get(key)?.as_str()?;
        let index = result.find(old_value)?;
        result.replace_range(index..index + old_value.len(), next_value);
    }
    Some(result)
}

fn estimated_caption_width(text: &str) -> f64 {
    let max_chars = text
        .lines()
        .map(str::chars)
        .map(Iterator::count)
        .max()
        .unwrap_or(1);
    crate::round2((max_chars as f64 * FONT_SIZE * 0.56).max(80.0))
}
