// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
//! Generative (property-based) tests for the SLM stage of the gating
//! contract.
//!
//! Invariants under test (from the upstream delivery spec):
//! - an oracle Block is terminal: the provider is never invoked, for ANY
//!   provider outcome;
//! - the SLM decision matrix obeys the enforcement thresholds at ALL scores,
//!   including 0 and 1;
//! - the oracle Warn addend (+0.2 no-go) shifts the matrix predictably;
//! - low LLM confidence (<= 0.8) always escalates;
//! - any provider failure (timeout, transport, invalid response, outage)
//!   fails closed — never Allow/Warn, always a non-overridable Escalate;
//! - determinism: identical votes → identical verdict;
//! - responses preserve the request/correlation ID;
//! - concurrent requests never mix correlation IDs.
//!
//! The OTP arbiter counterpart of this suite lives in
//! `src/arbiter/test/` (ExUnit); audit-sink persistence invariants are
//! tested there (`audit persistence failure never allows`).

use gating_contract::{ContractRunner, GatingRequest, RefusalCode, Verdict};
use policy_oracle::{ActionType, EnforcementConfig, Proposal};
use slm_evaluator::{SlmError, SlmEvaluation, SlmProvider, SlmRequest};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use uuid::Uuid;

use proptest::prelude::*;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const CLEAN_PATH: &str = "src/main.rs";
const CLEAN_CONTENT: &str = "fn main() { println!(\"ok\"); }";
const WARN_PATH: &str = "script.rkt";
const WARN_CONTENT: &str = "#lang racket\n(displayln \"hi\")\n";
const BLOCK_PATH: &str = "tool.py";
const BLOCK_CONTENT: &str = "import os\nos.system('ls')\n";

fn proposal(path: &str, content: &str, llm_confidence: f32) -> Proposal {
    Proposal {
        id: Uuid::new_v4(),
        action_type: ActionType::CreateFile {
            path: path.to_string(),
        },
        content: content.to_string(),
        files_affected: vec![path.to_string()],
        llm_confidence,
    }
}

// ---------------------------------------------------------------------------
// Mock providers
// ---------------------------------------------------------------------------

/// Provider with a proptest-controlled outcome, counting invocations.
struct ScriptedSlm {
    calls: AtomicUsize,
    outcome: Outcome,
}

#[derive(Clone, Debug)]
enum Outcome {
    /// Echoes the request's proposal ID in the response.
    Eval {
        spirit_score: f64,
        confidence: f64,
        should_block: bool,
        reasoning: String,
    },
    /// Fails with the given error (constructed per call; SlmError is !Clone).
    Err(ErrorKind),
}

#[derive(Clone, Debug)]
enum ErrorKind {
    ModelNotLoaded,
    Inference,
    NotConfigured,
    Timeout,
    Transport,
    InvalidResponse,
}

impl ErrorKind {
    fn build(&self, msg: &str) -> SlmError {
        match self {
            ErrorKind::ModelNotLoaded => SlmError::ModelNotLoaded,
            ErrorKind::Inference => SlmError::InferenceError(msg.to_string()),
            ErrorKind::NotConfigured => SlmError::NotConfigured(msg.to_string()),
            ErrorKind::Timeout => SlmError::Timeout(msg.to_string()),
            ErrorKind::Transport => SlmError::Transport(msg.to_string()),
            ErrorKind::InvalidResponse => SlmError::InvalidResponse(msg.to_string()),
        }
    }
}

impl SlmProvider for ScriptedSlm {
    fn name(&self) -> &str {
        "scripted-test-double"
    }

    fn evaluate(&self, request: &SlmRequest) -> Result<SlmEvaluation, SlmError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        match &self.outcome {
            Outcome::Eval {
                spirit_score,
                confidence,
                should_block,
                reasoning,
            } => Ok(SlmEvaluation {
                proposal_id: request.proposal_id,
                spirit_score: *spirit_score,
                confidence: *confidence,
                reasoning: reasoning.clone(),
                should_block: *should_block,
            }),
            Outcome::Err(kind) => Err(kind.build("scripted failure")),
        }
    }
}

