use std::{sync::Arc, time::Instant};

use happy_wakey_interfaces::{
    Alarm, CreateAlarmRequest, SyncChange, SyncEnvelope, TransitionAlarmRequest,
    TransitionAlarmResponse,
};
use next_loggers::{json, Logger, Map, Value};
use reqwest::{Client as HttpClient, Method, StatusCode};
use serde::de::DeserializeOwned;
use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("HTTPS is required outside an explicitly enabled loopback environment")]
    HttpsRequired,
    #[error("a Shared Auth bearer token is required")]
    MissingToken,
    #[error("invalid base URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("transport failed: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("Happy Wakey request failed ({status})")]
    Api { status: StatusCode, body: Value },
}

#[derive(Clone)]
pub struct HappyWakeyClient {
    base: Url,
    token: Option<String>,
    http: HttpClient,
    telemetry: Option<Arc<Logger>>,
}

impl HappyWakeyClient {
    pub fn new(
        base: &str,
        token: Option<String>,
        telemetry: Option<Arc<Logger>>,
    ) -> Result<Self, ClientError> {
        Self::new_with_loopback_policy(base, token, telemetry, false)
    }

    pub fn new_with_loopback_policy(
        base: &str,
        token: Option<String>,
        telemetry: Option<Arc<Logger>>,
        allow_insecure_loopback: bool,
    ) -> Result<Self, ClientError> {
        let mut base = Url::parse(base)?;
        let loopback = matches!(base.host_str(), Some("localhost" | "127.0.0.1" | "::1"));
        if base.scheme() != "https" && !(allow_insecure_loopback && loopback) {
            return Err(ClientError::HttpsRequired);
        }
        if !base.path().ends_with('/') {
            base.set_path(&format!("{}/", base.path()));
        }
        let http = HttpClient::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        Ok(Self {
            base,
            token,
            http,
            telemetry,
        })
    }

    pub async fn health(&self) -> Result<Value, ClientError> {
        self.request("health", Method::GET, "healthz", Option::<&()>::None, false)
            .await
    }
    pub async fn list_alarms(&self) -> Result<Vec<Alarm>, ClientError> {
        self.request(
            "list_alarms",
            Method::GET,
            "v1/alarms",
            Option::<&()>::None,
            true,
        )
        .await
    }
    pub async fn create_alarm(&self, request: &CreateAlarmRequest) -> Result<Alarm, ClientError> {
        self.request(
            "create_alarm",
            Method::POST,
            "v1/alarms",
            Some(request),
            true,
        )
        .await
    }
    pub async fn transition_occurrence(
        &self,
        id: &str,
        request: &TransitionAlarmRequest,
    ) -> Result<TransitionAlarmResponse, ClientError> {
        self.request(
            "transition_occurrence",
            Method::POST,
            &format!("v1/occurrences/{id}/transitions"),
            Some(request),
            true,
        )
        .await
    }
    pub async fn pull_changes(
        &self,
        cursor: &str,
        limit: u16,
    ) -> Result<SyncEnvelope, ClientError> {
        self.request(
            "pull_changes",
            Method::GET,
            &format!("v1/sync/pull?cursor={cursor}&limit={limit}"),
            Option::<&()>::None,
            true,
        )
        .await
    }
    pub async fn push_changes(&self, changes: &[SyncChange]) -> Result<SyncEnvelope, ClientError> {
        self.request(
            "push_changes",
            Method::POST,
            "v1/sync/push",
            Some(changes),
            true,
        )
        .await
    }

    async fn request<T: DeserializeOwned, B: serde::Serialize + ?Sized>(
        &self,
        operation: &str,
        method: Method,
        path: &str,
        body: Option<&B>,
        auth: bool,
    ) -> Result<T, ClientError> {
        let started = Instant::now();
        let mut request = self
            .http
            .request(method, self.base.join(path)?)
            .header("accept", "application/json");
        if auth {
            let token = self
                .token
                .as_deref()
                .filter(|value| !value.is_empty())
                .ok_or(ClientError::MissingToken)?;
            request = request.bearer_auth(token);
        }
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = request.send().await;
        match response {
            Ok(response) => {
                let status = response.status();
                if !status.is_success() {
                    let body = response.json().await.unwrap_or(Value::Null);
                    self.emit(operation, status.as_u16(), started, true);
                    return Err(ClientError::Api { status, body });
                }
                let value = response.json().await?;
                self.emit(operation, status.as_u16(), started, false);
                Ok(value)
            }
            Err(error) => {
                self.emit(operation, 0, started, true);
                Err(error.into())
            }
        }
    }

    fn emit(&self, operation: &str, status: u16, started: Instant, failed: bool) {
        let Some(logger) = &self.telemetry else {
            return;
        };
        let mut fields = Map::new();
        fields.insert("operation".into(), json!(operation));
        fields.insert("status".into(), json!(status));
        fields.insert(
            "duration_ms".into(),
            json!(started.elapsed().as_secs_f64() * 1000.0),
        );
        let event = if failed {
            logger.error(vec![json!("happy_wakey.client.request")])
        } else {
            logger.info(vec![json!("happy_wakey.client.request")])
        };
        let _ = event
            .add_fields(fields)
            .add_tags(["happy-wakey", "client"])
            .send();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tls_policy_fails_closed() {
        assert!(matches!(
            HappyWakeyClient::new("http://example.com", None, None),
            Err(ClientError::HttpsRequired)
        ));
        assert!(HappyWakeyClient::new_with_loopback_policy(
            "http://127.0.0.1:8120",
            None,
            None,
            true
        )
        .is_ok());
    }
}
