// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
//! SLM provider adapters: local `llama.cpp` CLI and (optionally) remote HTTPS.
//!
//! Both providers honour the same contract:
//!
//! * **Deterministic decoding** — temperature 0, explicit token limit.
//! * **Strict response shape** — the model must answer with a JSON object
//!   `{"spirit_score", "confidence", "reasoning", "should_block"}`. Malformed
//!   output is rejected, never coerced into a "safe-looking" result.
//! * **Range validation** — scores must be finite and within `0..=1`.
//! * **Correlation** — the request's `proposal_id` is carried through and
//!   echoed on the evaluation; providers never invent one.
//! * **Fail-closed** — every failure mode is a [`SlmError`] variant, and
//!   downstream callers (contract runner, arbiter) must treat provider
//!   failure as NO-GO.
//!
//! Configuration is explicit: nothing downloads models or probes for binaries
//! at runtime. See `docs/SLM_PROVIDERS.adoc`.

use crate::{SlmError, SlmEvaluation};
use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Maximum proposal content forwarded to a provider (chars). Guards argv
/// limits for the CLI provider and keeps prompts bounded.
pub const MAX_CONTENT_CHARS: usize = 4096;

/// Maximum context forwarded to a provider (chars).
pub const MAX_CONTEXT_CHARS: usize = 1024;

/// Default decoding token budget.
/// Default per-request decoding budget. 256 gives small instruct models
/// enough room to emit long-form reasoning AND still close the JSON object:
/// at 128 tokens SmolLM2-135M/Qwen2.5-0.5B responses were observed truncated
/// mid-object, which correctly fails closed but wastes the evaluation.
pub const DEFAULT_MAX_TOKENS: u32 = 256;

/// Default provider timeout.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);

/// A correlated evaluation request handed to a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlmRequest {
    /// Correlation ID linking this request to a proposal — must be preserved
    /// end-to-end so audit records can be joined to the gating request.
    pub proposal_id: Uuid,
    /// Proposal content under evaluation (bounded, see [`MAX_CONTENT_CHARS`]).
    pub content: String,
    /// Policy/request context (bounded, see [`MAX_CONTEXT_CHARS`]).
    pub context: String,
    /// Decoding token budget for this request.
    pub max_tokens: u32,
}

/// An SLM backend able to evaluate requests.
pub trait SlmProvider: Send + Sync {
    /// Human-readable provider name for diagnostics and audit context.
    fn name(&self) -> &str;
    /// Evaluate a request, honouring the module-level provider contract.
    fn evaluate(&self, request: &SlmRequest) -> Result<SlmEvaluation, SlmError>;
}

/// Trim `text` to at most `max` chars on a char boundary.
fn bounded(text: &str, max: usize) -> &str {
    if text.chars().count() <= max {
        text
    } else {
        match text.char_indices().nth(max) {
            Some((idx, _)) => &text[..idx],
            None => text,
        }
    }
}

/// Build the deterministic evaluation prompt shared by all providers.
pub fn build_prompt(request: &SlmRequest) -> String {
    let content = bounded(&request.content, MAX_CONTENT_CHARS);
    let context = bounded(&request.context, MAX_CONTEXT_CHARS);
    format!(
        "You are a policy-spirit evaluator in a code-gating system. The deterministic \
         oracle has already ruled on explicit rules; you judge whether the proposal \
         violates the SPIRIT of the policy (e.g. disguised intent, verbosity abuse, \
         over-documentation to hide complexity, structural evasion).\n\n\
         Respond with ONLY a JSON object, no prose, no markdown fences, exactly:\n\
         {{\"spirit_score\": <number 0..1>, \"confidence\": <number 0..1>, \
         \"reasoning\": \"<one short sentence>\", \"should_block\": <true|false>}}\n\n\
         - spirit_score: estimated probability that the proposal violates the spirit of policy\n\
         - confidence: your confidence in that estimate\n\
         - should_block: true only if it must be blocked outright\n\n\
         POLICY CONTEXT:\n{context}\n\nPROPOSAL UNDER EVALUATION:\n{content}"
    )
}

