use super::*;
use crate::protocol::SESSION_PROTOCOL_VERSION;

pub(super) struct SessionDocument {
    input: String,
    engine: Engine,
    document: ChemcoreDocument,
}

impl SessionDocument {
    fn open(input: String) -> Result<Self, String> {
        let engine = load_engine_from_file(&input)?;
        let document = engine_document(&engine)?;
        Ok(Self {
            input,
            engine,
            document,
        })
    }

    fn refresh_document(&mut self) -> Result<(), String> {
        self.document = engine_document(&self.engine)?;
        Ok(())
    }

    fn summary_json(&self) -> Value {
        let fragments = self.document.editable_fragments();
        json!({
            "input": self.input,
            "revision": self.engine.revision(),
            "objects": self.document.objects.len(),
            "molecules": fragments.len(),
            "nodes": fragments.iter().map(|entry| entry.fragment.nodes.len()).sum::<usize>(),
            "bonds": fragments.iter().map(|entry| entry.fragment.bonds.len()).sum::<usize>(),
        })
    }
}

pub(crate) fn session_command(args: &[String]) -> Result<(), String> {
    let mut initial_input = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--input" | "-i" => {
                index += 1;
                initial_input = Some(
                    args.get(index)
                        .ok_or_else(|| "--input requires a path.".to_string())?
                        .clone(),
                );
            }
            value if initial_input.is_none() => initial_input = Some(value.to_string()),
            value => return Err(format!("Unexpected session argument '{value}'.")),
        }
        index += 1;
    }

    let mut session = match initial_input {
        Some(input) => Some(SessionDocument::open(input)?),
        None => None,
    };

    write_session_line(session_ready_json(session.as_ref()))?;
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line.map_err(|error| format!("Failed to read session input: {error}"))?;
        if line.trim().is_empty() {
            continue;
        }
        let request: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(error) => {
                write_session_line(session_error(
                    Value::Null,
                    Value::Null,
                    "invalid_json",
                    format!("Invalid JSON request: {error}"),
                ))?;
                continue;
            }
        };
        let (response, exit) = handle_session_request(&mut session, request);
        write_session_line(response)?;
        if exit {
            break;
        }
    }
    Ok(())
}

pub(super) fn handle_session_request(
    session: &mut Option<SessionDocument>,
    request: Value,
) -> (Value, bool) {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let op_value = request
        .get("op")
        .or_else(|| request.get("operation"))
        .or_else(|| request.get("command"))
        .cloned()
        .unwrap_or(Value::Null);
    let Some(op) = op_value.as_str() else {
        return (
            session_error(
                id,
                op_value,
                "missing_operation",
                "Session request requires op, operation, or command.".to_string(),
            ),
            false,
        );
    };
    let op = op.trim().to_ascii_lowercase();
    let result = match op.as_str() {
        "help" | "capabilities" => Ok(session_help_json()),
        "open" => session_open(session, &request),
        "close" => {
            *session = None;
            Ok(json!({ "closed": true }))
        }
        "targets" => with_session(session, |document| {
            Ok(targets_report(&document.input, &document.document))
        }),
        "detail" | "details" | "describe" | "show" => with_session(session, |document| {
            let target = request_required_target(&request)?;
            let summary_only =
                request_bool(&request, &["summaryOnly", "summary-only", "noRaw"])?.unwrap_or(false);
            let include_resource =
                request_bool(&request, &["includeResource", "include-resource"])?.unwrap_or(false);
            let options = DetailOptions {
                include_raw: !summary_only,
                include_resource,
            };
            detail_report(&document.input, &document.document, &target, options)
        }),
        "context" => with_session(session, |document| session_context(document, &request)),
        "capture" | "screenshot" => {
            with_session(session, |document| session_capture(document, &request))
        }
        "execute" | "run" => {
            with_session_mut(session, |document| session_execute(document, &request))
        }
        "save" => with_session(session, |document| session_save(document, &request)),
        "status" => with_session(session, |document| Ok(document.summary_json())),
        "exit" | "quit" => Ok(json!({ "exiting": true })),
        _ => Err(format!(
            "Unknown session operation '{op}'. Use op=help for supported operations."
        )),
    };

    let exit = matches!(op.as_str(), "exit" | "quit") && result.is_ok();
    match result {
        Ok(result) => (session_ok(id, op, result), exit),
        Err(error) => (
            session_error(id, json!(op), "operation_failed", error),
            false,
        ),
    }
}

