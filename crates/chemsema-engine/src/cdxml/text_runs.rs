use crate::LabelRun;
use std::collections::BTreeMap;

use super::colors::CdxmlColorTable;
use super::round2;

#[derive(Default)]
pub(super) struct CdxmlFontRunState {
    font_id: Option<String>,
}

impl CdxmlFontRunState {
    pub(super) fn resolve(&mut self, explicit_font_id: Option<&str>) -> &str {
        if let Some(font_id) = explicit_font_id {
            self.font_id = Some(font_id.to_string());
        }
        // ChemDraw's styled-string font state starts at Arial independently
        // of LabelFont/CaptionFont. A missing font on a later <s> retains the
        // preceding <s> font; a font attribute on <t> is not part of that
        // state. An empty ID deliberately reaches label_source_run's Arial
        // rule without inventing a font-table entry.
        self.font_id.as_deref().unwrap_or("")
    }
}

pub(super) fn label_source_run(
    text: &str,
    face: u32,
    font_id: &str,
    color_id: &str,
    font_size: f64,
    colors: &CdxmlColorTable,
    fonts: &BTreeMap<String, String>,
) -> LabelRun {
    let decoded_face = decode_cdxml_face(face);
    let source_family = fonts.get(font_id).map(String::as_str).unwrap_or("Arial");
    let (font_family, family_bold, family_italic) = split_cdxml_font_family_style(source_family);
    LabelRun {
        text: text.to_string(),
        font_family: Some(font_family),
        font_size: Some(round2(font_size)),
        fill: Some(colors.resolve(Some(color_id))),
        font_weight: Some(if decoded_face.bold || family_bold {
            700
        } else {
            400
        }),
        font_style: Some(
            if decoded_face.italic || family_italic {
                "italic"
            } else {
                "normal"
            }
            .to_string(),
        ),
        underline: Some(decoded_face.underline),
        outline: Some(decoded_face.outline),
        shadow: Some(decoded_face.shadow),
        script: Some(decoded_face.script.to_string()),
    }
}

fn split_cdxml_font_family_style(name: &str) -> (String, bool, bool) {
    let lower = name.to_ascii_lowercase();
    for (suffix, bold, italic) in [
        (" bold italic", true, true),
        (" bold oblique", true, true),
        (" italic", false, true),
        (" oblique", false, true),
        (" bold", true, false),
    ] {
        if lower.ends_with(suffix) {
            let family = name[..name.len() - suffix.len()].trim_end();
            if !family.is_empty() {
                return (family.to_string(), bold, italic);
            }
        }
    }
    (name.to_string(), false, false)
}

pub(super) fn label_display_runs(
    text: &str,
    face: u32,
    font_id: &str,
    color_id: &str,
    font_size: f64,
    colors: &CdxmlColorTable,
    fonts: &BTreeMap<String, String>,
) -> Vec<LabelRun> {
    let source = label_source_run(text, face, font_id, color_id, font_size, colors, fonts);
    if source.script.as_deref() == Some("chemical") {
        expand_cdxml_chemical_run(&source)
    } else {
        vec![source]
    }
}

pub(super) fn label_display_runs_from_source_runs(source_runs: &[LabelRun]) -> Vec<LabelRun> {
    expand_cdxml_mixed_runs(source_runs)
}

struct CdxmlFace {
    bold: bool,
    italic: bool,
    underline: bool,
    outline: bool,
    shadow: bool,
    script: &'static str,
}

fn decode_cdxml_face(face: u32) -> CdxmlFace {
    let has_subscript = face & 32 != 0;
    let has_superscript = face & 64 != 0;
    let script = match (has_subscript, has_superscript) {
        (true, true) => "chemical",
        (true, false) => "subscript",
        (false, true) => "superscript",
        (false, false) => "normal",
    };
    CdxmlFace {
        bold: face & 1 != 0,
        italic: face & 2 != 0,
        underline: face & 4 != 0,
        outline: face & 8 != 0,
        shadow: face & 16 != 0,
        script,
    }
}

fn expand_cdxml_chemical_run(base: &LabelRun) -> Vec<LabelRun> {
    expand_cdxml_chemical_runs(std::slice::from_ref(base))
}

