//! Bounded host-side request and response contract for the AttoLang projection.
//! This crate does NOT compile or run .atto guest code and does not provide HTTP.
//! A Transport implementation must enforce network egress policy, TLS validation,
//! DNS rebinding protections, timeouts, redirect refusal and streaming byte caps.
//! JSON wire-schema parity still requires independent validation.

use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::net::{IpAddr, Ipv4Addr};
use std::time::Instant;
use url::Url;

pub const MAX_REQUEST_BYTES: usize = 32_768;
pub const MAX_RESPONSE_BYTES: usize = 921_600;
pub const TELEMETRY_EVENT: &str = "happy_wakey.client.request";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClientError {
    Contract,
    InvalidOrigin,
    InvalidInput,
    InvalidResponse,
    Redirect,
    HttpStatus(u16),
    Network,
}
impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HttpStatus(s) => write!(f, "Happy Wakey HTTP status {s}"),
            _ => write!(f, "Happy Wakey request rejected: {self:?}"),
        }
    }
}
impl std::error::Error for ClientError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Operation {
    pub id: &'static str,
    pub method: &'static str,
    pub path: &'static str,
    pub auth: bool,
}
const OPERATIONS: [Operation; 6] = [
    Operation { id:"health", method:"GET", path:"/healthz", auth:false },
    Operation { id:"listAlarms", method:"GET", path:"/v1/alarms", auth:true },
    Operation { id:"createAlarm", method:"POST", path:"/v1/alarms", auth:true },
    Operation { id:"transitionOccurrence", method:"POST", path:"/v1/occurrences/{occurrence_id}/transitions", auth:true },
    Operation { id:"pullChanges", method:"GET", path:"/v1/sync/pull", auth:true },
    Operation { id:"pushChanges", method:"POST", path:"/v1/sync/push", auth:true },
];

pub fn operations() -> Result<&'static [Operation], ClientError> {
    let source: Value = serde_json::from_str(include_str!("../../../../contract/operations.json"))
        .map_err(|_| ClientError::Contract)?;
    let entries = source["operations"].as_array().ok_or(ClientError::Contract)?;
    if source["schema"] != "happy-wakey/client-operations/v1" || entries.len() != OPERATIONS.len() {
        return Err(ClientError::Contract);
    }
    for (got, expected) in entries.iter().zip(&OPERATIONS) {
        if got["id"] != expected.id || got["method"] != expected.method ||
            got["path"] != expected.path || got["auth"] != expected.auth {
            return Err(ClientError::Contract);
        }
    }
    Ok(&OPERATIONS)
}

/// A validated, immutable origin. Its private URL cannot be replaced by callers.
#[derive(Clone)]
pub struct ValidatedOrigin {
    url: Url,
}
impl ValidatedOrigin {
    pub fn as_str(&self) -> &str { self.url.as_str() }
}
pub fn validated_origin(raw: &str, allow_http_loopback: bool) -> Result<ValidatedOrigin, ClientError> {
    if raw.is_empty() || raw.len() > 2048 ||
        raw.bytes().any(|b| b.is_ascii_control() || b == b' ' || b == b'\\') {
        return Err(ClientError::InvalidOrigin);
    }
    let authority = raw.split_once("://").ok_or(ClientError::InvalidOrigin)?.1
        .split(['/', '?', '#']).next().ok_or(ClientError::InvalidOrigin)?;
    if authority.is_empty() || authority.contains('@') { return Err(ClientError::InvalidOrigin); }
    // URL parsers can normalize 127.1 or numeric host shortcuts to 127.0.0.1.
    let raw_host = if authority.starts_with('[') {
        authority.split_once(']').ok_or(ClientError::InvalidOrigin)?.0.trim_start_matches('[')
    } else {
        authority.split(':').next().ok_or(ClientError::InvalidOrigin)?
    };
    if raw_host.chars().all(|c| c.is_ascii_digit() || c == '.') &&
        raw_host.parse::<Ipv4Addr>().is_err() {
        return Err(ClientError::InvalidOrigin);
    }
    let url = Url::parse(raw).map_err(|_| ClientError::InvalidOrigin)?;
    if url.cannot_be_a_base() || !url.username().is_empty() || url.password().is_some() ||
        url.query().is_some() || url.fragment().is_some() || url.path() != "/" ||
        url.host_str().is_none() {
        return Err(ClientError::InvalidOrigin);
    }
    let host = url.host_str().ok_or(ClientError::InvalidOrigin)?;
    let host = host.trim_start_matches('[').trim_end_matches(']');
    let ip = host.parse::<IpAddr>().ok();
    let loopback = host.eq_ignore_ascii_case("localhost") || ip.is_some_and(|ip| ip.is_loopback());
    if ip.is_some() && !loopback { return Err(ClientError::InvalidOrigin); }
    if ip.is_none() && (host.split('.').any(|s| s.is_empty() || s.starts_with('-') || s.ends_with('-')) ||
        host.chars().all(|c| c.is_ascii_digit() || c == '.')) {
        return Err(ClientError::InvalidOrigin);
    }
    if url.scheme() != "https" && !(url.scheme() == "http" && loopback && allow_http_loopback) {
        return Err(ClientError::InvalidOrigin);
    }
    Ok(ValidatedOrigin { url })
}