impl ScriptedSlm {
    fn eval(score: f64, confidence: f64, should_block: bool) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            outcome: Outcome::Eval {
                spirit_score: score,
                confidence,
                should_block,
                reasoning: "scripted reasoning".to_string(),
            },
        }
    }

    fn failing(kind: ErrorKind) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            outcome: Outcome::Err(kind),
        }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

/// Enforcement thresholds under test, sourced from the same defaults the
/// runner uses (RSR default policy).
fn enforcement() -> EnforcementConfig {
    EnforcementConfig::default()
}

/// The decision matrix the implementation must satisfy (from the spec):
/// `no_go >= block` (or `should_block`) → Block; `no_go >= escalate` or
/// `go <= 0.8` → Escalate; otherwise the oracle verdict stands.
fn predict(
    oracle_standing: Verdict,
    score: f64,
    go: f32,
    should_block: bool,
    addend: f64,
) -> Verdict {
    let e = enforcement();
    let no_go = score * e.slm_weight + addend;
    if should_block || no_go >= e.block_threshold {
        Verdict::Block
    } else if no_go >= e.escalate_threshold || go <= 0.8 {
        Verdict::Escalate
    } else {
        oracle_standing
    }
}

// ---------------------------------------------------------------------------
// Proptest strategies
// ---------------------------------------------------------------------------

fn arb_score() -> impl Strategy<Value = f64> {
    0.0f64..=1.0
}

fn arb_confidence() -> impl Strategy<Value = f64> {
    0.0f64..=1.0
}

fn arb_go() -> impl Strategy<Value = f32> {
    0.0f32..=1.0
}

