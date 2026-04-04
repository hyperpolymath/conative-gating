// SPDX-License-Identifier: PMPL-1.0-or-later
// Copyright (C) 2026 Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
//
//! Criterion benchmarks for the Gating Contract.
//!
//! Measures:
//! - Policy evaluation latency (simple allow vs. complex block path)
//! - Contract validation throughput (proposals per second)
//! - Audit entry creation cost
//! - GatingRequest builder overhead
//!
//! Target baselines:
//! - Single gate decision: <1ms
//! - Throughput: >1000 proposals/sec
//! - Audit entry creation: <10µs additional overhead

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use gating_contract::{AuditEntry, ContractRunner, GatingRequest, RequestContext};
use policy_oracle::{ActionType, Policy, Proposal};
use uuid::Uuid;

// ── Fixture helpers ──────────────────────────────────────────────────────────

/// Compliant Rust proposal — exercises the Allow fast path.
fn rust_proposal() -> Proposal {
    Proposal {
        id: Uuid::new_v4(),
        action_type: ActionType::CreateFile {
            path: "src/lib.rs".to_string(),
        },
        content: "pub fn compute(x: u64) -> u64 { x.wrapping_mul(6364136223846793005) }".to_string(),
        files_affected: vec!["src/lib.rs".to_string()],
        llm_confidence: 0.97,
    }
}

/// TypeScript proposal — exercises the Block hard-violation path.
fn typescript_proposal() -> Proposal {
    Proposal {
        id: Uuid::new_v4(),
        action_type: ActionType::CreateFile {
            path: "app/handler.ts".to_string(),
        },
        content: "export function handle(req: Request): Response { return new Response('ok'); }"
            .to_string(),
        files_affected: vec!["app/handler.ts".to_string()],
        llm_confidence: 0.9,
    }
}

/// Nickel proposal — exercises the Warn / SoftConcern path.
fn nickel_proposal() -> Proposal {
    Proposal {
        id: Uuid::new_v4(),
        action_type: ActionType::CreateFile {
            path: "infra/server.ncl".to_string(),
        },
        content: "{ port = 8080, host = \"0.0.0.0\" }".to_string(),
        files_affected: vec!["infra/server.ncl".to_string()],
        llm_confidence: 0.85,
    }
}

/// Complex proposal touching many files and long content — stresses the inner loops.
fn complex_proposal() -> Proposal {
    let files: Vec<String> = (0..20)
        .map(|i| format!("src/module_{}.rs", i))
        .collect();

    let mut content = String::with_capacity(4096);
    for i in 0..50 {
        content.push_str(&format!(
            "pub fn func_{i}(x: u32) -> u32 {{ x + {i} }}\n"
        ));
    }

    Proposal {
        id: Uuid::new_v4(),
        action_type: ActionType::ModifyFile {
            path: "src/lib.rs".to_string(),
        },
        content,
        files_affected: files,
        llm_confidence: 0.92,
    }
}

/// Build a batch of N GatingRequests alternating between Rust and TypeScript.
fn mixed_requests(n: usize) -> Vec<GatingRequest> {
    (0..n)
        .map(|i| {
            let proposal = if i % 2 == 0 {
                rust_proposal()
            } else {
                typescript_proposal()
            };
            GatingRequest::new(proposal)
        })
        .collect()
}

// ── Benchmark groups ─────────────────────────────────────────────────────────

/// Benchmark: policy evaluation on simple (single-file, short-content) proposals.
///
/// Covers the full `ContractRunner::evaluate` path including oracle check,
/// verdict mapping, and `GatingDecision` construction.
fn bench_policy_evaluation_simple(c: &mut Criterion) {
    let runner = ContractRunner::new();

    let allow_request = GatingRequest::new(rust_proposal());
    let block_request = GatingRequest::new(typescript_proposal());
    let warn_request = GatingRequest::new(nickel_proposal());

    let mut group = c.benchmark_group("policy_evaluation_simple");

    group.bench_function("allow_rust", |b| {
        b.iter(|| black_box(runner.evaluate(black_box(&allow_request)).unwrap()))
    });

    group.bench_function("block_typescript", |b| {
        b.iter(|| black_box(runner.evaluate(black_box(&block_request)).unwrap()))
    });

    group.bench_function("warn_nickel", |b| {
        b.iter(|| black_box(runner.evaluate(black_box(&warn_request)).unwrap()))
    });

    group.finish();
}

