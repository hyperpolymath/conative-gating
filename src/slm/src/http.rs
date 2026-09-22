// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
//! Remote SLM provider over HTTPS (OpenAI-compatible chat-completions).
//!
//! Enabled with `--features http`. Honours the provider contract in
//! [`crate::provider`]: deterministic decoding (`temperature: 0`,
//! `response_format: json_object`, explicit token budget), strict response
//! shape, range validation, correlation echo, fail-closed errors.
//!
//! Endpoints must be `https://`. Plain `http://` is accepted **only** for
//! loopback hosts (`127.0.0.1`, `::1`, `localhost`) so local servers (e.g.
//! `llama-server`) and test fixtures can be exercised without TLS. This is a
//! fail-closed guard against transmitting requests — and credentials — in
//! cleartext.
//!
//! The API key is read from the environment (`SLM_API_KEY`) and never
//! logged or embedded in errors.

use crate::provider::{
    build_prompt, complete_evaluation, parse_verdict, SlmProvider, DEFAULT_MAX_TOKENS,
    DEFAULT_TIMEOUT,
};
use crate::{SlmError, SlmEvaluation};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Blocking, OpenAI-compatible remote provider.
#[derive(Debug, Clone)]
pub struct HttpSlmProvider {
    endpoint: String,
    model: String,
    api_key: Option<String>,
    max_tokens: u32,
    timeout: Duration,
}

impl HttpSlmProvider {
    /// Create a provider for `endpoint` serving `model`. `endpoint` is the
    /// server base — the OpenAI path `/v1/chat/completions` is appended.
    pub fn new(
        endpoint: impl Into<String>,
        model: impl Into<String>,
        api_key: Option<String>,
    ) -> Result<Self, SlmError> {
        let endpoint = endpoint.into();
        let endpoint = endpoint.trim_end_matches('/').to_string();
        if endpoint.is_empty() {
            return Err(SlmError::NotConfigured(
                "SLM endpoint must not be empty".to_string(),
            ));
        }
        reject_plaintext_off_loopback(&endpoint)?;
        Ok(Self {
            endpoint,
            model: model.into(),
            api_key,
            max_tokens: DEFAULT_MAX_TOKENS,
            timeout: DEFAULT_TIMEOUT,
        })
    }

    /// Override the default token budget and timeout.
    pub fn with_limits(mut self, max_tokens: u32, timeout: Duration) -> Self {
        self.max_tokens = max_tokens;
        self.timeout = timeout;
        self
    }

    /// Build from the process environment:
    ///
    /// * `CONATIVE_SLM_ENDPOINT` — server base URL (required for
    ///   `Ok(Some(_))`; unset means the HTTP provider is not configured).
    /// * `CONATIVE_SLM_MODEL_NAME` — model identifier (default `local-slm`).
    /// * `SLM_API_KEY` — optional bearer token.
    /// * `CONATIVE_SLM_MAX_TOKENS` / `CONATIVE_SLM_TIMEOUT_SECS` — as for the
    ///   local provider.
    ///
    /// The endpoint and key belong in a protected GitHub Environment for CI
    /// smoke tests; they must never be exposed to untrusted PR code. See
    /// `docs/SLM_PROVIDERS.adoc`.
    pub fn from_env() -> Result<Option<Self>, SlmError> {
        let Ok(endpoint) = std::env::var("CONATIVE_SLM_ENDPOINT") else {
            return Ok(None);
        };
        let model =
            std::env::var("CONATIVE_SLM_MODEL_NAME").unwrap_or_else(|_| "local-slm".to_string());
        let api_key = std::env::var("SLM_API_KEY")
            .ok()
            .filter(|key| !key.trim().is_empty());
        let mut provider = Self::new(endpoint, model, api_key)?;
        if let Ok(tokens) = std::env::var("CONATIVE_SLM_MAX_TOKENS") {
            provider.max_tokens = tokens.parse().map_err(|_| {
                SlmError::NotConfigured("CONATIVE_SLM_MAX_TOKENS must be an integer".to_string())
            })?;
        }
        if let Ok(secs) = std::env::var("CONATIVE_SLM_TIMEOUT_SECS") {
            let secs: u64 = secs.parse().map_err(|_| {
                SlmError::NotConfigured("CONATIVE_SLM_TIMEOUT_SECS must be an integer".to_string())
            })?;
            provider.timeout = Duration::from_secs(secs);
        }
        Ok(Some(provider))
    }

    /// The full chat-completions URL (diagnostics/tests).
    pub fn completions_url(&self) -> String {
        format!("{}/v1/chat/completions", self.endpoint)
    }
}

