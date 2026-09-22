// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
//! Client for the OTP Consensus Arbiter's versioned JSON-lines protocol.
//!
//! Protocol version 1 (see `docs/ARBITER_PROTOCOL.adoc` and
//! `src/arbiter/lib/conative_gating/`):
//!
//! ```text
//! request:  {"protocol_version":1,"request_id":"…",
//!            "llm":{"confidence":0.95},
//!            "slm":{"violation_confidence":0.05},
//!            "oracle":{"verdict":"allow"}}
//! response: {"protocol_version":1,"request_id":"…",
//!            "verdict":"allow|escalate|block","reason":"…","audit_recorded":true}
//! error:    {"protocol_version":1,"request_id":"…","error":"…"}
//! ```
//!
//! Design decisions:
//!
//! * **One arbiter process per decision** (spawn per call): stateless, robust
//!   against a wedged service, and trivially timeout-enforced. The arbiter is
//!   cheap to start relative to an SLM inference.
//! * **Correlation is mandatory**: the response's `request_id` must equal the
//!   request's. Per-call processes make cross-request mix-ups structurally
//!   impossible; the check is still enforced and tested.
//! * **Audit must be confirmed**: `audit_recorded: true` is required. The
//!   "every accepted request has exactly one audit record" invariant is
//!   enforced client-side — a decision the arbiter did not durably record is
//!   treated as [`ArbiterError::AuditNotConfirmed`] and fails closed.
//! * **Every failure is [`ArbiterError`]**: callers must map arbiter failure
//!   to Escalate/NO-GO, never to Allow.

use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use thiserror::Error;
use uuid::Uuid;

/// Wire protocol version spoken by this client.
pub const ARBITER_PROTOCOL_VERSION: u32 = 1;

/// Default arbiter timeout (the arbiter adds negligible latency to an SLM
/// call; 30s is generous).
pub const DEFAULT_ARBITER_TIMEOUT: Duration = Duration::from_secs(30);

/// The oracle's vote on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OracleVote {
    /// No violation.
    Allow,
    /// Soft concern raised.
    SoftConcern,
    /// Hard violation (arbiter must answer `block`).
    HardViolation,
}

/// A consensus decision returned by the arbiter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArbiterVerdict {
    /// Proposal may proceed.
    Allow,
    /// Route to human review.
    Escalate,
    /// Proposal is rejected.
    Block,
}

/// Protocol v1 consensus request.
#[derive(Debug, Clone, Serialize)]
pub struct ArbiterRequest {
    protocol_version: u32,
    request_id: Uuid,
    llm: VoteConfidence,
    slm: VoteViolation,
    oracle: OracleBallot,
}

#[derive(Debug, Clone, Serialize)]
struct VoteConfidence {
    confidence: f64,
}

#[derive(Debug, Clone, Serialize)]
struct VoteViolation {
    violation_confidence: f64,
}

#[derive(Debug, Clone, Serialize)]
struct OracleBallot {
    verdict: OracleVote,
}

/// Parsed protocol v1 response (before validation).
#[derive(Debug, Deserialize)]
struct WireResponse {
    protocol_version: u32,
    request_id: Uuid,
    verdict: Option<ArbiterVerdict>,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    audit_recorded: Option<bool>,
    #[serde(default)]
    error: Option<String>,
}

/// A validated consensus response.
#[derive(Debug, Clone, PartialEq)]
pub struct ArbiterDecision {
    /// Correlated request ID (always equals the request's).
    pub request_id: Uuid,
    /// Consensus verdict.
    pub verdict: ArbiterVerdict,
    /// Optional arbiter-provided reason (diagnostics).
    pub reason: Option<String>,
}

/// Every client failure mode (fail-closed: never map these to Allow).
#[derive(Error, Debug)]
pub enum ArbiterError {
    /// Spawning or talking to the arbiter process failed.
    #[error("arbiter transport failure: {0}")]
    Transport(String),
    /// The arbiter did not answer within the timeout.
    #[error("arbiter timeout: {0}")]
    Timeout(String),
    /// The arbiter exited without producing a response line.
    #[error("arbiter closed without answering: {0}")]
    Closed(String),
    /// The response line was not a well-formed protocol message.
    #[error("malformed arbiter response: {0}")]
    Malformed(String),
    /// The response declared an unsupported protocol version.
    #[error("unsupported arbiter protocol version {0} (client speaks {ARBITER_PROTOCOL_VERSION})")]
    ProtocolVersion(u32),
    /// The response answered a different request than the one sent.
    #[error("arbiter correlation mismatch: expected {expected}, got {got}")]
    CorrelationMismatch {
        /// The request_id of the request actually sent.
        expected: Uuid,
        /// The request_id present in the response.
        got: Uuid,
    },
    /// The arbiter reported an application-level error.
    #[error("arbiter service error: {0}")]
    Service(String),
    /// The arbiter answered a decision it did not durably audit.
    #[error("arbiter decision without confirmed audit record")]
    AuditNotConfirmed,
}