pub(super) fn session_open(
    session: &mut Option<SessionDocument>,
    request: &Value,
) -> Result<Value, String> {
    let input = request_required_string(request, &["input", "path", "file"])?;
    let document = SessionDocument::open(input)?;
    let summary = document.summary_json();
    *session = Some(document);
    Ok(summary)
}

pub(super) fn session_context(
    document: &SessionDocument,
    request: &Value,
) -> Result<Value, String> {
    let target = request_required_target(request)?;
    let expansion = request_expansion(request, CropExpansion::uniform_abs(30.0))?;
    let raster = request_raster_options(request)?;
    let limit = request_usize(request, &["limit"])?.unwrap_or(200);
    let target_bounds = target_bounds(&document.document, &target)?;
    let query_view_box = expanded_view_box(target_bounds, expansion);
    let query_bounds = view_box_to_bounds(query_view_box);
    let mut report = context_report(
        &document.input,
        &document.document,
        &target,
        target_bounds,
        query_bounds,
        expansion,
        limit,
    )?;

    if let Some(capture_output) = request_string(
        request,
        &["captureOut", "capture-out", "capture_out", "screenshotOut"],
    )? {
        let format = request_capture_format(request)?
            .or_else(|| infer_capture_format_from_path(&capture_output))
            .ok_or_else(|| {
                "captureOut format is ambiguous; use .svg/.png or format=svg|png.".to_string()
            })?;
        let render = capture_render_primitives(&document.document, &target, query_view_box);
        let render_output = write_capture_output(
            &render.primitives,
            query_view_box,
            &capture_output,
            format,
            raster,
        )?;
        let primitive_count = render.primitives.len();
        set_object_field(
            &mut report,
            "capture",
            json!({
                "ok": true,
                "path": capture_output,
                "format": format.as_str(),
                "verified": true,
                "bytes": render_output.bytes,
                "pixelSize": render_output.pixel_size.map(PixelSize::to_json),
                "viewBox": view_box_json(query_view_box),
                "render": {
                    "mode": render.mode,
                    "primitiveCount": primitive_count,
                    "targets": render.targets.to_json(),
                },
            }),
        );
    }
    Ok(report)
}

pub(super) fn session_capture(
    document: &SessionDocument,
    request: &Value,
) -> Result<Value, String> {
    let target = request_required_target(request)?;
    let output = request_string(request, &["out", "output", "path"])?;
    let format = request_capture_format(request)?;
    let expansion = request_expansion(request, CropExpansion::uniform_abs(8.0))?;
    let raster = request_raster_options(request)?;
    let (output, format, output_defaulted) = resolve_capture_output(output, format)?;
    let bounds = target_bounds(&document.document, &target)?;
    let view_box = expanded_view_box(bounds, expansion);
    let render = capture_render_primitives(&document.document, &target, view_box);
    let render_output =
        write_capture_output(&render.primitives, view_box, &output, format, raster)?;
    let primitive_count = render.primitives.len();
    Ok(json!({
        "ok": true,
        "input": document.input,
        "target": target.to_json(),
        "warnings": default_capture_warnings(output_defaulted, &output),
        "output": {
            "path": output,
            "format": format.as_str(),
            "defaulted": output_defaulted,
            "verified": true,
            "bytes": render_output.bytes,
            "pixelSize": render_output.pixel_size.map(PixelSize::to_json),
        },
        "bounds": bounds_json(bounds),
        "viewBox": view_box_json(view_box),
        "expansion": expansion.to_json(),
        "render": {
            "mode": render.mode,
            "primitiveCount": primitive_count,
            "targets": render.targets.to_json(),
        },
    }))
}

