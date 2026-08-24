import gleam/result

pub type Client {
  Client(
    base_url: String,
    authorization: String,
    transport: fn(String, String, String, String) -> Result(String, String),
    telemetry: fn(String, Bool) -> Nil,
  )
}

fn call(
  client: Client,
  operation: String,
  method: String,
  path: String,
  body: String,
  auth: Bool,
) -> Result(String, String) {
  let Client(base, authorization, transport, telemetry) = client
  let header = case auth {
    True -> authorization
    False -> ""
  }
  let outcome = transport(method, base <> path, header, body)
  telemetry(operation, result.is_ok(outcome))
  outcome
}

pub fn health(c: Client) {
  call(c, "health", "GET", "/healthz", "", False)
}

pub fn list_alarms(c: Client) {
  call(c, "list_alarms", "GET", "/v1/alarms", "", True)
}

pub fn create_alarm(c: Client, json: String) {
  call(c, "create_alarm", "POST", "/v1/alarms", json, True)
}

pub fn transition_occurrence(c: Client, id: String, json: String) {
  call(
    c,
    "transition_occurrence",
    "POST",
    "/v1/occurrences/" <> id <> "/transitions",
    json,
    True,
  )
}

pub fn pull_changes(c: Client, cursor: String) {
  call(c, "pull_changes", "GET", "/v1/sync/pull?cursor=" <> cursor, "", True)
}

pub fn push_changes(c: Client, json: String) {
  call(c, "push_changes", "POST", "/v1/sync/push", json, True)
}
// The telemetry callback must emit ores-otel next-loggers/v1 without authorization or bodies.
