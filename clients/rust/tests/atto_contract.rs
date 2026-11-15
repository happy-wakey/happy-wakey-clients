//! Rust regression coverage for the AttoLang client source projection.
use serde_json::Value;

const IDS: [(&str, &str); 6] = [
    ("health", "health_path"),
    ("listAlarms", "list_alarms_path"),
    ("createAlarm", "create_alarm_path"),
    ("transitionOccurrence", "transition_occurrence_path_template"),
    ("pullChanges", "pull_changes_path"),
    ("pushChanges", "push_changes_path"),
];

fn fixtures() -> (String, Value) {
    let atto = include_str!("../../atto/client.atto").to_string();
    let json = serde_json::from_str(include_str!("../../../contract/operations.json")).unwrap();
    (atto, json)
}

fn normalized(source: &str) -> String {
    source.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn function_body(source: &str, name: &str, args: &str, ty: &str) -> Result<String, String> {
    let header = format!("pub fnc {name}({args}): {ty} {{");
    let mut matches = source.match_indices(&header);
    let (position, _) = matches.next().ok_or_else(|| format!("missing {name}"))?;
    if matches.next().is_some() { return Err(format!("duplicate {name}")); }
    let start = position + header.len();
    let (mut depth, mut quoted, mut escape) = (1usize, false, false);
    for (offset, ch) in source[start..].char_indices() {
        if quoted && ch == '\\' && !escape { escape = true; continue; }
        if ch == '"' && !escape { quoted = !quoted; }
        else if !quoted && ch == '{' { depth += 1; }
        else if !quoted && ch == '}' {
            depth -= 1;
            if depth == 0 { return Ok(source[start..start + offset].into()); }
        }
        escape = false;
    }
    Err(format!("unclosed {name}"))
}

fn literal(source: &str, name: &str) -> Result<String, String> {
    let body = function_body(source, name, "", "string")?;
    let value = body.trim().strip_prefix("return \"")
        .and_then(|x| x.strip_suffix("\";"))
        .ok_or_else(|| format!("nonliteral {name}"))?;
    if value.contains('"') || value.contains('\\') { return Err("unsafe literal".into()); }
    Ok(value.into())
}

fn verify(source: &str, contract: &Value) -> Result<(), String> {
    if contract["schema"] != "happy-wakey/client-operations/v1" {
        return Err("schema version drift".into());
    }
    let operations = contract["operations"].as_array().ok_or("no operations")?;
    if operations.len() != IDS.len() { return Err("operation count drift".into()); }
    let (mut known, mut methods) = (String::new(), String::new());
    for (index, (id, function)) in IDS.iter().enumerate() {
        let op = &operations[index];
        let method = op["method"].as_str().ok_or("missing method")?;
        let route = op["path"].as_str().ok_or("missing route")?;
        if op["id"] != *id || op["auth"].as_bool() != Some(*id != "health") ||
            !matches!(method, "GET" | "POST") || literal(source, function)? != route ||
            !route.starts_with('/') || route.contains("..") || route.contains("//") ||
            route.contains('?') || route.contains('#') {
            return Err(format!("contract drift: {id}"));
        }
        if route.contains('{') && (*id != "transitionOccurrence" ||
            route.matches("{occurrence_id}").count() != 1) {
            return Err("unapproved path template".into());
        }
        known.push_str(&format!("if operation eq \"{id}\" then return true; fi "));
        methods.push_str(&format!("if operation eq \"{id}\" then return \"{method}\"; fi "));
    }
    known.push_str("return false;");
    methods.push_str("return \"\";");
    if normalized(&function_body(source, "is_known_operation", "string operation", "bool")?) != normalized(&known)
        || normalized(&function_body(source, "operation_method", "string operation", "string")?) != normalized(&methods)
        || normalized(&function_body(source, "requires_bearer", "string operation", "bool")?)
           != "if operation eq \"health\" then return false; fi return true;"
        || literal(source, "bearer_header_name")? != "Authorization"
        || literal(source, "telemetry_event_name")? != "happy_wakey.client.request" {
        return Err("allowlist, auth, or telemetry drift".into());
    }
    Ok(())
}

#[test]
fn six_operations_match_source_and_json() {
    let (s, ops) = fixtures();
    verify(&s, &ops).unwrap();
}

#[test]
fn rejects_path_and_dynamic_expression_drift() {
    let (s, ops) = fixtures();
    assert!(verify(&s.replace("return \"/v1/sync/pull\";", "return \"/v2/sync/pull\";"), &ops).is_err());
    assert!(verify(&s.replace("return \"/healthz\";", "return user_input();"), &ops).is_err());
}

#[test]
fn rejects_method_and_allowlist_drift() {
    let (s, ops) = fixtures();
    assert!(verify(&s.replace("if operation eq \"pushChanges\" then return \"POST\"; fi",
                              "if operation eq \"pushChanges\" then return \"GET\"; fi"), &ops).is_err());
    assert!(verify(&s.replace("if operation eq \"pushChanges\" then return true; fi",
                              "if operation eq \"unlisted\" then return true; fi"), &ops).is_err());
}

#[test]
fn rejects_bearer_downgrade() {
    let (s, ops) = fixtures();
    assert!(verify(&s.replace("if operation eq \"health\" then return false; fi",
                              "if operation eq \"createAlarm\" then return false; fi"), &ops).is_err());
    let mut ops = ops;
    ops["operations"][2]["auth"] = Value::Bool(false);
    assert!(verify(&s, &ops).is_err());
}

#[test]
fn rejects_duplicate_operation_and_function() {
    let (s, mut ops) = fixtures();
    ops["operations"][5]["id"] = Value::String("health".into());
    assert!(verify(&s, &ops).is_err());
    let (_, ops) = fixtures();
    assert!(verify(&format!("{s}\npub fnc health_path(): string {{ return \"/healthz\"; }}"),
                   &ops).is_err());
}