/// Inputs are borrowed only for synchronous request planning; no token is logged.
#[derive(Clone, Default)]
pub struct Call<'a> {
    pub id: &'a str,
    pub bearer: Option<&'a str>,
    pub body: Option<&'a Value>,
    pub occurrence_id: Option<&'a str>,
    pub cursor: Option<&'a str>,
    pub limit: Option<u16>,
}

/// Intentionally not Debug: Authorization headers contain credentials.
pub struct RequestPlan {
    pub method: &'static str,
    pub url: Url,
    pub headers: BTreeMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub operation: &'static str,
}
fn valid_bearer(token: &str) -> bool {
    let core = token.trim_end_matches('=');
    !core.is_empty() && token.len() <= 16_384 && token.len() - core.len() <= 2 &&
        core.bytes().all(|b| b.is_ascii_alphanumeric() || b"-._~+/".contains(&b))
}
fn segment(input: &str) -> Result<String, ClientError> {
    if input.is_empty() || input.len() > 128 || matches!(input, "." | "..") ||
        input.chars().any(char::is_control) { return Err(ClientError::InvalidInput); }
    let mut encoded = String::new();
    for b in input.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            encoded.push(b as char);
        } else {
            encoded.push_str(&format!("%{b:02X}"));
        }
    }
    Ok(encoded)
}
fn nonempty_string(object: &Map<String, Value>, key: &str) -> bool {
    object.get(key).and_then(Value::as_str).is_some_and(|v| !v.trim().is_empty())
}
fn fields(object: &Map<String, Value>, required: &[&str]) -> bool {
    object.len() == required.len() && required.iter().all(|k| object.contains_key(*k))
}
fn validate_body(op: &str, value: &Value) -> Result<(), ClientError> {
    match op {
        "createAlarm" => {
            let m = value.as_object().ok_or(ClientError::InvalidInput)?;
            if !fields(m, &["transition_id", "label", "local_time", "time_zone",
                "weekdays", "enabled", "sound", "volume", "gradual_seconds", "tags"]) ||
                !["transition_id","label","local_time","time_zone","sound"].iter().all(|k| nonempty_string(m, k)) ||
                !m.get("enabled").is_some_and(Value::is_boolean) {
                return Err(ClientError::InvalidInput);
            }
            let label = m["label"].as_str().ok_or(ClientError::InvalidInput)?;
            let local_time = m["local_time"].as_str().ok_or(ClientError::InvalidInput)?;
            let zone = m["time_zone"].as_str().ok_or(ClientError::InvalidInput)?;
            let sound = m["sound"].as_str().ok_or(ClientError::InvalidInput)?;
            if label.len() > 120 || sound.len() > 128 || zone.len() > 64 ||
                !(5..=8).contains(&local_time.len()) || !local_time.contains(':') {
                return Err(ClientError::InvalidInput);
            }
            let volume = m["volume"].as_f64().ok_or(ClientError::InvalidInput)?;
            if !(0.0..=1.0).contains(&volume) || m["gradual_seconds"].as_u64().is_none_or(|v| v > 1800) {
                return Err(ClientError::InvalidInput);
            }
            let weekdays = m["weekdays"].as_array().ok_or(ClientError::InvalidInput)?;
            let mut seen = BTreeSet::new();
            if !(1..=7).contains(&weekdays.len()) ||
                weekdays.iter().any(|v| v.as_u64().is_none_or(|day| day > 6 || !seen.insert(day))) {
                return Err(ClientError::InvalidInput);
            }
            let tags = m["tags"].as_array().ok_or(ClientError::InvalidInput)?;
            let mut seen_tags = BTreeSet::new();
            if tags.len() > 20 || tags.iter().any(|v| v.as_str().is_none_or(|t|
                t.is_empty() || t.len() > 40 || !seen_tags.insert(t.to_string()))) {
                return Err(ClientError::InvalidInput);
            }
            Ok(())
        }
        "transitionOccurrence" => {
            let m = value.as_object().ok_or(ClientError::InvalidInput)?;
            if !fields(m, &["transition_id","expected_generation","event","snooze_until","client_time"]) ||
                !nonempty_string(m, "transition_id") || !nonempty_string(m, "client_time") ||
                m["expected_generation"].as_u64().is_none() { return Err(ClientError::InvalidInput); }
            let event = m["event"].as_str().ok_or(ClientError::InvalidInput)?;
            if !matches!(event, "fire" | "acknowledge" | "snooze" | "complete" | "mark_missed" | "cancel") {
                return Err(ClientError::InvalidInput);
            }
            let until = &m["snooze_until"];
            if (event == "snooze" && until.as_str().is_none_or(str::is_empty)) ||
                (event != "snooze" && !until.is_null()) {
                return Err(ClientError::InvalidInput);
            }
            Ok(())
        }
        "pushChanges" => {
            let changes = value.as_array().ok_or(ClientError::InvalidInput)?;
            if changes.len() > 256 { return Err(ClientError::InvalidInput); }
            for change in changes {
                let m = change.as_object().ok_or(ClientError::InvalidInput)?;
                if !fields(m, &["change_id","scope","collection","entity_id","operation",
                    "generation","actor_id","document","occurred_at"]) ||
                    !["change_id","scope","collection","entity_id","actor_id","occurred_at"]
                        .iter().all(|k| nonempty_string(m, k)) ||
                    !matches!(m["operation"].as_str(), Some("upsert" | "delete")) ||
                    m["generation"].as_u64().is_none() {
                    return Err(ClientError::InvalidInput);
                }
            }
            Ok(())
        }
        _ => Err(ClientError::InvalidInput),
    }
}
pub fn plan_request(base: &ValidatedOrigin, call: &Call<'_>) -> Result<RequestPlan, ClientError> {
    let op = operations()?.iter().find(|v| v.id == call.id).ok_or(ClientError::InvalidInput)?;
    let mut headers = BTreeMap::new();
    headers.insert("Accept".into(), "application/json".into());
    if op.auth {
        let token = call.bearer.filter(|t| valid_bearer(t)).ok_or(ClientError::InvalidInput)?;
        headers.insert("Authorization".into(), format!("Bearer {token}"));
    } else if call.bearer.is_some() { return Err(ClientError::InvalidInput); }

    let route = if op.id == "transitionOccurrence" {
        op.path.replace("{occurrence_id}",
            &segment(call.occurrence_id.ok_or(ClientError::InvalidInput)?)?)
    } else {
        if call.occurrence_id.is_some() { return Err(ClientError::InvalidInput); }
        op.path.to_string()
    };
    let mut url = base.url.join(&route).map_err(|_| ClientError::InvalidInput)?;
    if op.id == "pullChanges" {
        let limit = call.limit.unwrap_or(100);
        let cursor = call.cursor.unwrap_or("0");
        if !(1..=100).contains(&limit) || cursor.len() > 2048 ||
            cursor.chars().any(char::is_control) { return Err(ClientError::InvalidInput); }
        url.query_pairs_mut().append_pair("cursor", cursor).append_pair("limit", &limit.to_string());
    } else if call.cursor.is_some() || call.limit.is_some() {
        return Err(ClientError::InvalidInput);
    }
    if url.origin() != base.url.origin() { return Err(ClientError::InvalidInput); }
    let body = if op.method == "POST" {
        let value = call.body.ok_or(ClientError::InvalidInput)?;
        validate_body(op.id, value)?;
        let bytes = serde_json::to_vec(value).map_err(|_| ClientError::InvalidInput)?;
        if bytes.len() > MAX_REQUEST_BYTES { return Err(ClientError::InvalidInput); }
        headers.insert("Content-Type".into(), "application/json".into());
        Some(bytes)
    } else {
        if call.body.is_some() { return Err(ClientError::InvalidInput); }
        None
    };
    Ok(RequestPlan { method:op.method, url, headers, body, operation:op.id })
}