/// Wire format of the model's answer (validated before use).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderVerdict {
    /// Estimated probability the proposal violates the spirit of policy.
    pub spirit_score: f64,
    /// Model confidence in its estimate.
    pub confidence: f64,
    /// Short human-readable justification.
    pub reasoning: String,
    /// Hard-block recommendation.
    pub should_block: bool,
}

impl ProviderVerdict {
    /// Enforce the provider contract: finite in-range scores, bounded reasoning.
    pub fn validate(&self) -> Result<(), SlmError> {
        for (name, score) in [
            ("spirit_score", self.spirit_score),
            ("confidence", self.confidence),
        ] {
            if !score.is_finite() || !(0.0..=1.0).contains(&score) {
                return Err(SlmError::InvalidResponse(format!(
                    "{name} must be a finite number within 0..=1, got {score}"
                )));
            }
        }
        if self.reasoning.chars().count() > 1000 {
            return Err(SlmError::InvalidResponse(
                "reasoning exceeds 1000 characters".to_string(),
            ));
        }
        Ok(())
    }
}

/// Extract ALL balanced top-level `{...}` JSON objects from `raw`, respecting
/// string literals and escapes (models sometimes wrap the object in prose).
fn extract_json_objects(raw: &str) -> Vec<&str> {
    let mut objects = Vec::new();
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut start = 0usize;
    for (offset, ch) in raw.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' if in_string => escaped = true,
            '"' => in_string = !in_string,
            '{' if !in_string => {
                if depth == 0 {
                    start = offset;
                }
                depth += 1;
            }
            '}' if !in_string && depth != 0 => {
                depth -= 1;
                if depth == 0 {
                    objects.push(&raw[start..=offset]);
                }
            }
            _ => {}
        }
    }
    objects
}

/// Parse provider output text into a validated [`ProviderVerdict`].
///
/// Real CLI front-ends (notably current `llama-cli` conversation mode) pollute
/// stdout with banners, an echo of the prompt — which itself contains an
/// *invalid* template of the verdict object (`<number 0..1>` placeholders) —
/// and throughput stats. The assistant's answer is the final output block, so
/// this scans every balanced object and accepts the LAST one that satisfies
/// the verdict schema and range validation. Anything else is treated as noise;
/// if no object validates, the response fails closed as invalid.
pub fn parse_verdict(raw: &str) -> Result<ProviderVerdict, SlmError> {
    let objects = extract_json_objects(raw);
    if objects.is_empty() {
        return Err(SlmError::InvalidResponse(
            "no JSON object found in provider output".to_string(),
        ));
    }
    let mut last_error = String::new();
    for object in objects.iter().rev() {
        match serde_json::from_str::<ProviderVerdict>(object)
            .map_err(|error| {
                SlmError::InvalidResponse(format!(
                    "provider output failed schema validation: {error}"
                ))
            })
            .and_then(|verdict| verdict.validate().map(|()| verdict))
        {
            Ok(verdict) => return Ok(verdict),
            Err(error) => last_error = error.to_string(),
        }
    }
    Err(SlmError::InvalidResponse(format!(
        "no valid verdict object in provider output ({} object candidates; last error: {last_error})",
        objects.len()
    )))
}

/// Join a validated verdict with the request it answers, preserving correlation.
pub(crate) fn complete_evaluation(request: &SlmRequest, verdict: ProviderVerdict) -> SlmEvaluation {
    SlmEvaluation {
        proposal_id: request.proposal_id,
        spirit_score: verdict.spirit_score,
        confidence: verdict.confidence,
        reasoning: verdict.reasoning,
        should_block: verdict.should_block,
    }
}

// ============ Local llama.cpp provider ============