fn expand_cdxml_chemical_runs(base_runs: &[LabelRun]) -> Vec<LabelRun> {
    expand_cdxml_mixed_runs(base_runs)
}

fn expand_cdxml_mixed_runs(base_runs: &[LabelRun]) -> Vec<LabelRun> {
    let scripts = crate::chemical_text_runs::infer_display_scripts(base_runs);

    let mut out = Vec::new();
    let mut char_index = 0;
    for base in base_runs {
        let mut buffer = String::new();
        let authored_script = base.script.as_deref().unwrap_or("normal");
        let mut active_script = authored_script;
        for character in base.text.chars() {
            let script = if authored_script == "chemical" {
                scripts[char_index]
            } else {
                authored_script
            };
            char_index += 1;
            if !buffer.is_empty() && script != active_script {
                let mut run = base.clone();
                run.text = std::mem::take(&mut buffer);
                run.script = Some(active_script.to_string());
                out.push(run);
            }
            active_script = script;
            buffer.push(character);
        }
        if !buffer.is_empty() {
            let mut run = base.clone();
            run.text = buffer;
            run.script = Some(active_script.to_string());
            out.push(run);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chemical_run(text: &str) -> LabelRun {
        LabelRun {
            text: text.to_string(),
            script: Some("chemical".to_string()),
            ..LabelRun::default()
        }
    }

    #[test]
    fn chemical_runs_keep_internal_hyphen_normal() {
        let runs = expand_cdxml_chemical_run(&chemical_run("t-Bu"));
        assert_eq!(
            runs.iter()
                .map(|run| (run.text.as_str(), run.script.as_deref()))
                .collect::<Vec<_>>(),
            vec![("t-Bu", Some("normal"))]
        );
    }

    #[test]
    fn chemical_runs_keep_terminal_charge_suffix_superscript() {
        let runs = expand_cdxml_chemical_run(&chemical_run("Fe3+"));
        assert_eq!(
            runs.iter()
                .map(|run| (run.text.as_str(), run.script.as_deref()))
                .collect::<Vec<_>>(),
            vec![("Fe", Some("normal")), ("3+", Some("superscript"))]
        );
    }

    #[test]
    fn chemical_runs_treat_charge_before_line_break_as_line_terminal() {
        let runs = expand_cdxml_chemical_run(&chemical_run("H+\nN"));
        assert_eq!(
            runs.iter()
                .map(|run| (run.text.as_str(), run.script.as_deref()))
                .collect::<Vec<_>>(),
            vec![
                ("H", Some("normal")),
                ("+", Some("superscript")),
                ("\nN", Some("normal"))
            ]
        );
    }

    #[test]
    fn chemical_runs_treat_charge_before_species_separator_as_token_terminal() {
        let runs = expand_cdxml_chemical_run(&chemical_run("Et3NH+ Cl-"));
        assert_eq!(
            runs.iter()
                .map(|run| (run.text.as_str(), run.script.as_deref()))
                .collect::<Vec<_>>(),
            vec![
                ("Et", Some("normal")),
                ("3", Some("subscript")),
                ("NH", Some("normal")),
                ("+", Some("superscript")),
                (" Cl", Some("normal")),
                ("-", Some("superscript")),
            ]
        );
    }

    #[test]
    fn chemical_runs_keep_mid_label_hyphen_normal_even_after_digits() {
        let runs = expand_cdxml_chemical_run(&chemical_run("CH3-CH2"));
        assert_eq!(
            runs.iter()
                .map(|run| (run.text.as_str(), run.script.as_deref()))
                .collect::<Vec<_>>(),
            vec![
                ("CH", Some("normal")),
                ("3", Some("subscript")),
                ("-CH", Some("normal")),
                ("2", Some("subscript"))
            ]
        );
    }

    #[test]
    fn chemical_runs_classify_subscripts_across_style_boundaries() {
        let mut formula = chemical_run("OCF");
        formula.fill = Some("#ff0000".to_string());
        let mut digit = chemical_run("3");
        digit.fill = Some("#ff0000".to_string());

        let runs = label_display_runs_from_source_runs(&[formula, digit]);

        assert_eq!(
            runs.iter()
                .map(|run| (
                    run.text.as_str(),
                    run.script.as_deref(),
                    run.fill.as_deref()
                ))
                .collect::<Vec<_>>(),
            vec![
                ("OCF", Some("normal"), Some("#ff0000")),
                ("3", Some("subscript"), Some("#ff0000"))
            ]
        );
    }

    #[test]
    fn label_runs_preserve_explicit_superscript_between_chemical_groups() {
        let runs = label_display_runs_from_source_runs(&[
            chemical_run("Pd"),
            LabelRun {
                text: "IV".to_string(),
                script: Some("superscript".to_string()),
                ..LabelRun::default()
            },
            chemical_run("(OCF"),
            chemical_run("3"),
            chemical_run(")n"),
        ]);

        assert_eq!(
            runs.iter()
                .map(|run| (run.text.as_str(), run.script.as_deref()))
                .collect::<Vec<_>>(),
            vec![
                ("Pd", Some("normal")),
                ("IV", Some("superscript")),
                ("(OCF", Some("normal")),
                ("3", Some("subscript")),
                (")n", Some("normal"))
            ]
        );
    }

    #[test]
    fn explicit_charge_run_does_not_promote_a_formula_count() {
        let runs = label_display_runs_from_source_runs(&[
            chemical_run("NH3"),
            LabelRun {
                text: "+".to_string(),
                script: Some("superscript".to_string()),
                ..LabelRun::default()
            },
        ]);

        assert_eq!(
            runs.iter()
                .map(|run| (run.text.as_str(), run.script.as_deref()))
                .collect::<Vec<_>>(),
            vec![
                ("NH", Some("normal")),
                ("3", Some("subscript")),
                ("+", Some("superscript")),
            ]
        );
    }

    #[test]
    fn chemical_face_digit_uses_neighboring_regular_runs_as_formula_context() {
        let runs = label_display_runs_from_source_runs(&[
            LabelRun {
                text: "(PhO)".to_string(),
                script: Some("normal".to_string()),
                ..LabelRun::default()
            },
            chemical_run("2"),
            LabelRun {
                text: "POH".to_string(),
                script: Some("normal".to_string()),
                ..LabelRun::default()
            },
        ]);

        assert_eq!(
            runs.iter()
                .map(|run| (run.text.as_str(), run.script.as_deref()))
                .collect::<Vec<_>>(),
            vec![
                ("(PhO)", Some("normal")),
                ("2", Some("subscript")),
                ("POH", Some("normal"))
            ]
        );
    }

    #[test]
    fn chemical_runs_style_complete_multi_digit_counts_and_charges() {
        let counts = expand_cdxml_chemical_run(&chemical_run("C10H21O3"));
        assert_eq!(
            counts
                .iter()
                .map(|run| (run.text.as_str(), run.script.as_deref()))
                .collect::<Vec<_>>(),
            vec![
                ("C", Some("normal")),
                ("10", Some("subscript")),
                ("H", Some("normal")),
                ("21", Some("subscript")),
                ("O", Some("normal")),
                ("3", Some("subscript")),
            ]
        );

        let charge = expand_cdxml_chemical_run(&chemical_run("Fe10+"));
        assert_eq!(
            charge
                .iter()
                .map(|run| (run.text.as_str(), run.script.as_deref()))
                .collect::<Vec<_>>(),
            vec![("Fe", Some("normal")), ("10+", Some("superscript"))]
        );
    }

    #[test]
    fn legacy_font_style_suffixes_resolve_to_css_family_and_face() {
        assert_eq!(
            split_cdxml_font_family_style("Arial Bold"),
            ("Arial".to_string(), true, false)
        );
        assert_eq!(
            split_cdxml_font_family_style("Helvetica Bold Oblique"),
            ("Helvetica".to_string(), true, true)
        );
        assert_eq!(
            split_cdxml_font_family_style("Times New Roman"),
            ("Times New Roman".to_string(), false, false)
        );
    }

    #[test]
    fn font_runs_start_at_arial_and_retain_only_preceding_run_state() {
        let mut state = CdxmlFontRunState::default();
        assert_eq!(state.resolve(None), "");
        assert_eq!(state.resolve(Some("3")), "3");
        assert_eq!(state.resolve(None), "3");
        assert_eq!(state.resolve(Some("4")), "4");
        assert_eq!(state.resolve(None), "4");
    }
}