pub(super) fn session_execute(
    document: &mut SessionDocument,
    request: &Value,
) -> Result<Value, String> {
    let commands = session_request_commands(request)?;
    let continue_on_error =
        request_bool(request, &["continueOnError", "continue-on-error"])?.unwrap_or(false);
    let before_revision = document.engine.revision();
    let mut results = Vec::new();
    let mut failed_indices = Vec::new();
    for (index, command) in commands.into_iter().enumerate() {
        let command_type = session_command_type_name(&command);
        let command_before_revision = document.engine.revision();
        match document.engine.execute_command_json(&command.to_string()) {
            Ok(result_text) => {
                let engine_result: Value =
                    serde_json::from_str(&result_text).map_err(|error| error.to_string())?;
                results.push(json!({
                    "index": index,
                    "ok": true,
                    "changed": engine_result.get("changed").and_then(Value::as_bool).unwrap_or(false),
                    "commandType": command_type,
                    "beforeRevision": command_before_revision,
                    "afterRevision": document.engine.revision(),
                    "result": engine_result,
                }));
            }
            Err(error) => {
                failed_indices.push(index);
                results.push(json!({
                    "index": index,
                    "ok": false,
                    "changed": false,
                    "commandType": command_type,
                    "beforeRevision": command_before_revision,
                    "afterRevision": document.engine.revision(),
                    "error": {
                        "message": error,
                    },
                }));
                if !continue_on_error {
                    break;
                }
            }
        }
    }
    document.refresh_document()?;
    let failed_count = failed_indices.len();
    Ok(json!({
        "ok": failed_count == 0,
        "commandCount": results.len(),
        "failedCount": failed_count,
        "failedIndices": failed_indices,
        "continueOnError": continue_on_error,
        "document": {
            "beforeRevision": before_revision,
            "afterRevision": document.engine.revision(),
            "revisionChanged": before_revision != document.engine.revision(),
        },
        "results": results,
    }))
}

pub(super) fn session_save(document: &SessionDocument, request: &Value) -> Result<Value, String> {
    let output = request_required_string(request, &["out", "output", "path"])?;
    let format = request_string(request, &["format", "saveFormat", "save-format"])?;
    write_engine_output(&document.engine, &output, format.as_deref())?;
    Ok(json!({
        "ok": true,
        "path": output,
        "format": format.or_else(|| infer_format_from_path(&output)),
        "revision": document.engine.revision(),
    }))
}

pub(super) fn with_session<F>(session: &Option<SessionDocument>, f: F) -> Result<Value, String>
where
    F: FnOnce(&SessionDocument) -> Result<Value, String>,
{
    let Some(document) = session.as_ref() else {
        return Err(
            "No document is open. Send {\"op\":\"open\",\"input\":\"path\"} first.".to_string(),
        );
    };
    f(document)
}

pub(super) fn with_session_mut<F>(
    session: &mut Option<SessionDocument>,
    f: F,
) -> Result<Value, String>
where
    F: FnOnce(&mut SessionDocument) -> Result<Value, String>,
{
    let Some(document) = session.as_mut() else {
        return Err(
            "No document is open. Send {\"op\":\"open\",\"input\":\"path\"} first.".to_string(),
        );
    };
    f(document)
}

pub(super) fn write_session_line(value: Value) -> Result<(), String> {
    let mut stdout = io::stdout();
    serde_json::to_writer(&mut stdout, &value).map_err(|error| error.to_string())?;
    stdout.write_all(b"\n").map_err(|error| error.to_string())?;
    stdout.flush().map_err(|error| error.to_string())
}