fn arb_error_kind() -> impl Strategy<Value = ErrorKind> {
    prop_oneof![
        Just(ErrorKind::ModelNotLoaded),
        Just(ErrorKind::Inference),
        Just(ErrorKind::NotConfigured),
        Just(ErrorKind::Timeout),
        Just(ErrorKind::Transport),
        Just(ErrorKind::InvalidResponse),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(96))]

    /// An oracle Block is terminal for ANY provider outcome: the provider is
    /// never invoked (proposal content never leaves the process), no SLM or
    /// arbiter evidence is recorded, and the verdict is Block.
    #[test]
    fn oracle_block_is_terminal_for_any_provider_outcome(
        eval_outcome in prop_oneof![
            (arb_score(), arb_confidence(), any::<bool>())
                .prop_map(|(s, c, b)| Outcome::Eval {
                    spirit_score: s,
                    confidence: c,
                    should_block: b,
                    reasoning: "arbitrary".to_string(),
                }),
            arb_error_kind().prop_map(Outcome::Err),
        ],
    ) {
        let runner = ContractRunner::new();
        let provider = ScriptedSlm {
            calls: AtomicUsize::new(0),
            outcome: eval_outcome,
        };
        let request = GatingRequest::new(proposal(BLOCK_PATH, BLOCK_CONTENT, 0.95));
        let decision = runner
            .evaluate_with_provider(&request, &provider)
            .expect("evaluation must not error");

        prop_assert_eq!(decision.verdict, Verdict::Block);
        prop_assert_eq!(provider.calls(), 0, "provider must never be invoked on an oracle block");
        prop_assert!(decision.evaluations.slm.is_none());
        prop_assert!(decision.evaluations.arbiter.is_none());
        prop_assert_eq!(decision.processing.stages_executed, vec!["oracle".to_string()]);
        prop_assert_eq!(decision.request_id, request.request_id);
    }

    /// The clean-proposal decision matrix obeys threshold arithmetic at every
    /// score, including 0 and 1.
    #[test]
    fn clean_matrix_matches_threshold_arithmetic(
        score in arb_score(),
        should_block in any::<bool>(),
    ) {
        let runner = ContractRunner::new();
        let provider = ScriptedSlm::eval(score, 0.99, should_block);
        let request = GatingRequest::new(proposal(CLEAN_PATH, CLEAN_CONTENT, 0.95));
        let decision = runner
            .evaluate_with_provider(&request, &provider)
            .expect("evaluation must not error");

        let expected = predict(Verdict::Allow, score, 0.95, should_block, 0.0);
        prop_assert_eq!(decision.verdict, expected,
            "score {} should_block {} must follow the matrix", score, should_block);
        prop_assert_eq!(provider.calls(), 1);

        // SLM evidence and arbiter record populated and consistent.
        let slm = decision.evaluations.slm.as_ref().expect("slm stage recorded");
        prop_assert_eq!(slm.spirit_score, score);
        let arbiter = decision.evaluations.arbiter.as_ref().expect("arbiter record");
        prop_assert!(arbiter.consensus_reached);
        prop_assert_eq!(arbiter.oracle_vote, Verdict::Allow);
        prop_assert_eq!(arbiter.final_verdict, decision.verdict);
        prop_assert_eq!(arbiter.slm_weight, enforcement().slm_weight);
        match decision.verdict {
            Verdict::Block => prop_assert_eq!(arbiter.slm_vote, Verdict::Block),
            Verdict::Escalate => prop_assert_eq!(arbiter.slm_vote, Verdict::Escalate),
            standing => prop_assert_eq!(arbiter.slm_vote, Verdict::Allow, "standing {:?}", standing),
        }
        prop_assert_eq!(decision.request_id, request.request_id);
    }

    /// The oracle Warn addend (+0.2 to no-go) shifts the matrix, and a passing
    /// SLM verdict leaves the Warn standing (never upgraded to Allow).
    #[test]
    fn warn_addend_shifts_matrix_and_warn_stands(
        score in arb_score(),
    ) {
        let runner = ContractRunner::new();
        let provider = ScriptedSlm::eval(score, 0.99, false);
        let request = GatingRequest::new(proposal(WARN_PATH, WARN_CONTENT, 0.95));
        let decision = runner
            .evaluate_with_provider(&request, &provider)
            .expect("evaluation must not error");

        let expected = predict(Verdict::Warn, score, 0.95, false, 0.2);
        prop_assert_eq!(decision.verdict, expected,
            "score {} with +0.2 addend must follow the shifted matrix", score);
        let arbiter = decision.evaluations.arbiter.as_ref().expect("arbiter record");
        prop_assert_eq!(arbiter.oracle_vote, Verdict::Warn);
        if expected == Verdict::Warn {
            prop_assert!(decision.refusal.is_some(), "the oracle soft refusal is preserved");
        }
    }

    /// Score 0 never escalates or blocks; score 1 always blocks, for any
    /// SLM-side confidence.
    #[test]
    fn score_zero_and_one_obey_supremum_bounds(
        slm_confidence in arb_confidence(),
    ) {
        let runner = ContractRunner::new();

        let provider_zero = ScriptedSlm::eval(0.0, slm_confidence, false);
        let request = GatingRequest::new(proposal(CLEAN_PATH, CLEAN_CONTENT, 0.95));
        let d0 = runner
            .evaluate_with_provider(&request, &provider_zero)
            .expect("evaluation must not error");
        prop_assert_eq!(d0.verdict, Verdict::Allow, "spirit score 0 must allow");

        let provider_one = ScriptedSlm::eval(1.0, slm_confidence, false);
        let request = GatingRequest::new(proposal(CLEAN_PATH, CLEAN_CONTENT, 0.95));
        let d1 = runner
            .evaluate_with_provider(&request, &provider_one)
            .expect("evaluation must not error");
        prop_assert_eq!(d1.verdict, Verdict::Block, "spirit score 1 must block");
        prop_assert_eq!(
            d1.refusal.as_ref().map(|r| &r.code),
            Some(&RefusalCode::Spirit599OtherSpirit),
        );
    }

    /// `should_block: true` forces Block at ANY spirit score (including 0).
    #[test]
    fn should_block_flag_always_blocks(
        score in arb_score(),
        slm_confidence in arb_confidence(),
    ) {
        let runner = ContractRunner::new();
        let provider = ScriptedSlm::eval(score, slm_confidence, true);
        let request = GatingRequest::new(proposal(CLEAN_PATH, CLEAN_CONTENT, 0.95));
        let decision = runner
            .evaluate_with_provider(&request, &provider)
            .expect("evaluation must not error");
        prop_assert_eq!(decision.verdict, Verdict::Block);
    }

    /// Low LLM-side go (llm_confidence <= 0.8) always escalates, even with a
    /// perfectly clean SLM vote — a provider cannot rescue provider metadata.
    #[test]
    fn low_llm_confidence_always_escalates(
        go in 0.0f32..=0.8f32,
    ) {
        let runner = ContractRunner::new();
        let provider = ScriptedSlm::eval(0.0, 1.0, false);
        let request = GatingRequest::new(proposal(CLEAN_PATH, CLEAN_CONTENT, go));
        let decision = runner
            .evaluate_with_provider(&request, &provider)
            .expect("evaluation must not error");
        prop_assert_eq!(decision.verdict, Verdict::Escalate);
    }

    /// High go (> 0.8) with a mid-band score still escalates via no-go, and a
    /// clean score allows — predictions delegated to the shared matrix.
    #[test]
    fn high_go_follows_no_go_band(
        score in arb_score(),
        go in 0.8000001f32..=1.0f32,
    ) {
        let runner = ContractRunner::new();
        let provider = ScriptedSlm::eval(score, 0.99, false);
        let request = GatingRequest::new(proposal(CLEAN_PATH, CLEAN_CONTENT, go));
        let decision = runner
            .evaluate_with_provider(&request, &provider)
            .expect("evaluation must not error");
        prop_assert_eq!(decision.verdict, predict(Verdict::Allow, score, go, false, 0.0));
    }

    /// EVERY provider failure mode fails closed: Escalate with a
    /// non-overridable 9xx system refusal, an explicit `slm_error` stage, and
    /// no SLM/arbiter evidence — on both clean and warn fixtures.
    #[test]
    fn any_provider_failure_fails_closed(
        kind in arb_error_kind(),
        use_warn_fixture in any::<bool>(),
    ) {
        let runner = ContractRunner::new();
        let provider = ScriptedSlm::failing(kind);
        let (path, content) = if use_warn_fixture {
            (WARN_PATH, WARN_CONTENT)
        } else {
            (CLEAN_PATH, CLEAN_CONTENT)
        };
        let request = GatingRequest::new(proposal(path, content, 0.95));
        let decision = runner
            .evaluate_with_provider(&request, &provider)
            .expect("evaluation must not error");

        prop_assert_eq!(provider.calls(), 1);
        prop_assert_eq!(decision.verdict, Verdict::Escalate, "provider failure must never allow");
        let refusal = decision.refusal.as_ref().expect("refusal recorded");
        prop_assert_eq!(&refusal.code, &RefusalCode::Sys902InternalError);
        prop_assert!(!refusal.overridable, "system failures are not policy-overridable");
        prop_assert!(decision.processing.stages_executed.iter().any(|s| s == "slm_error"));
        prop_assert!(decision.evaluations.slm.is_none());
        prop_assert!(decision.evaluations.arbiter.is_none());
        prop_assert_eq!(decision.request_id, request.request_id);
    }

    /// Determinism: identical votes → identical verdict and refusal code,
    /// across two fresh runners and providers.
    #[test]
    fn identical_votes_identical_verdict(
        score in arb_score(),
        go in arb_go(),
        should_block in any::<bool>(),
        use_warn_fixture in any::<bool>(),
    ) {
        let (path, content) = if use_warn_fixture {
            (WARN_PATH, WARN_CONTENT)
        } else {
            (CLEAN_PATH, CLEAN_CONTENT)
        };
        let decisions: Vec<_> = (0..2)
            .map(|_| {
                ContractRunner::new()
                    .evaluate_with_provider(
                        &GatingRequest::new(proposal(path, content, go)),
                        &ScriptedSlm::eval(score, 0.99, should_block),
                    )
                    .expect("evaluation must not error")
            })
            .collect();

        prop_assert_eq!(decisions[0].verdict, decisions[1].verdict);
        prop_assert_eq!(
            decisions[0].refusal.as_ref().map(|r| &r.code),
            decisions[1].refusal.as_ref().map(|r| &r.code),
        );
    }

    /// The response preserves the request ID for every provider outcome.
    #[test]
    fn response_preserves_request_id(
        score in arb_score(),
        go in arb_go(),
        should_block in any::<bool>(),
    ) {
        let runner = ContractRunner::new();
        let request = GatingRequest::new(proposal(CLEAN_PATH, CLEAN_CONTENT, go));
        let decision = runner
            .evaluate_with_provider(&request, &ScriptedSlm::eval(score, 0.99, should_block))
            .expect("evaluation must not error");
        prop_assert_eq!(decision.request_id, request.request_id);
    }
}