/// Provider that shells out to a `llama.cpp`-compatible executable.
///
/// Invocation (explicit, deterministic, per the provider contract):
///
/// ```text
/// llama-cli -m MODEL -p PROMPT -n TOKENS --temp 0 --no-display-prompt --single-turn
/// ```
///
/// `--no-display-prompt` and `--single-turn` are additive to the documented
/// argument set: the first keeps stdout JSON-extractable, the second
/// guarantees the process terminates after one generation instead of
/// parking in llama-cli's interactive conversation loop (observed with
/// llama.cpp nightlies, where plain `-p` never exits). Nothing is
/// downloaded or auto-discovered: both the executable and the GGUF model
/// path must be configured explicitly.
#[derive(Debug, Clone)]
pub struct LlamaCppProvider {
    cli_path: PathBuf,
    model_path: PathBuf,
    max_tokens: u32,
    timeout: Duration,
}

impl LlamaCppProvider {
    /// Create a provider; the model file must exist (fail fast with a clear
    /// error rather than a confusing model-load failure at first request).
    pub fn new(
        cli_path: impl Into<PathBuf>,
        model_path: impl Into<PathBuf>,
    ) -> Result<Self, SlmError> {
        let model_path = model_path.into();
        if !model_path.is_file() {
            return Err(SlmError::NotConfigured(format!(
                "GGUF model not found: {}",
                model_path.display()
            )));
        }
        Ok(Self {
            cli_path: cli_path.into(),
            model_path,
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
    /// * `CONATIVE_GGUF_MODEL` — path to the GGUF model (required for
    ///   `Ok(Some(_))`; unset means the local provider is not configured).
    /// * `CONATIVE_LLAMA_CLI` — executable path/name (default `llama-cli`).
    /// * `CONATIVE_SLM_MAX_TOKENS` — token budget (default [`DEFAULT_MAX_TOKENS`]).
    /// * `CONATIVE_SLM_TIMEOUT_SECS` — timeout seconds (default 120).
    pub fn from_env() -> Result<Option<Self>, SlmError> {
        let Ok(model) = std::env::var("CONATIVE_GGUF_MODEL") else {
            return Ok(None);
        };
        let cli = std::env::var("CONATIVE_LLAMA_CLI").unwrap_or_else(|_| "llama-cli".to_string());
        let mut provider = Self::new(cli, model)?;
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

    /// Path of the configured model (diagnostics only).
    pub fn model_path(&self) -> &Path {
        &self.model_path
    }

    /// Path of the configured executable (diagnostics only).
    pub fn cli_path(&self) -> &Path {
        &self.cli_path
    }
}

impl SlmProvider for LlamaCppProvider {
    fn name(&self) -> &str {
        "llama.cpp-cli"
    }

    fn evaluate(&self, request: &SlmRequest) -> Result<SlmEvaluation, SlmError> {
        let prompt = build_prompt(request);
        // Per-request budget, defaulting to — and capped by — the provider's
        // configured budget. `0` means "use the provider budget".
        let tokens = match request.max_tokens {
            0 => self.max_tokens,
            requested => requested.min(self.max_tokens.max(1)),
        };

        let mut child = Command::new(&self.cli_path)
            .arg("-m")
            .arg(&self.model_path)
            .arg("-p")
            .arg(&prompt)
            .arg("-n")
            .arg(tokens.to_string())
            .arg("--temp")
            .arg("0")
            .arg("--no-display-prompt")
            .arg("--single-turn")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| {
                SlmError::Transport(format!(
                    "failed to spawn {}: {error}",
                    self.cli_path.display()
                ))
            })?;

        // Drain pipes on threads so a verbose child cannot deadlock on a full
        // stderr/stdout buffer while the main thread enforces the timeout.
        let mut stdout_reader = child.stdout.take().expect("invariant: stdout was piped");
        let mut stderr_reader = child.stderr.take().expect("invariant: stderr was piped");
        let stdout_thread = std::thread::spawn(move || {
            let mut buffer = String::new();
            let _ = stdout_reader.read_to_string(&mut buffer);
            buffer
        });
        let stderr_thread = std::thread::spawn(move || {
            let mut buffer = String::new();
            let _ = stderr_reader.read_to_string(&mut buffer);
            buffer
        });

        let started = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => {
                    if started.elapsed() >= self.timeout {
                        let _ = child.kill();
                        let _ = child.wait();
                        let _ = stdout_thread.join();
                        let _ = stderr_thread.join();
                        return Err(SlmError::Timeout(format!(
                            "{} did not answer within {:?}",
                            self.cli_path.display(),
                            self.timeout
                        )));
                    }
                    std::thread::sleep(Duration::from_millis(25));
                }
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(SlmError::Transport(format!(
                        "failed while waiting on {}: {error}",
                        self.cli_path.display()
                    )));
                }
            }
        };

        let stdout = stdout_thread.join().unwrap_or_default();
        let stderr = stderr_thread.join().unwrap_or_default();

        if !status.success() {
            return Err(SlmError::Transport(format!(
                "{} exited with {status}: {}",
                self.cli_path.display(),
                stderr.chars().take(200).collect::<String>()
            )));
        }

        let verdict = parse_verdict(&stdout)?;
        Ok(complete_evaluation(request, verdict))
    }
}