pub(super) fn session_ready_json(session: Option<&SessionDocument>) -> Value {
    json!({
        "ok": true,
        "event": "ready",
        "protocol": SESSION_PROTOCOL_VERSION,
        "input": session.map(|document| document.input.clone()),
        "document": session.map(SessionDocument::summary_json),
        "help": {
            "request": {"id": 1, "op": "help"},
            "open": {"id": 2, "op": "open", "input": "input.cdxml"},
            "capture": {"id": 3, "op": "capture", "target": "molecule:0", "out": "crop.png", "scale": 6},
            "captureSelection": {"id": 4, "op": "capture", "target": ["object:obj_a", "object:obj_b"], "out": "selection.png", "width": 1800},
            "exit": {"id": 99, "op": "exit"},
        }
    })
}

pub(super) fn session_help_json() -> Value {
    json!({
        "protocol": SESSION_PROTOCOL_VERSION,
        "transport": "stdin/stdout JSON Lines; one compact JSON response per request.",
        "operations": {
            "open": {"required": ["input"], "description": "Load a document into the session."},
            "targets": {"description": "Return stable selectors and bounds for the open document."},
            "detail": {"required": ["target"], "description": "Return one object/molecule/node/bond detail JSON."},
            "context": {"required": ["target"], "optional": ["targets", "radius", "captureOut", "scale", "width", "height", "limit"], "description": "Return nearby summaries and optionally a screenshot. target/targets may be a selector string or an array of selector strings."},
            "capture": {"required": ["target"], "optional": ["targets", "out", "format", "scale", "width", "height", "expand", "expandRel"], "description": "Write a precise crop; target/targets may be a selector string or an array. Multi-target crops use the minimum union bounds."},
            "execute": {"required": ["command or commands"], "optional": ["continueOnError"], "description": "Run one or more engine JSON commands against the in-memory document."},
            "save": {"required": ["out"], "optional": ["format"], "description": "Save the current in-memory document."},
            "status": {"description": "Return the open document summary."},
            "close": {"description": "Close the open document without saving."},
            "exit": {"description": "Terminate the session process."}
        },
        "targetSelectors": ["all", "object:<id>", "molecule:<index>", "node:<id>", "bond:<id>", "bounds:minX,minY,maxX,maxY", "selection:<selector;selector>"]
    })
}

pub(super) fn session_ok(id: Value, op: String, result: Value) -> Value {
    let ok = result.get("ok").and_then(Value::as_bool).unwrap_or(true);
    json!({
        "ok": ok,
        "id": id,
        "op": op,
        "result": result,
    })
}

pub(super) fn session_error(id: Value, op: Value, kind: &str, message: String) -> Value {
    json!({
        "ok": false,
        "id": id,
        "op": op,
        "error": {
            "kind": kind,
            "message": message,
        }
    })
}

pub(super) fn request_required_target(request: &Value) -> Result<TargetSelector, String> {
    request_target(request)?.ok_or_else(|| {
        "Request requires target, object, molecule, node, bond, or bounds.".to_string()
    })
}

pub(super) fn request_target(request: &Value) -> Result<Option<TargetSelector>, String> {
    if let Some(target) = request.get("target") {
        return parse_target_value(target).map(Some);
    }
    if let Some(targets) = request.get("targets") {
        return parse_target_value(targets).map(Some);
    }
    if let Some(id) = request.get("object").and_then(Value::as_str) {
        return Ok(Some(TargetSelector::Object(id.to_string())));
    }
    if let Some(index) = request.get("molecule").and_then(Value::as_u64) {
        return Ok(Some(TargetSelector::Molecule(index as usize)));
    }
    if let Some(id) = request.get("node").and_then(Value::as_str) {
        return Ok(Some(TargetSelector::Node(id.to_string())));
    }
    if let Some(id) = request.get("bond").and_then(Value::as_str) {
        return Ok(Some(TargetSelector::Bond(id.to_string())));
    }
    if let Some(bounds) = request.get("bounds") {
        return parse_bounds_value(bounds)
            .map(TargetSelector::Bounds)
            .map(Some);
    }
    Ok(None)
}