/// Client spawning one short-lived arbiter process per decision.
#[derive(Debug)]
pub struct ArbiterClient {
    command: Vec<String>,
    timeout: Duration,
}

impl ArbiterClient {
    /// `command` is the full invocation (program + args) of a protocol v1
    /// arbiter, e.g. `["/path/to/conative_arbiter"]` or
    /// `["escript", "arbiter_protocol.exs"]`.
    pub fn new(command: &[&str], timeout: Duration) -> Result<Self, ArbiterError> {
        if command.is_empty() {
            return Err(ArbiterError::Transport("empty arbiter command".to_string()));
        }
        Ok(Self {
            command: command.iter().map(ToString::to_string).collect(),
            timeout,
        })
    }

    /// Build from `CONATIVE_ARBITER_CMD` (split on whitespace, e.g.
    /// `escript src/arbiter/priv/arbiter_protocol.exs`). Unset → `Ok(None)`.
    pub fn from_env() -> Result<Option<Self>, ArbiterError> {
        let Ok(raw) = std::env::var("CONATIVE_ARBITER_CMD") else {
            return Ok(None);
        };
        let parts: Vec<&str> = raw.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(None);
        }
        Ok(Some(Self::new(&parts, DEFAULT_ARBITER_TIMEOUT)?))
    }

    /// Ask the arbiter for a consensus decision.
    ///
    /// If the arbiter closes its input before the request is fully written,
    /// the response still determines the outcome: no response returns
    /// [`ArbiterError::Closed`], while malformed output returns
    /// [`ArbiterError::Malformed`]. Other request-write failures return
    /// [`ArbiterError::Transport`].
    pub fn decide(
        &self,
        llm_confidence: f64,
        violation_confidence: f64,
        oracle_vote: OracleVote,
    ) -> Result<ArbiterDecision, ArbiterError> {
        let request = ArbiterRequest {
            protocol_version: ARBITER_PROTOCOL_VERSION,
            request_id: Uuid::new_v4(),
            llm: VoteConfidence {
                confidence: llm_confidence,
            },
            slm: VoteViolation {
                violation_confidence,
            },
            oracle: OracleBallot {
                verdict: oracle_vote,
            },
        };
        let line = serde_json::to_string(&request)
            .map_err(|error| ArbiterError::Malformed(error.to_string()))?;

        let mut child = Command::new(&self.command[0])
            .args(&self.command[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| {
                ArbiterError::Transport(format!(
                    "failed to spawn arbiter {}: {error}",
                    self.command[0]
                ))
            })?;

        // Write the request and close stdin so stream-driven servers exit.
        //
        // A BrokenPipe here is NOT fatal: a fast, broken, or non-protocol
        // arbiter may exit before the write lands, closing the read end of
        // our stdin pipe. That is exactly the failure mode the read/validate
        // path below already classifies correctly (no answer → `Closed`,
        // garbage answer → `Malformed`), so swallow EPIPE and let the
        // exit/read path decide. Any other write error is a real transport
        // failure.
        let write_result = child
            .stdin
            .take()
            .expect("invariant: stdin piped")
            .write_all(line.as_bytes());
        if let Err(error) = write_result {
            if error.kind() != std::io::ErrorKind::BrokenPipe {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ArbiterError::Transport(format!(
                    "write to arbiter stdin: {error}"
                )));
            }
        }

        let mut stdout = child.stdout.take().expect("invariant: stdout piped");
        let reader = std::thread::spawn(move || {
            let mut reader = BufReader::new(&mut stdout);
            let mut line = String::new();
            let _ = reader.read_line(&mut line);
            line
        });

        let started = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => {
                    if started.elapsed() >= self.timeout {
                        let _ = child.kill();
                        let _ = child.wait();
                        let _ = reader.join();
                        return Err(ArbiterError::Timeout(format!(
                            "no answer within {:?}",
                            self.timeout
                        )));
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(ArbiterError::Transport(format!(
                        "waiting on arbiter failed: {error}"
                    )));
                }
            }
        };

        let answer = reader.join().unwrap_or_default();
        let answer = answer.trim();
        if answer.is_empty() {
            return Err(ArbiterError::Closed(format!(
                "arbiter exited {status} with no response"
            )));
        }
        Self::validate_response(&request, answer)
    }

    /// Validate a response line against the request (extracted for testing).
    fn validate_response(
        request: &ArbiterRequest,
        answer: &str,
    ) -> Result<ArbiterDecision, ArbiterError> {
        let wire: WireResponse = serde_json::from_str(answer)
            .map_err(|error| ArbiterError::Malformed(format!("{error} (line: {answer:.200})")))?;

        if wire.protocol_version != ARBITER_PROTOCOL_VERSION {
            return Err(ArbiterError::ProtocolVersion(wire.protocol_version));
        }
        if wire.request_id != request.request_id {
            return Err(ArbiterError::CorrelationMismatch {
                expected: request.request_id,
                got: wire.request_id,
            });
        }
        if let Some(error) = wire.error {
            return Err(ArbiterError::Service(error));
        }
        if wire.audit_recorded != Some(true) {
            return Err(ArbiterError::AuditNotConfirmed);
        }
        let verdict = wire
            .verdict
            .ok_or_else(|| ArbiterError::Malformed("missing verdict".to_string()))?;
        Ok(ArbiterDecision {
            request_id: wire.request_id,
            verdict,
            reason: wire.reason,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("conative-arbiter-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[cfg(unix)]
    fn make_script(dir: &std::path::Path, name: &str, body: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join(name);
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    /// A well-behaved protocol v1 server: echoes the request_id, answers with
    /// a fixed verdict, confirms its audit record.
    #[cfg(unix)]
    fn good_server(dir: &std::path::Path, verdict: &str) -> std::path::PathBuf {
        make_script(
            dir,
            "arbiter-good.sh",
            &format!(
                "IFS= read -r line\nid=$(printf '%s' \"$line\" | sed -n 's/.*\"request_id\"[ ]*:[ ]*\"\\([^\"]*\\)\".*/\\1/p')\nprintf '%s\\n' '{{\"protocol_version\":1,\"request_id\":\"'\"$id\"'\",\"verdict\":\"{verdict}\",\"reason\":\"fixture\",\"audit_recorded\":true}}'\n"
            ),
        )
    }

    #[cfg(unix)]
    fn client_for(script: &std::path::Path) -> ArbiterClient {
        let s = script.to_string_lossy().to_string();
        ArbiterClient::new(&[s.as_str()], Duration::from_secs(10)).unwrap()
    }

    #[test]
    #[cfg(unix)]
    fn good_arbiter_allow() {
        let dir = fixture_dir();
        let script = good_server(&dir, "allow");
        let client = client_for(&script);
        let decision = client.decide(0.95, 0.05, OracleVote::Allow).unwrap();
        assert_eq!(decision.verdict, ArbiterVerdict::Allow);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    #[cfg(unix)]
    fn good_arbiter_block_on_hard_violation() {
        let dir = fixture_dir();
        let script = good_server(&dir, "block");
        let client = client_for(&script);
        let decision = client.decide(0.99, 0.0, OracleVote::HardViolation).unwrap();
        assert_eq!(decision.verdict, ArbiterVerdict::Block);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    #[cfg(unix)]
    fn protocol_version_mismatch_fails_closed() {
        let dir = fixture_dir();
        let script = make_script(
            &dir,
            "arbiter-v2.sh",
            "IFS= read -r line\nid=$(printf '%s' \"$line\" | sed -n 's/.*\"request_id\"[ ]*:[ ]*\"\\([^\"]*\\)\".*/\\1/p')\nprintf '%s\\n' '{\"protocol_version\":2,\"request_id\":\"'\"$id\"'\",\"verdict\":\"allow\",\"audit_recorded\":true}'\n",
        );
        let client = client_for(&script);
        assert!(matches!(
            client.decide(0.5, 0.5, OracleVote::Allow),
            Err(ArbiterError::ProtocolVersion(2))
        ));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    #[cfg(unix)]
    fn correlation_mismatch_fails_closed() {
        let dir = fixture_dir();
        let script = make_script(
            &dir,
            "arbiter-wrongid.sh",
            "IFS= read -r line\nprintf '%s\\n' '{\"protocol_version\":1,\"request_id\":\"00000000-0000-0000-0000-000000000000\",\"verdict\":\"allow\",\"audit_recorded\":true}'\n",
        );
        let client = client_for(&script);
        assert!(matches!(
            client.decide(0.5, 0.5, OracleVote::Allow),
            Err(ArbiterError::CorrelationMismatch { .. })
        ));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    #[cfg(unix)]
    fn service_error_fails_closed() {
        let dir = fixture_dir();
        let script = make_script(
            &dir,
            "arbiter-error.sh",
            "IFS= read -r line\nid=$(printf '%s' \"$line\" | sed -n 's/.*\"request_id\"[ ]*:[ ]*\"\\([^\"]*\\)\".*/\\1/p')\nprintf '%s\\n' '{\"protocol_version\":1,\"request_id\":\"'\"$id\"'\",\"error\":\"consensus unavailable\"}'\n",
        );
        let client = client_for(&script);
        assert!(matches!(
            client.decide(0.5, 0.5, OracleVote::Allow),
            Err(ArbiterError::Service(_))
        ));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    #[cfg(unix)]
    fn missing_audit_confirmation_fails_closed() {
        let dir = fixture_dir();
        let script = make_script(
            &dir,
            "arbiter-noaudit.sh",
            "IFS= read -r line\nid=$(printf '%s' \"$line\" | sed -n 's/.*\"request_id\"[ ]*:[ ]*\"\\([^\"]*\\)\".*/\\1/p')\nprintf '%s\\n' '{\"protocol_version\":1,\"request_id\":\"'\"$id\"'\",\"verdict\":\"allow\",\"audit_recorded\":false}'\n",
        );
        let client = client_for(&script);
        assert!(matches!(
            client.decide(0.5, 0.5, OracleVote::Allow),
            Err(ArbiterError::AuditNotConfirmed)
        ));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    #[cfg(unix)]
    fn garbage_output_fails_closed() {
        let dir = fixture_dir();
        let script = make_script(&dir, "arbiter-garbage.sh", "echo 'not json at all'\n");
        let client = client_for(&script);
        let result = client.decide(0.5, 0.5, OracleVote::Allow);
        assert!(
            matches!(result, Err(ArbiterError::Malformed(_))),
            "garbage output must classify as Malformed, got {result:?}"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    #[cfg(unix)]
    fn closed_output_fails_closed() {
        let dir = fixture_dir();
        let script = make_script(&dir, "arbiter-closed.sh", "exit 0\n");
        let client = client_for(&script);
        let result = client.decide(0.5, 0.5, OracleVote::Allow);
        assert!(
            matches!(result, Err(ArbiterError::Closed(_))),
            "immediate exit must classify as Closed, got {result:?}"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    #[cfg(unix)]
    fn timeout_fails_closed() {
        let dir = fixture_dir();
        let script = make_script(&dir, "arbiter-slow.sh", "sleep 5\n");
        let s = script.to_string_lossy().to_string();
        let client = ArbiterClient::new(&[s.as_str()], Duration::from_millis(300)).unwrap();
        assert!(matches!(
            client.decide(0.5, 0.5, OracleVote::Allow),
            Err(ArbiterError::Timeout(_))
        ));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    #[cfg(unix)]
    fn verdict_shape_is_serialized_as_protocol_v1() {
        // Serialized request must exactly match the documented wire shape.
        let request = ArbiterRequest {
            protocol_version: 1,
            request_id: Uuid::nil(),
            llm: VoteConfidence { confidence: 0.95 },
            slm: VoteViolation {
                violation_confidence: 0.05,
            },
            oracle: OracleBallot {
                verdict: OracleVote::SoftConcern,
            },
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["protocol_version"], 1);
        assert_eq!(json["request_id"], Uuid::nil().to_string());
        assert_eq!(json["llm"]["confidence"], 0.95);
        assert_eq!(json["slm"]["violation_confidence"], 0.05);
        assert_eq!(json["oracle"]["verdict"], "soft_concern");
    }
}
