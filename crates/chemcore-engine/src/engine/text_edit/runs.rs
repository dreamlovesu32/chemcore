use super::*;

pub(super) fn payload_text(payload: &crate::ObjectPayload) -> String {
    payload
        .extra
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

pub(super) fn payload_box(payload: &crate::ObjectPayload) -> Option<[f64; 4]> {
    payload
        .extra
        .get("box")
        .cloned()
        .and_then(|value| serde_json::from_value::<[f64; 4]>(value).ok())
}

pub(super) fn payload_runs_or_text(payload: &crate::ObjectPayload) -> Vec<LabelRun> {
    if let Some(value) = payload.extra.get("runs").cloned() {
        if let Ok(runs) = serde_json::from_value::<Vec<LabelRun>>(value) {
            if !runs.is_empty() {
                return runs;
            }
        }
    }
    let text = payload_text(payload);
    if text.is_empty() {
        Vec::new()
    } else {
        vec![LabelRun {
            text,
            font_family: payload
                .extra
                .get("fontFamily")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            font_size: payload.extra.get("fontSize").and_then(Value::as_f64),
            fill: payload
                .extra
                .get("fill")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            font_weight: Some(400),
            font_style: Some("normal".to_string()),
            underline: Some(false),
            script: Some("normal".to_string()),
        }]
    }
}

pub(super) fn runs_text(runs: &[LabelRun]) -> String {
    runs.iter().map(|run| run.text.as_str()).collect()
}

pub(super) fn normalize_source_runs(session: &TextEditSession, text: &str) -> Vec<LabelRun> {
    let source_runs = if !session.source_runs.is_empty() {
        session.source_runs.clone()
    } else if text.is_empty() {
        Vec::new()
    } else {
        vec![LabelRun {
            text: text.to_string(),
            font_family: session.font_family.clone(),
            font_size: session.font_size,
            fill: session.fill.clone(),
            font_weight: Some(400),
            font_style: Some("normal".to_string()),
            underline: Some(false),
            script: Some(if session.default_chemical {
                "chemical".to_string()
            } else {
                "normal".to_string()
            }),
        }]
    };
    source_runs
        .into_iter()
        .filter(|run| !run.text.is_empty())
        .map(|mut run| {
            if run.font_family.is_none() {
                run.font_family = session.font_family.clone();
            }
            if run.font_size.is_none() {
                run.font_size = session.font_size;
            }
            if run.fill.is_none() {
                run.fill = session.fill.clone();
            }
            if run.font_weight.is_none() {
                run.font_weight = Some(400);
            }
            if run.font_style.is_none() {
                run.font_style = Some("normal".to_string());
            }
            if run.underline.is_none() {
                run.underline = Some(false);
            }
            if run.script.is_none() {
                run.script = Some(if session.default_chemical {
                    "chemical".to_string()
                } else {
                    "normal".to_string()
                });
            }
            run
        })
        .collect()
}

pub(super) fn source_runs_are_chemical(source_runs: &[LabelRun]) -> bool {
    source_runs
        .iter()
        .any(|run| run.script.as_deref() == Some("chemical"))
}

pub(super) fn display_runs_from_source_runs(
    source_runs: &[LabelRun],
    fallback_font_family: &str,
    fallback_font_size: f64,
    fallback_fill: &str,
) -> Vec<LabelRun> {
    let mut out = Vec::new();
    for run in source_runs {
        if run.text.is_empty() {
            continue;
        }
        let base = LabelRun {
            text: String::new(),
            font_family: Some(
                run.font_family
                    .clone()
                    .unwrap_or_else(|| fallback_font_family.to_string()),
            ),
            font_size: Some(run.font_size.unwrap_or(fallback_font_size)),
            fill: Some(
                run.fill
                    .clone()
                    .unwrap_or_else(|| fallback_fill.to_string()),
            ),
            font_weight: Some(run.font_weight.unwrap_or(400)),
            font_style: Some(
                run.font_style
                    .clone()
                    .unwrap_or_else(|| "normal".to_string()),
            ),
            underline: Some(run.underline.unwrap_or(false)),
            script: Some("normal".to_string()),
        };
        match run.script.as_deref().unwrap_or("normal") {
            "chemical" => out.extend(expand_chemical_run(&base, &run.text)),
            "subscript" | "superscript" => {
                let mut next = base.clone();
                next.text = run.text.clone();
                next.script = run.script.clone();
                out.push(next);
            }
            _ => {
                let mut next = base.clone();
                next.text = run.text.clone();
                out.push(next);
            }
        }
    }
    merge_adjacent_runs(out)
}

pub(super) fn merge_adjacent_runs(runs: Vec<LabelRun>) -> Vec<LabelRun> {
    let mut merged: Vec<LabelRun> = Vec::new();
    for run in runs {
        if let Some(previous) = merged.last_mut() {
            if previous.font_family == run.font_family
                && previous.font_size == run.font_size
                && previous.fill == run.fill
                && previous.font_weight == run.font_weight
                && previous.font_style == run.font_style
                && previous.script == run.script
            {
                previous.text.push_str(&run.text);
                continue;
            }
        }
        merged.push(run);
    }
    merged
}

pub(super) fn expand_chemical_run(base: &LabelRun, text: &str) -> Vec<LabelRun> {
    let chars: Vec<char> = text.chars().collect();
    let mut scripts = vec!["normal"; chars.len()];

    for index in 0..chars.len() {
        let character = chars[index];
        if character.is_ascii_digit()
            && index > 0
            && (chars[index - 1].is_ascii_alphabetic() || chars[index - 1] == ')')
        {
            scripts[index] = "subscript";
        }
        if matches!(character, '+' | '-') {
            scripts[index] = "superscript";
            if index > 0 && chars[index - 1].is_ascii_digit() {
                let previous_index = index - 1;
                if previous_index > 0 && !chars[previous_index - 1].is_whitespace() {
                    scripts[previous_index] = "superscript";
                }
            }
        }
    }

    let mut out = Vec::new();
    let mut buffer = String::new();
    let mut active_script = "normal";
    for (index, character) in chars.into_iter().enumerate() {
        let script = scripts[index];
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
    out
}
