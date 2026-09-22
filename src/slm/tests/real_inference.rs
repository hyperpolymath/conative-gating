// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
//! Real model smoke tests — **ignored by default** and driven entirely by
//! the environment. They exercise the production provider paths against real
//! inference backends, never fixtures:
//!
//! Local llama.cpp (CI job `slm-real-inference` downloads a pinned binary +
//! pinned, SHA-256-verified GGUF model and runs this):
//!
//! ```sh
//! CONATIVE_LLAMA_CLI=./llama-cli CONATIVE_GGUF_MODEL=./model.gguf \
//!   cargo test -p slm-evaluator --test real_inference -- --ignored --nocapture
//! ```
//!
//! Remote OpenAI-compatible endpoint (protected CI environment; also usable
//! against a local `llama-server`):
//!
//! ```sh
//! cargo test -p slm-evaluator --features http --test real_inference -- \
//!     --ignored --nocapture \
//!   # with CONATIVE_SLM_ENDPOINT, CONATIVE_SLM_MODEL_NAME, SLM_API_KEY
//! ```
//!
//! A genuine model answer must satisfy the provider contract: JSON verdict,
//! in-range scores, correlation echo.

use slm_evaluator::{LlamaCppProvider, SlmProvider, SlmRequest};
use std::time::Instant;
use uuid::Uuid;

fn smoke_request() -> SlmRequest {
    SlmRequest {
        proposal_id: Uuid::new_v4(),
        content: "fn main() { println!(\"hello\"); }".to_string(),
        context: "policy 'RSR Default Policy': Rust and Elixir are preferred; \
                  TypeScript, Python, Go and Java are forbidden; npm requires deno"
            .to_string(),
        max_tokens: 0,
    }
}

#[test]
#[ignore = "requires CONATIVE_LLAMA_CLI + CONATIVE_GGUF_MODEL"]
fn real_llama_cpp_model_roundtrip() {
    let provider = LlamaCppProvider::from_env()
        .expect("environment must parse")
        .unwrap_or_else(|| {
            panic!(
                "real smoke test requires CONATIVE_GGUF_MODEL (and optionally \
                 CONATIVE_LLAMA_CLI); see docs/SLM_PROVIDERS.adoc"
            )
        });

    let request = smoke_request();
    let started = Instant::now();
    let evaluation = provider
        .evaluate(&request)
        .unwrap_or_else(|error| panic!("real model evaluation failed contract: {error}"));
    let elapsed = started.elapsed();

    assert_eq!(
        evaluation.proposal_id, request.proposal_id,
        "correlation id must echo the request"
    );
    assert!((0.0..=1.0).contains(&evaluation.spirit_score));
    assert!((0.0..=1.0).contains(&evaluation.confidence));

    // Surface the run for CI logs / docs/SLM_PROVIDERS.adoc benchmarking.
    eprintln!(
        "REAL-INFERENCE(llama.cpp): model={} verdict={{spirit_score: {:.2}, confidence: {:.2}, \
         should_block: {}, reasoning: {:?}}} latency={:?} tokens_budget={} cli={}",
        provider.model_path().display(),
        evaluation.spirit_score,
        evaluation.confidence,
        evaluation.should_block,
        evaluation.reasoning,
        elapsed,
        request.max_tokens,
        provider.cli_path().display(),
    );
}

#[cfg(feature = "http")]
#[test]
#[ignore = "requires CONATIVE_SLM_ENDPOINT (+ optional SLM_API_KEY)"]
fn real_http_endpoint_roundtrip() {
    let provider = slm_evaluator::HttpSlmProvider::from_env()
        .expect("environment must parse")
        .unwrap_or_else(|| {
            panic!(
                "real HTTP smoke test requires CONATIVE_SLM_ENDPOINT; see \
                 docs/SLM_PROVIDERS.adoc"
            )
        });

    let request = smoke_request();
    let started = Instant::now();
    let evaluation = provider
        .evaluate(&request)
        .unwrap_or_else(|error| panic!("real HTTP evaluation failed contract: {error}"));
    let elapsed = started.elapsed();

    assert_eq!(evaluation.proposal_id, request.proposal_id);
    assert!((0.0..=1.0).contains(&evaluation.spirit_score));
    assert!((0.0..=1.0).contains(&evaluation.confidence));

    eprintln!(
        "REAL-INFERENCE(http): url={} verdict={{spirit_score: {:.2}, confidence: {:.2}, \
         should_block: {}, reasoning: {:?}}} latency={:?}",
        provider.completions_url(),
        evaluation.spirit_score,
        evaluation.confidence,
        evaluation.should_block,
        evaluation.reasoning,
        elapsed,
    );
}
