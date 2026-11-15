use happy_wakey_atto_host_contract::{
    execute, operations, plan_request, validated_origin, Call, ClientError,
    RequestPlan, Response, Telemetry, TelemetryRecord, Transport, TransportError,
    MAX_REQUEST_BYTES, MAX_RESPONSE_BYTES, TELEMETRY_EVENT,
};
use serde_json::{json, Value};

#[derive(Default)]
struct FakeTransport {
    exchanges: usize,
    last_url: Option<String>,
    last_method: Option<String>,
    last_authorization: Option<String>,
    reply: Option<Response>,
    fail: bool,
}
impl Transport for FakeTransport {
    fn exchange(&mut self, p: &RequestPlan) -> Result<Response, TransportError> {
        self.exchanges += 1;
        self.last_url = Some(p.url.to_string());
        self.last_method = Some(p.method.into());
        self.last_authorization = p.headers.get("Authorization").cloned();
        if self.fail { return Err(TransportError); }
        Ok(self.reply.take().unwrap_or(Response {
            status: 200, content_type: "application/json".into(), bytes: b"{\"ok\":true}".to_vec(),
        }))
    }
}
#[derive(Default)]
struct Records(Vec<(&'static str, TelemetryRecord)>);
impl Telemetry for Records {
    fn record(&mut self, event: &'static str, fields: TelemetryRecord) {
        self.0.push((event, fields));
    }
}
fn base() -> happy_wakey_atto_host_contract::ValidatedOrigin {
    validated_origin("https://api.example.org", false).unwrap()
}
fn create() -> Value {
    json!({
        "transition_id": "a1", "label":"Morning", "local_time":"07:30",
        "time_zone":"America/Lima", "weekdays":[0,1,2,3,4],
        "enabled":true, "sound":"bell", "volume":0.5,
        "gradual_seconds":60, "tags":["home"]
    })
}
fn transition() -> Value {
    json!({
        "transition_id":"a1", "expected_generation":3, "event":"complete",
        "snooze_until":null, "client_time":"2026-11-15T12:30:00Z"
    })
}
fn push() -> Value {
    json!([{
        "change_id":"a", "scope":"alarms", "collection":"alarms",
        "entity_id":"a1", "operation":"upsert", "generation":1,
        "actor_id":"actor", "document":null, "occurred_at":"2026-11-15T12:30:00Z"
    }])
}
fn plan(id: &str, body: Option<&Value>) -> Result<RequestPlan, ClientError> {
    plan_request(&base(), &Call {
        id,
        bearer: (id != "health").then_some("TOKEN"),
        body,
        occurrence_id:(id == "transitionOccurrence").then_some("occ"),
        ..Call::default()
    })
}

#[test]
fn contract_six_operations_are_pinned() {
    let ops = operations().unwrap();
    assert_eq!(ops.len(), 6);
    assert_eq!(ops.iter().filter(|op| !op.auth).count(), 1);
    assert_eq!(ops[0].id, "health");
    assert!(ops.iter().any(|op| op.id == "transitionOccurrence" && op.method == "POST"));
}
#[test]
fn plan_and_execute_all_six_operations() {
    let create = create(); let transition = transition(); let push = push();
    let mut net = FakeTransport::default(); let mut records = Records::default();
    for id in ["health","listAlarms","createAlarm","transitionOccurrence","pullChanges","pushChanges"] {
        let body = match id {
            "createAlarm" => Some(&create),
            "transitionOccurrence" => Some(&transition),
            "pushChanges" => Some(&push),
            _ => None,
        };
        let p = plan(id, body).unwrap();
        assert!(p.url.as_str().starts_with("https://api.example.org/"));
        assert_eq!(execute(&mut net, &mut records, &p).unwrap(), json!({"ok":true}));
    }
    assert_eq!(net.exchanges, 6);
    assert_eq!(records.0.len(), 6);
}
#[test]
fn invalid_origins_rejected_before_dispatch() {
    for raw in [
        "http://api.example.org", "http://127.0.0.1", "https://a:b@api.example.org",
        "https://@api.example.org", "https://api.example.org/path",
        "https://api.example.org/?token=secret", "https://api.example.org/#tag",
        "https://192.0.2.2", "https://8.8.8.8", "https://127.1",
        "https://2130706433", "https://-bad.test", "https://foo..test",
        "https://api.example.org\\@evil.org", "https://api.example.org\r\nHost:evil",
        "ftp://api.example.org",
    ] {
        assert!(validated_origin(raw, false).is_err(), "admitted origin: {raw}");
    }
}
#[test]
fn only_explicit_loopback_http_allowed() {
    let local = validated_origin("http://127.0.0.1:8000", true).unwrap();
    assert_eq!(local.as_str(), "http://127.0.0.1:8000/");
    assert!(validated_origin("http://localhost:8000", true).is_ok());
    assert!(validated_origin("http://example.org", true).is_err());
}
#[test]
fn unknown_operations_fail_closed() {
    assert!(plan_request(&base(), &Call {
        id:"deleteAccount", bearer:Some("secret"), ..Call::default()
    }).is_err());
}
#[test]
fn bearer_rejection_and_header_separation() {
    for bearer in [None,Some(""),Some("bad token"),Some("abc\nEvil:1"),Some("é")] {
        assert!(plan_request(&base(), &Call {
            id:"listAlarms",bearer,..Call::default()
        }).is_err());
    }
    let p = plan("listAlarms",None).unwrap();
    assert_eq!(p.headers.get("Authorization").map(String::as_str), Some("Bearer TOKEN"));
    assert!(!p.url.as_str().contains("TOKEN"));
    assert!(plan_request(&base(), &Call {
        id:"health", bearer:Some("TOKEN"),..Call::default()
    }).is_err());
}
#[test]
fn occurrence_id_encoded_as_one_segment() {
    let b = transition();
    let p = plan_request(&base(), &Call {
        id:"transitionOccurrence",bearer:Some("TOKEN"),
        body:Some(&b),occurrence_id:Some("a/b?c"),..Call::default()
    }).unwrap();
    assert!(p.url.as_str().contains("a%2Fb%3Fc"));
    for id in ["",".","..","a\nb","a\rb"] {
        assert!(plan_request(&base(), &Call {
            id:"transitionOccurrence",bearer:Some("TOKEN"),
            body:Some(&b),occurrence_id:Some(id),..Call::default()
        }).is_err());
    }
}
#[test]
fn pagination_query_is_encoded_and_bounded() {
    let p = plan_request(&base(), &Call {
        id:"pullChanges",bearer:Some("TOKEN"), cursor:Some("x y&z"),
        limit:Some(5),..Call::default()
    }).unwrap();
    assert!(p.url.as_str().contains("cursor=x+y%26z"));
    assert!(p.url.as_str().contains("limit=5"));
    for limit in [0,101] {
        assert!(plan_request(&base(), &Call {
            id:"pullChanges",bearer:Some("TOKEN"),limit:Some(limit),..Call::default()
        }).is_err());
    }
    assert!(plan_request(&base(), &Call {
        id:"health", cursor:Some("x"),..Call::default()
    }).is_err());
}
#[test]
fn create_request_requires_full_fields() {
    let mut b = create();
    assert!(plan("createAlarm", Some(&b)).is_ok());
    b.as_object_mut().unwrap().remove("label");
    assert!(plan("createAlarm", Some(&b)).is_err());
}
#[test]
fn create_rejects_extra_fields_bad_days_and_volume() {
    let mut b = create();
    b["unexpected"] = json!(true);
    assert!(plan("createAlarm", Some(&b)).is_err());
    let mut b = create(); b["weekdays"] = json!([1,1]);
    assert!(plan("createAlarm", Some(&b)).is_err());
    let mut b = create(); b["volume"] = json!(1.5);
    assert!(plan("createAlarm", Some(&b)).is_err());
}
#[test]
fn transition_rejects_stale_type_missing_time_and_snooze_mismatch() {
    let mut b = transition(); b["expected_generation"] = json!(-1);
    assert!(plan("transitionOccurrence", Some(&b)).is_err());
    let mut b = transition(); b.as_object_mut().unwrap().remove("client_time");
    assert!(plan("transitionOccurrence", Some(&b)).is_err());
    let mut b = transition(); b["event"] = json!("snooze");
    assert!(plan("transitionOccurrence", Some(&b)).is_err());
    b["snooze_until"] = json!("2026-11-15T12:33:00Z");
    assert!(plan("transitionOccurrence", Some(&b)).is_ok());
}
#[test]
fn push_changes_refuses_invalid_shape() {
    assert!(plan("pushChanges", Some(&json!({}))).is_err());
    let mut b = push(); b[0]["operation"] = json!("erase_all");
    assert!(plan("pushChanges", Some(&b)).is_err());
    let mut b = push(); b[0]["generation"] = json!(-1);
    assert!(plan("pushChanges", Some(&b)).is_err());
}
#[test]
fn request_body_is_bounded_and_get_has_no_body() {
    let b = json!({"x":"secret"});
    assert!(plan("health", Some(&b)).is_err());
    let mut b = create(); b["label"] = json!("x".repeat(MAX_REQUEST_BYTES));
    assert!(plan("createAlarm", Some(&b)).is_err());
}
fn attempt(reply: Response) -> (Result<Value,ClientError>,Records) {
    let p = plan("health",None).unwrap();
    let mut net = FakeTransport {reply:Some(reply),..FakeTransport::default()};
    let mut logs = Records::default();
    (execute(&mut net,&mut logs,&p),logs)
}
#[test]
fn rejects_redirects_and_http_failures_without_body_leakage() {
    let (result,logs) = attempt(Response {
        status:302,content_type:"text/plain".into(),bytes:b"PRIVATE".to_vec()
    });
    assert_eq!(result.unwrap_err(),ClientError::Redirect);
    assert_eq!(logs.0[0].1.outcome,"error");
    let (result,_) = attempt(Response {
        status:401,content_type:"application/json".into(),bytes:b"\"TOKEN PRIVATE\"".to_vec()
    });
    let error = result.unwrap_err();
    assert_eq!(error,ClientError::HttpStatus(401));
    assert!(!format!("{error:?}").contains("PRIVATE"));
}
#[test]
fn invalid_json_mime_duplicate_keys_and_size_are_refused() {
    for (bytes,mime) in [
        (b"{\"x\":1,\"x\":2}".to_vec(),"application/json"),
        (b"{\"nested\":{\"x\":1,\"x\":2}}".to_vec(),"application/json"),
        (b"{\"x\":NaN}".to_vec(),"application/json"),
        (b"{\"x\":1e999}".to_vec(),"application/json"),
        (b"{\"x\":".to_vec(),"application/json"),
        (vec![b'a';MAX_RESPONSE_BYTES + 1],"application/json"),
        (b"{\"ok\":true}".to_vec(),"text/plain"),
    ] {
        let (result,logs) = attempt(Response {
            status:200,content_type:mime.into(),bytes
        });
        assert_eq!(result.unwrap_err(), ClientError::InvalidResponse);
        assert_eq!(logs.0.len(),1);
    }
}
#[test]
fn safe_telemetry_contains_no_auth_or_request_body() {
    let mut net = FakeTransport::default();
    let mut logs = Records::default();
    let mut data = create(); data["label"] = json!("PERSONAL_SECRET");
    let p = plan_request(&base(), &Call {
        id:"createAlarm",bearer:Some("TOKEN_PRIVATE"),body:Some(&data),..Call::default()
    }).unwrap();
    execute(&mut net,&mut logs,&p).unwrap();
    assert_eq!(logs.0[0].0, TELEMETRY_EVENT);
    let output = format!("{:?}",logs.0);
    assert!(!output.contains("TOKEN_PRIVATE"));
    assert!(!output.contains("PERSONAL_SECRET"));
}
#[test]
fn network_error_is_sanitized_and_telemetry_emitted() {
    let p = plan("listAlarms",None).unwrap();
    let mut net = FakeTransport {fail:true,..FakeTransport::default()};
    let mut logs = Records::default();
    assert_eq!(execute(&mut net,&mut logs,&p).unwrap_err(),ClientError::Network);
    assert_eq!(logs.0.len(),1);
}