// ---------------------------------------------------------------------------
// Concurrency: correlation IDs must never mix across in-flight requests
// ---------------------------------------------------------------------------

/// Provider whose evaluation echoes a marker found in the request content and
/// forces a Block, so the marker flows into the refusal message of the exact
/// decision belonging to that request. Sleeps an id-derived 0-4ms to maximise
/// interleaving.
struct MarkerBlockingSlm {
    calls: AtomicUsize,
}

impl SlmProvider for MarkerBlockingSlm {
    fn name(&self) -> &str {
        "marker-blocking-test-double"
    }

    fn evaluate(&self, request: &SlmRequest) -> Result<SlmEvaluation, SlmError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let jitter = request.proposal_id.as_bytes()[0] % 5;
        std::thread::sleep(std::time::Duration::from_millis(u64::from(jitter)));
        let marker = request
            .content
            .rsplit("// marker:")
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        Ok(SlmEvaluation {
            proposal_id: request.proposal_id,
            spirit_score: 1.0,
            confidence: 1.0,
            reasoning: format!("flagged {marker}"),
            should_block: true,
        })
    }
}

#[test]
fn concurrent_requests_do_not_mix_correlation_ids() {
    const THREADS: usize = 8;
    const REQ_PER_THREAD: usize = 4;

    let provider = Arc::new(MarkerBlockingSlm {
        calls: AtomicUsize::new(0),
    });

    let handles: Vec<_> = (0..THREADS)
        .map(|t| {
            let provider = Arc::clone(&provider);
            std::thread::spawn(move || {
                let runner = ContractRunner::new();
                let mut results = Vec::new();
                for r in 0..REQ_PER_THREAD {
                    let marker = format!("t{t}-r{r}");
                    let request = GatingRequest::new(proposal(
                        CLEAN_PATH,
                        &format!("{CLEAN_CONTENT} // marker:{marker}"),
                        0.95,
                    ));
                    let request_id = request.request_id;
                    let decision = runner
                        .evaluate_with_provider(&request, provider.as_ref())
                        .expect("evaluation must not error");
                    results.push((marker, request_id, decision));
                }
                results
            })
        })
        .collect();

    let mut total = 0;
    for handle in handles {
        for (marker, request_id, decision) in handle.join().expect("thread panicked") {
            total += 1;
            assert_eq!(decision.verdict, Verdict::Block);
            assert_eq!(
                decision.request_id, request_id,
                "decision must answer its own request"
            );
            let message = &decision.refusal.as_ref().expect("refusal recorded").message;
            assert!(
                message.contains(&format!("flagged {marker}")),
                "correlation mix-up: decision for marker {marker} carried {message:?}"
            );
        }
    }
    assert_eq!(total, THREADS * REQ_PER_THREAD);
    assert_eq!(
        provider.calls.load(Ordering::SeqCst),
        THREADS * REQ_PER_THREAD,
        "exactly one provider call per request passed the oracle"
    );
}