/// Fail closed when a plaintext endpoint is not loopback.
fn reject_plaintext_off_loopback(endpoint: &str) -> Result<(), SlmError> {
    if endpoint.starts_with("https://") {
        return Ok(());
    }
    if let Some(rest) = endpoint.strip_prefix("http://") {
        let authority = rest.split('/').next().unwrap_or_default();
        // Bracketed IPv6 (`[::1]:8080`) vs. `host:port`.
        let host = if let Some(bracketed) = authority.strip_prefix('[') {
            bracketed.split(']').next().unwrap_or_default()
        } else {
            authority.split(':').next().unwrap_or_default()
        };
        if matches!(host, "127.0.0.1" | "localhost" | "::1") {
            return Ok(());
        }
    }
    Err(SlmError::NotConfigured(format!(
        "refusing non-loopback plaintext SLM endpoint (use https://, or a loopback \
         address for local servers): {endpoint}"
    )))
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: [ChatMessage<'a>; 1],
    temperature: f64,
    max_tokens: u32,
    response_format: ResponseFormat,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: String,
}

#[derive(Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    kind: &'static str,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

#[derive(Deserialize)]
struct ChatResponseMessage {
    content: String,
}

impl SlmProvider for HttpSlmProvider {
    fn name(&self) -> &str {
        "http-openai-compatible"
    }

    fn evaluate(&self, request: &crate::provider::SlmRequest) -> Result<SlmEvaluation, SlmError> {
        let tokens = match request.max_tokens {
            0 => self.max_tokens,
            requested => requested.min(self.max_tokens.max(1)),
        };
        let body = ChatRequest {
            model: &self.model,
            messages: [ChatMessage {
                role: "user",
                content: build_prompt(request),
            }],
            temperature: 0.0,
            max_tokens: tokens,
            response_format: ResponseFormat {
                kind: "json_object",
            },
        };

        let client = reqwest::blocking::Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(|error| {
                SlmError::Transport(format!("failed to build HTTP client: {error}"))
            })?;

        let mut call = client.post(self.completions_url()).json(&body);
        if let Some(key) = &self.api_key {
            call = call.bearer_auth(key);
        }

        let response = call.send().map_err(|error| {
            let hint = if error.is_timeout() {
                "request timed out"
            } else {
                "transport error"
            };
            SlmError::Transport(format!("{hint} calling SLM endpoint: {error}"))
        })?;

        let status = response.status();
        if !status.is_success() {
            let detail = response
                .text()
                .unwrap_or_default()
                .chars()
                .take(200)
                .collect::<String>();
            return Err(SlmError::Transport(format!(
                "SLM endpoint returned {status}: {detail}"
            )));
        }

        let parsed: ChatResponse = response.json().map_err(|error| {
            SlmError::InvalidResponse(format!(
                "endpoint returned non-chat-completions JSON: {error}"
            ))
        })?;
        let content = parsed
            .choices
            .first()
            .map(|choice| choice.message.content.as_str())
            .ok_or_else(|| {
                SlmError::InvalidResponse("endpoint returned zero choices".to_string())
            })?;

        // The message content must itself be the provider JSON verdict —
        // exactly like the local provider's stdout.
        let verdict = parse_verdict(content)?;
        Ok(complete_evaluation(request, verdict))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{SlmProvider, SlmRequest};
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use uuid::Uuid;

    /// Minimal std-only HTTP/1.1 test server: reads one request (headers +
    /// Content-Length body), returns a canned response, and records the
    /// request line, an optional Authorization header, and the raw body.
    struct MockServer {
        base_url: String,
        request_line: std::sync::Arc<std::sync::Mutex<String>>,
        auth_header: std::sync::Arc<std::sync::Mutex<String>>,
        request_body: std::sync::Arc<std::sync::Mutex<String>>,
        handle: Option<std::thread::JoinHandle<()>>,
    }

    impl MockServer {
        fn start(status_line: &'static str, response_body: &'static str) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
            let port = listener.local_addr().unwrap().port();
            let request_line = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
            let auth_header = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
            let request_body = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
            let (rl, ah, rb) = (
                request_line.clone(),
                auth_header.clone(),
                request_body.clone(),
            );
            let handle = std::thread::spawn(move || {
                let Ok((stream, _)) = listener.accept() else {
                    return;
                };
                let mut reader = BufReader::new(stream);
                let mut line = String::new();
                let mut content_length = 0usize;
                // Request line first.
                if reader.read_line(&mut line).is_ok() {
                    *rl.lock().unwrap() = line.trim().to_string();
                }
                // Headers until the empty line.
                loop {
                    line.clear();
                    let Ok(read) = reader.read_line(&mut line) else {
                        break;
                    };
                    if read == 0 || line == "\r\n" {
                        break;
                    }
                    let lower = line.to_lowercase();
                    if let Some(value) = lower.strip_prefix("content-length:") {
                        content_length = value.trim().parse().unwrap_or(0);
                    }
                    // Header names are case-insensitive on the wire.
                    if let Some(prefix_end) = line.find(':') {
                        if line[..prefix_end].eq_ignore_ascii_case("authorization") {
                            *ah.lock().unwrap() = line[prefix_end + 1..].trim().to_string();
                        }
                    }
                }
                // Body.
                let mut body = vec![0u8; content_length];
                let _ = reader.read_exact(&mut body);
                *rb.lock().unwrap() = String::from_utf8_lossy(&body).to_string();

                let response = format!(
                    "{status_line}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{response_body}",
                    response_body.len()
                );
                let _ = reader.get_mut().write_all(response.as_bytes());
            });
            Self {
                base_url: format!("http://127.0.0.1:{port}"),
                request_line,
                auth_header,
                request_body,
                handle: Some(handle),
            }
        }

        fn join(mut self) -> (String, String, String) {
            if let Some(handle) = self.handle.take() {
                let _ = handle.join();
            }
            let take = |m: &std::sync::Mutex<String>| m.lock().unwrap().clone();
            (
                take(&self.request_line),
                take(&self.auth_header),
                take(&self.request_body),
            )
        }
    }

    fn request() -> SlmRequest {
        SlmRequest {
            proposal_id: Uuid::new_v4(),
            content: "fn main() {}".to_string(),
            context: "unit test".to_string(),
            max_tokens: 32,
        }
    }

    const VERDICT_JSON: &str = r#"{\"spirit_score\": 0.2, \"confidence\": 0.9, \"reasoning\": \"mock\", \"should_block\": false}"#;

    #[test]
    fn posts_openai_shape_and_echoes_correlation() {
        let chat_response = format!(
            r#"{{"id":"chatcmpl-mock","choices":[{{"index":0,"message":{{"role":"assistant","content":"{VERDICT_JSON}"}}}}]}}"#
        );
        let server =
            MockServer::start("HTTP/1.1 200 OK", Box::leak(chat_response.into_boxed_str()));
        let provider =
            HttpSlmProvider::new(&server.base_url, "mock-model", Some("test-key".into())).unwrap();

        let req = request();
        let evaluation = provider.evaluate(&req).unwrap();
        assert_eq!(evaluation.proposal_id, req.proposal_id);
        assert!(!evaluation.should_block);
        assert_eq!(evaluation.spirit_score, 0.2);

        let (request_line, auth, body) = server.join();
        assert_eq!(request_line, "POST /v1/chat/completions HTTP/1.1");
        assert_eq!(auth, "Bearer test-key");
        assert!(body.contains(r#""model":"mock-model""#));
        assert!(body.contains(r#""temperature":0.0"#));
        assert!(body.contains(r#""max_tokens":32"#));
        assert!(body.contains(r#""json_object""#));
    }

    #[test]
    fn server_error_is_fail_closed() {
        let server = MockServer::start("HTTP/1.1 500 Internal Server Error", "{}");
        let provider = HttpSlmProvider::new(&server.base_url, "mock-model", None).unwrap();
        assert!(matches!(
            provider.evaluate(&request()),
            Err(SlmError::Transport(_))
        ));
        server.join();
    }

    #[test]
    fn zero_choices_is_fail_closed() {
        let server = MockServer::start("HTTP/1.1 200 OK", r#"{"choices":[]}"#);
        let provider = HttpSlmProvider::new(&server.base_url, "mock-model", None).unwrap();
        assert!(matches!(
            provider.evaluate(&request()),
            Err(SlmError::InvalidResponse(_))
        ));
        server.join();
    }

    #[test]
    fn malformed_content_json_is_fail_closed() {
        let server = MockServer::start(
            "HTTP/1.1 200 OK",
            r#"{"choices":[{"message":{"role":"assistant","content":"no verdict here"}}]}"#,
        );
        let provider = HttpSlmProvider::new(&server.base_url, "mock-model", None).unwrap();
        assert!(matches!(
            provider.evaluate(&request()),
            Err(SlmError::InvalidResponse(_))
        ));
        server.join();
    }

    #[test]
    fn plaintext_non_loopback_is_refused() {
        let result = HttpSlmProvider::new("http://slm.example.com", "m", None);
        assert!(matches!(result, Err(SlmError::NotConfigured(_))));
    }

    #[test]
    fn loopback_and_https_endpoints_accepted() {
        assert!(HttpSlmProvider::new("http://127.0.0.1:8080", "m", None).is_ok());
        assert!(HttpSlmProvider::new("http://localhost:8080", "m", None).is_ok());
        assert!(HttpSlmProvider::new("http://[::1]:8080/", "m", None).is_ok());
        assert!(HttpSlmProvider::new("https://slm.example.com/", "m", None).is_ok());
    }

    #[test]
    fn completions_url_strips_trailing_slash() {
        let provider = HttpSlmProvider::new("https://slm.example.com/", "m", None).unwrap();
        assert_eq!(
            provider.completions_url(),
            "https://slm.example.com/v1/chat/completions"
        );
    }
}