pub struct Response {
    pub status: u16,
    pub content_type: String,
    pub bytes: Vec<u8>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransportError;

pub trait Transport {
    /// Must reject redirects, enforce TLS/DNS/egress restrictions, timeouts and size caps
    /// before buffering the response. Returning a complete Response is not sufficient.
    fn exchange(&mut self, request: &RequestPlan) -> Result<Response, TransportError>;
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TelemetryRecord {
    pub operation: &'static str,
    pub outcome: &'static str,
    pub status: u16,
    pub duration_ms: u128,
}
pub trait Telemetry {
    fn record(&mut self, event: &'static str, fields: TelemetryRecord);
}

/// Duplicate-key-rejecting JSON decoder, including nested objects and arrays.
struct StrictValue(Value);
impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct StrictVisitor;
        impl<'de> Visitor<'de> for StrictVisitor {
            type Value = StrictValue;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("valid JSON with no duplicate object keys")
            }
            fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> { Ok(StrictValue(Value::Null)) }
            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> { Ok(StrictValue(Value::Bool(v))) }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> { Ok(StrictValue(Value::from(v))) }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> { Ok(StrictValue(Value::from(v))) }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
                if v.is_finite() { Ok(StrictValue(Value::from(v))) } else { Err(E::custom("nonfinite")) }
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::String(v.into())))
            }
            fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
                Ok(StrictValue(Value::String(v)))
            }
            fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> { Ok(StrictValue(Value::Null)) }
            fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Self::Value, D::Error> {
                StrictValue::deserialize(d)
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut values = Vec::new();
                while let Some(value) = seq.next_element::<StrictValue>()? { values.push(value.0); }
                Ok(StrictValue(Value::Array(values)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut values = Map::new();
                while let Some((key, value)) = map.next_entry::<String, StrictValue>()? {
                    if values.contains_key(&key) { return Err(de::Error::custom("duplicate key")); }
                    values.insert(key, value.0);
                }
                Ok(StrictValue(Value::Object(values)))
            }
        }
        deserializer.deserialize_any(StrictVisitor)
    }
}
pub fn execute<T: Transport, L: Telemetry>(
    transport: &mut T, telemetry: &mut L, plan: &RequestPlan,
) -> Result<Value, ClientError> {
    let started = Instant::now();
    let mut status = 0u16;
    let result = match transport.exchange(plan) {
        Err(_) => Err(ClientError::Network),
        Ok(reply) => {
            status = reply.status;
            if (300..400).contains(&status) {
                Err(ClientError::Redirect)
            } else if !(200..300).contains(&status) {
                Err(ClientError::HttpStatus(status))
            } else if reply.bytes.len() > MAX_RESPONSE_BYTES ||
                !matches!(reply.content_type.split(';').next().unwrap_or("").trim().to_ascii_lowercase().as_str(),
                    "application/json" | "application/problem+json") {
                Err(ClientError::InvalidResponse)
            } else {
                (|| {
                    let mut decoder = serde_json::Deserializer::from_slice(&reply.bytes);
                    let value = StrictValue::deserialize(&mut decoder).map_err(|_| ClientError::InvalidResponse)?;
                    decoder.end().map_err(|_| ClientError::InvalidResponse)?;
                    Ok(value.0)
                })()
            }
        }
    };
    telemetry.record(TELEMETRY_EVENT, TelemetryRecord {
        operation:plan.operation,
        outcome:if result.is_ok() { "success" } else { "error" },
        status,
        duration_ms:started.elapsed().as_millis(),
    });
    result
}