/// Benchmark: policy evaluation on a complex proposal (20 files, 50 functions).
///
/// Stresses the `files_affected` iteration in `check_proposal` and the
/// content-scanning loops that check every forbidden language marker.
fn bench_policy_evaluation_complex(c: &mut Criterion) {
    let runner = ContractRunner::new();
    let request = GatingRequest::new(complex_proposal());

    let mut group = c.benchmark_group("policy_evaluation_complex");

    group.bench_function("complex_rust_proposal", |b| {
        b.iter(|| black_box(runner.evaluate(black_box(&request)).unwrap()))
    });

    group.finish();
}

/// Benchmark: contract validation throughput — proposals per second.
///
/// Measures how many gate decisions can be processed in a tight loop.
/// This is the primary throughput regression signal.
fn bench_contract_throughput(c: &mut Criterion) {
    let runner = ContractRunner::new();

    let requests10 = mixed_requests(10);
    let requests100 = mixed_requests(100);

    let mut group = c.benchmark_group("contract_throughput");

    group.throughput(Throughput::Elements(10));
    group.bench_function(BenchmarkId::new("mixed_batch", 10), |b| {
        b.iter(|| {
            for req in &requests10 {
                black_box(runner.evaluate(black_box(req)).unwrap());
            }
        })
    });

    group.throughput(Throughput::Elements(100));
    group.bench_function(BenchmarkId::new("mixed_batch", 100), |b| {
        b.iter(|| {
            for req in &requests100 {
                black_box(runner.evaluate(black_box(req)).unwrap());
            }
        })
    });

    group.finish();
}

/// Benchmark: audit entry creation from a decision.
///
/// `AuditEntry::from_decision` hashes proposal content and clones several
/// fields. This benchmark isolates that overhead from the evaluation itself.
fn bench_audit_entry_creation(c: &mut Criterion) {
    let runner = ContractRunner::new();

    let allow_req = GatingRequest::new(rust_proposal());
    let block_req = GatingRequest::new(typescript_proposal());

    let allow_decision = runner.evaluate(&allow_req).unwrap();
    let block_decision = runner.evaluate(&block_req).unwrap();

    let mut group = c.benchmark_group("audit_entry_creation");

    group.bench_function("from_allow_decision", |b| {
        b.iter(|| {
            black_box(AuditEntry::from_decision(
                black_box(&allow_req),
                black_box(&allow_decision),
            ))
        })
    });

    group.bench_function("from_block_decision", |b| {
        b.iter(|| {
            black_box(AuditEntry::from_decision(
                black_box(&block_req),
                black_box(&block_decision),
            ))
        })
    });

    // Also benchmark JSON serialisation of the audit entry.
    let audit = AuditEntry::from_decision(&allow_req, &allow_decision);
    group.bench_function("to_json_compact", |b| {
        b.iter(|| black_box(audit.to_json_compact().unwrap()))
    });

    group.bench_function("to_json_pretty", |b| {
        b.iter(|| black_box(audit.to_json_pretty().unwrap()))
    });

    group.finish();
}

/// Benchmark: GatingRequest builder overhead.
///
/// Tests the cost of constructing requests with and without optional context,
/// as these are created once per proposal in production.
fn bench_request_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("request_construction");

    group.bench_function("minimal_new", |b| {
        b.iter(|| black_box(GatingRequest::new(rust_proposal())))
    });

    group.bench_function("with_full_context", |b| {
        b.iter(|| {
            let ctx = RequestContext {
                source: "bench-runner".to_string(),
                session_id: Some(Uuid::new_v4().to_string()),
                agent_id: Some("agent-bench".to_string()),
                repository: None,
                session_history: vec![Uuid::new_v4(), Uuid::new_v4()],
                metadata: Default::default(),
            };
            black_box(GatingRequest::new(rust_proposal()).with_context(ctx))
        })
    });

    group.finish();
}

/// Benchmark: ContractRunner construction cost.
///
/// Creating a runner builds the Policy and Oracle. This is typically done
/// once at startup but is useful as a regression signal.
fn bench_runner_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("runner_construction");

    group.bench_function("new_rsr_defaults", |b| {
        b.iter(|| black_box(ContractRunner::new()))
    });

    group.bench_function("with_custom_policy", |b| {
        b.iter(|| black_box(ContractRunner::with_policy(Policy::rsr_default())))
    });

    group.finish();
}

// ── Criterion entry points ───────────────────────────────────────────────────

criterion_group!(
    contract_benches,
    bench_policy_evaluation_simple,
    bench_policy_evaluation_complex,
    bench_contract_throughput,
    bench_audit_entry_creation,
    bench_request_construction,
    bench_runner_construction,
);
criterion_main!(contract_benches);