pub(super) fn parse_target_value(value: &Value) -> Result<TargetSelector, String> {
    if let Some(target) = value.as_str() {
        return parse_target_selector(target);
    }
    let Some(values) = value.as_array() else {
        return Err(
            "target must be a selector string or an array of selector strings.".to_string(),
        );
    };
    let mut targets = Vec::new();
    for value in values {
        let Some(target) = value.as_str() else {
            return Err("target arrays must contain selector strings.".to_string());
        };
        collect_selection_targets(parse_target_selector(target)?, &mut targets);
    }
    target_from_selection_targets(targets)
}

pub(super) fn parse_bounds_value(value: &Value) -> Result<[f64; 4], String> {
    if let Some(text) = value.as_str() {
        return parse_bounds_arg(text);
    }
    let Some(values) = value.as_array() else {
        return Err("bounds must be a string or an array of four numbers.".to_string());
    };
    if values.len() != 4 {
        return Err("bounds array must contain four numbers.".to_string());
    }
    let mut out = [0.0; 4];
    for (index, value) in values.iter().enumerate() {
        out[index] = value
            .as_f64()
            .ok_or_else(|| "bounds array values must be finite numbers.".to_string())?;
        if !out[index].is_finite() {
            return Err("bounds array values must be finite numbers.".to_string());
        }
    }
    if out[2] <= out[0] || out[3] <= out[1] {
        return Err("bounds must satisfy maxX > minX and maxY > minY.".to_string());
    }
    Ok(out)
}

pub(super) fn request_expansion(
    request: &Value,
    mut expansion: CropExpansion,
) -> Result<CropExpansion, String> {
    if let Some(value) = request_f64(request, &["radius", "padding", "expand"])? {
        expansion.abs_left = value;
        expansion.abs_top = value;
        expansion.abs_right = value;
        expansion.abs_bottom = value;
    }
    if let Some(value) = request_f64(request, &["expandX", "expand-x"])? {
        expansion.abs_left = value;
        expansion.abs_right = value;
    }
    if let Some(value) = request_f64(request, &["expandY", "expand-y"])? {
        expansion.abs_top = value;
        expansion.abs_bottom = value;
    }
    if let Some(value) = request_f64(request, &["expandLeft", "expand-left"])? {
        expansion.abs_left = value;
    }
    if let Some(value) = request_f64(request, &["expandRight", "expand-right"])? {
        expansion.abs_right = value;
    }
    if let Some(value) = request_f64(request, &["expandTop", "expand-top"])? {
        expansion.abs_top = value;
    }
    if let Some(value) = request_f64(request, &["expandBottom", "expand-bottom"])? {
        expansion.abs_bottom = value;
    }
    if let Some(value) = request_f64(request, &["expandRel", "expand-rel"])? {
        expansion.rel_left = value;
        expansion.rel_top = value;
        expansion.rel_right = value;
        expansion.rel_bottom = value;
    }
    if let Some(value) = request_f64(request, &["expandRelX", "expand-rel-x"])? {
        expansion.rel_left = value;
        expansion.rel_right = value;
    }
    if let Some(value) = request_f64(request, &["expandRelY", "expand-rel-y"])? {
        expansion.rel_top = value;
        expansion.rel_bottom = value;
    }
    if let Some(value) = request_f64(request, &["expandRelLeft", "expand-rel-left"])? {
        expansion.rel_left = value;
    }
    if let Some(value) = request_f64(request, &["expandRelRight", "expand-rel-right"])? {
        expansion.rel_right = value;
    }
    if let Some(value) = request_f64(request, &["expandRelTop", "expand-rel-top"])? {
        expansion.rel_top = value;
    }
    if let Some(value) = request_f64(request, &["expandRelBottom", "expand-rel-bottom"])? {
        expansion.rel_bottom = value;
    }
    Ok(expansion)
}