// ============ Provider selection from the environment ============

/// Select a provider from the environment:
///
/// * `CONATIVE_SLM_PROVIDER=none` (or unset) → `Ok(None)`
/// * `CONATIVE_SLM_PROVIDER=llama` → [`LlamaCppProvider::from_env`]
/// * `CONATIVE_SLM_PROVIDER=http` → `HttpSlmProvider::from_env` (requires the
///   crate's `http` feature; without it this is a fail-closed error)
pub fn from_env() -> Result<Option<std::sync::Arc<dyn SlmProvider>>, SlmError> {
    let provider = std::env::var("CONATIVE_SLM_PROVIDER")
        .unwrap_or_else(|_| "none".to_string())
        .to_lowercase();
    match provider.trim() {
        "" | "none" | "disabled" => Ok(None),
        "llama" | "llamacpp" | "llama-cpp" | "llama-cli" => Ok(LlamaCppProvider::from_env()?
            .map(|p| std::sync::Arc::new(p) as std::sync::Arc<dyn SlmProvider>)),
        #[cfg(feature = "http")]
        "http" | "https" | "openai" | "remote" => Ok(crate::http::HttpSlmProvider::from_env()?
            .map(|p| std::sync::Arc::new(p) as std::sync::Arc<dyn SlmProvider>)),
        #[cfg(not(feature = "http"))]
        "http" | "https" | "openai" | "remote" => Err(SlmError::NotConfigured(
            "CONATIVE_SLM_PROVIDER=http requested but this build lacks the `http` feature; \
             rebuild with --features http"
                .to_string(),
        )),
        other => Err(SlmError::NotConfigured(format!(
            "unknown CONATIVE_SLM_PROVIDER={other} (expected: none | llama | http)"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_is_deterministic_and_bounded() {
        let request = SlmRequest {
            proposal_id: Uuid::nil(),
            content: "x".repeat(10_000),
            context: "ctx".to_string(),
            max_tokens: 64,
        };
        let a = build_prompt(&request);
        let b = build_prompt(&request);
        assert_eq!(a, b);
        assert!(a.contains("POLICY CONTEXT"));
        assert!(a.chars().count() < 10_000);
    }

    #[test]
    fn parse_accepts_clean_json() {
        let verdict = parse_verdict(
            r#"{"spirit_score": 0.1, "confidence": 0.9, "reasoning": "fine", "should_block": false}"#,
        )
        .unwrap();
        assert!(!verdict.should_block);
        assert_eq!(verdict.spirit_score, 0.1);
    }

    #[test]
    fn parse_extracts_json_from_prose() {
        let raw = r#"Sure! Here is my answer:
        {"spirit_score": 0.8, "confidence": 0.7, "reasoning": "uses strings like }", "should_block": true}
        Hope that helps!"#;
        let verdict = parse_verdict(raw).unwrap();
        assert!(verdict.should_block);
        assert_eq!(verdict.reasoning, "uses strings like }");
    }

    #[test]
    fn parse_rejects_out_of_range_scores() {
        let raw =
            r#"{"spirit_score": 1.5, "confidence": 0.7, "reasoning": "x", "should_block": false}"#;
        assert!(matches!(
            parse_verdict(raw),
            Err(SlmError::InvalidResponse(_))
        ));
    }

    #[test]
    fn parse_rejects_non_json() {
        assert!(matches!(
            parse_verdict("definitely not json"),
            Err(SlmError::InvalidResponse(_))
        ));
    }

    #[test]
    fn parse_rejects_schema_mismatch() {
        let raw = r#"{"spirit_score": 0.1, "confidence": 0.2}"#;
        assert!(matches!(
            parse_verdict(raw),
            Err(SlmError::InvalidResponse(_))
        ));
    }

    #[test]
    fn llama_provider_requires_existing_model() {
        let result = LlamaCppProvider::new("llama-cli", "/nonexistent/model.gguf");
        assert!(matches!(result, Err(SlmError::NotConfigured(_))));
    }

    #[test]
    fn provider_echoes_correlation_id() {
        // A fake `llama-cli` shell script emitting a valid verdict; asserts
        // the provider's stdout parsing and correlation echo end to end.
        let dir = std::env::temp_dir().join(format!("conative-slm-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let model = dir.join("model.gguf");
        std::fs::write(&model, b"gguf-fixture").unwrap();
        let cli = dir.join("fake-llama-cli.sh");
        std::fs::write(
            &cli,
            "#!/bin/sh\nprintf '%s' '{\"spirit_score\": 0.2, \"confidence\": 0.9, \"reasoning\": \"fixture\", \"should_block\": false}'\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let provider = LlamaCppProvider::new(&cli, &model).unwrap();
        let request = SlmRequest {
            proposal_id: Uuid::new_v4(),
            content: "fn main() {}".to_string(),
            context: "test".to_string(),
            max_tokens: 16,
        };
        let evaluation = provider.evaluate(&request).unwrap();
        assert_eq!(evaluation.proposal_id, request.proposal_id);
        assert!(!evaluation.should_block);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn provider_rejects_garbage_output_fail_closed() {
        let dir = std::env::temp_dir().join(format!("conative-slm-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let model = dir.join("model.gguf");
        std::fs::write(&model, b"gguf-fixture").unwrap();
        let cli = dir.join("fake-llama-cli.sh");
        std::fs::write(&cli, "#!/bin/sh\necho 'I cannot answer that.'\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let provider = LlamaCppProvider::new(&cli, &model).unwrap();
        let request = SlmRequest {
            proposal_id: Uuid::new_v4(),
            content: "fn main() {}".to_string(),
            context: "test".to_string(),
            max_tokens: 16,
        };
        assert!(matches!(
            provider.evaluate(&request),
            Err(SlmError::InvalidResponse(_))
        ));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn provider_times_out_fail_closed() {
        let dir = std::env::temp_dir().join(format!("conative-slm-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let model = dir.join("model.gguf");
        std::fs::write(&model, b"gguf-fixture").unwrap();
        let cli = dir.join("fake-llama-cli.sh");
        std::fs::write(&cli, "#!/bin/sh\nsleep 5\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let provider = LlamaCppProvider::new(&cli, &model)
            .unwrap()
            .with_limits(16, Duration::from_millis(300));
        let request = SlmRequest {
            proposal_id: Uuid::new_v4(),
            content: "fn main() {}".to_string(),
            context: "test".to_string(),
            max_tokens: 16,
        };
        assert!(matches!(
            provider.evaluate(&request),
            Err(SlmError::Timeout(_))
        ));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
