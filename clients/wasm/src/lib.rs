//! Rust/WASM host-neutral client. JavaScript supplies Fetch and the
//! ores-otel next-loggers/v1 telemetry sink, keeping credentials out of WASM logs.

pub trait HostTransport {
    type Error;
    fn request(
        &self,
        operation: &str,
        method: &str,
        path: &str,
        body: Option<&str>,
        authorization: Option<&str>,
    ) -> Result<String, Self::Error>;
}
pub trait TelemetrySink {
    fn emit(&self, operation: &str, succeeded: bool);
}

pub struct HappyWakeyClient<T, L> {
    transport: T,
    telemetry: L,
    authorization: String,
}
impl<T: HostTransport, L: TelemetrySink> HappyWakeyClient<T, L> {
    pub fn new(transport: T, telemetry: L, bearer_token: String) -> Self {
        Self {
            transport,
            telemetry,
            authorization: format!("Bearer {bearer_token}"),
        }
    }
    fn call(
        &self,
        operation: &str,
        method: &str,
        path: &str,
        body: Option<&str>,
        auth: bool,
    ) -> Result<String, T::Error> {
        let result = self.transport.request(
            operation,
            method,
            path,
            body,
            auth.then_some(self.authorization.as_str()),
        );
        self.telemetry.emit(operation, result.is_ok());
        result
    }
    pub fn health(&self) -> Result<String, T::Error> {
        self.call("health", "GET", "/healthz", None, false)
    }
    pub fn list_alarms(&self) -> Result<String, T::Error> {
        self.call("list_alarms", "GET", "/v1/alarms", None, true)
    }
    pub fn create_alarm(&self, json: &str) -> Result<String, T::Error> {
        self.call("create_alarm", "POST", "/v1/alarms", Some(json), true)
    }
    pub fn transition_occurrence(&self, id: &str, json: &str) -> Result<String, T::Error> {
        self.call(
            "transition_occurrence",
            "POST",
            &format!("/v1/occurrences/{id}/transitions"),
            Some(json),
            true,
        )
    }
    pub fn pull_changes(&self, cursor: &str, limit: u16) -> Result<String, T::Error> {
        self.call(
            "pull_changes",
            "GET",
            &format!("/v1/sync/pull?cursor={cursor}&limit={limit}"),
            None,
            true,
        )
    }
    pub fn push_changes(&self, json: &str) -> Result<String, T::Error> {
        self.call("push_changes", "POST", "/v1/sync/push", Some(json), true)
    }
}