pub(super) fn request_raster_options(request: &Value) -> Result<RasterOptions, String> {
    let mut raster = RasterOptions::default();
    if let Some(scale) = request_f64(request, &["scale"])? {
        if scale <= 0.0 {
            return Err("scale must be positive.".to_string());
        }
        raster.scale = scale;
    }
    raster.width = request_u32(request, &["width"])?;
    raster.height = request_u32(request, &["height"])?;
    Ok(raster)
}

pub(super) fn request_capture_format(request: &Value) -> Result<Option<CaptureFormat>, String> {
    request_string(request, &["format"]).and_then(|value| {
        value
            .map(|format| parse_capture_format(&format))
            .transpose()
    })
}

pub(super) fn session_request_commands(request: &Value) -> Result<Vec<Value>, String> {
    if let Some(command) = request.get("command").filter(|value| value.is_object()) {
        return Ok(vec![command.clone()]);
    }
    if let Some(commands) = request.get("commands").and_then(Value::as_array) {
        if commands.is_empty() {
            return Err("commands must not be empty.".to_string());
        }
        return Ok(commands.clone());
    }
    Err("execute requires command object or commands array.".to_string())
}

pub(super) fn session_command_type_name(command: &Value) -> Value {
    command
        .get("type")
        .and_then(Value::as_str)
        .map(|value| json!(value))
        .unwrap_or(Value::Null)
}

pub(super) fn request_required_string(request: &Value, keys: &[&str]) -> Result<String, String> {
    request_string(request, keys)?.ok_or_else(|| {
        format!(
            "Request requires one of: {}.",
            keys.iter().copied().collect::<Vec<_>>().join(", ")
        )
    })
}

pub(super) fn request_string(request: &Value, keys: &[&str]) -> Result<Option<String>, String> {
    for key in keys {
        if let Some(value) = request.get(*key) {
            return value
                .as_str()
                .map(|text| Some(text.to_string()))
                .ok_or_else(|| format!("{key} must be a string."));
        }
    }
    Ok(None)
}

pub(super) fn request_bool(request: &Value, keys: &[&str]) -> Result<Option<bool>, String> {
    for key in keys {
        if let Some(value) = request.get(*key) {
            return value
                .as_bool()
                .map(Some)
                .ok_or_else(|| format!("{key} must be a boolean."));
        }
    }
    Ok(None)
}

pub(super) fn request_f64(request: &Value, keys: &[&str]) -> Result<Option<f64>, String> {
    for key in keys {
        if let Some(value) = request.get(*key) {
            let Some(number) = value.as_f64() else {
                return Err(format!("{key} must be a number."));
            };
            if number < 0.0 || !number.is_finite() {
                return Err(format!("{key} must be a non-negative finite number."));
            }
            return Ok(Some(number));
        }
    }
    Ok(None)
}

pub(super) fn request_u32(request: &Value, keys: &[&str]) -> Result<Option<u32>, String> {
    for key in keys {
        if let Some(value) = request.get(*key) {
            let Some(number) = value.as_u64() else {
                return Err(format!("{key} must be a positive integer."));
            };
            if number == 0 || number > u32::MAX as u64 {
                return Err(format!(
                    "{key} must be a positive integer up to {}.",
                    u32::MAX
                ));
            }
            return Ok(Some(number as u32));
        }
    }
    Ok(None)
}

pub(super) fn request_usize(request: &Value, keys: &[&str]) -> Result<Option<usize>, String> {
    for key in keys {
        if let Some(value) = request.get(*key) {
            let Some(number) = value.as_u64() else {
                return Err(format!("{key} must be a positive integer."));
            };
            if number == 0 || number > usize::MAX as u64 {
                return Err(format!("{key} must be a positive integer."));
            }
            return Ok(Some(number as usize));
        }
    }
    Ok(None)
}
