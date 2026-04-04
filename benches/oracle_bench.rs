// SPDX-License-Identifier: PMPL-1.0-or-later
// Copyright (C) 2026 Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
//
//! Criterion benchmarks for the Policy Oracle.
//!
//! Measures:
//! - Language detection latency (single file extension, batch 10, batch 100)
//! - Tier classification (TypeScript=forbidden, Rust=allowed, Nickel=tier2)
//! - Violation severity computation (critical vs high vs no violation)
//! - Rule count queries per language (rules_checked length)
//!
//! Target baselines:
//! - Single proposal evaluation: <500µs
//! - Batch 100 proposals: <50ms total
//! - No panics on pathological input

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use policy_oracle::{ActionType, Oracle, Policy, Proposal};
use uuid::Uuid;

// ── Fixture helpers ──────────────────────────────────────────────────────────

/// Build a minimal compliant Rust proposal.
fn rust_proposal() -> Proposal {
    Proposal {
        id: Uuid::new_v4(),
        action_type: ActionType::CreateFile {
            path: "src/lib.rs".to_string(),
        },
        content: "pub fn add(a: u32, b: u32) -> u32 { a + b }".to_string(),
        files_affected: vec!["src/lib.rs".to_string()],
        llm_confidence: 0.95,
    }
}

/// Build a forbidden TypeScript proposal (triggers Critical violation).
fn typescript_proposal() -> Proposal {
    Proposal {
        id: Uuid::new_v4(),
        action_type: ActionType::CreateFile {
            path: "util.ts".to_string(),
        },
        content: "const greet = (name: string): string => `Hello, ${name}`;".to_string(),
        files_affected: vec!["util.ts".to_string()],
        llm_confidence: 0.9,
    }
}

/// Build a Tier-2 Nickel proposal (triggers SoftConcern, no block).
fn nickel_proposal() -> Proposal {
    Proposal {
        id: Uuid::new_v4(),
        action_type: ActionType::CreateFile {
            path: "config.ncl".to_string(),
        },
        content: "{ server_port = 8080 }".to_string(),
        files_affected: vec!["config.ncl".to_string()],
        llm_confidence: 0.8,
    }
}

/// Build a proposal containing a hardcoded secret (high-severity forbidden pattern).
fn secret_proposal() -> Proposal {
    Proposal {
        id: Uuid::new_v4(),
        action_type: ActionType::CreateFile {
            path: "src/config.rs".to_string(),
        },
        content: r#"let api_key = "supersecretkey12345""#.to_string(),
        files_affected: vec!["src/config.rs".to_string()],
        llm_confidence: 0.9,
    }
}

/// Build a batch of N proposals that alternate between Rust (allowed) and TypeScript (blocked).
fn mixed_batch(n: usize) -> Vec<Proposal> {
    (0..n)
        .map(|i| {
            if i % 2 == 0 {
                rust_proposal()
            } else {
                typescript_proposal()
            }
        })
        .collect()
}

// ── Benchmark groups ─────────────────────────────────────────────────────────

/// Benchmark: language detection via file extension.
///
/// Covers the hot path inside `check_proposal` that iterates over
/// `policy.languages.forbidden` and calls `file_matches_language`.
fn bench_language_detection_single(c: &mut Criterion) {
    let oracle = Oracle::with_rsr_defaults();

    let ts_proposal = typescript_proposal();
    let rs_proposal = rust_proposal();

    let mut group = c.benchmark_group("language_detection_single");

    // Forbidden language: TypeScript (.ts extension, typed content)
    group.bench_function("typescript_forbidden", |b| {
        b.iter(|| {
            black_box(oracle.check_proposal(black_box(&ts_proposal)).unwrap())
        })
    });

    // Tier-1 language: Rust (should be Compliant)
    group.bench_function("rust_allowed", |b| {
        b.iter(|| {
            black_box(oracle.check_proposal(black_box(&rs_proposal)).unwrap())
        })
    });

    group.finish();
}

/// Benchmark: batch language detection — 10 and 100 proposals.
///
/// Models a CI job scanning multiple files in a single pass.
fn bench_language_detection_batch(c: &mut Criterion) {
    let oracle = Oracle::with_rsr_defaults();
    let batch10 = mixed_batch(10);
    let batch100 = mixed_batch(100);

    let mut group = c.benchmark_group("language_detection_batch");

    group.throughput(Throughput::Elements(10));
    group.bench_function(BenchmarkId::new("mixed_batch", 10), |b| {
        b.iter(|| {
            for proposal in &batch10 {
                black_box(oracle.check_proposal(black_box(proposal)).unwrap());
            }
        })
    });

    group.throughput(Throughput::Elements(100));
    group.bench_function(BenchmarkId::new("mixed_batch", 100), |b| {
        b.iter(|| {
            for proposal in &batch100 {
                black_box(oracle.check_proposal(black_box(proposal)).unwrap());
            }
        })
    });

    group.finish();
}

/// Benchmark: tier classification.
///
/// Exercises the three tiers:
/// - Tier 1 (Rust)     → Compliant
/// - Tier 2 (Nickel)   → SoftConcern
/// - Forbidden (TypeScript) → HardViolation
fn bench_tier_classification(c: &mut Criterion) {
    let oracle = Oracle::with_rsr_defaults();

    let tier1_proposal = rust_proposal();
    let tier2_proposal = nickel_proposal();
    let forbidden_proposal = typescript_proposal();

    let mut group = c.benchmark_group("tier_classification");

    group.bench_function("tier1_rust", |b| {
        b.iter(|| {
            let eval = oracle.check_proposal(black_box(&tier1_proposal)).unwrap();
            black_box(eval)
        })
    });

    group.bench_function("tier2_nickel", |b| {
        b.iter(|| {
            let eval = oracle.check_proposal(black_box(&tier2_proposal)).unwrap();
            black_box(eval)
        })
    });

    group.bench_function("forbidden_typescript", |b| {
        b.iter(|| {
            let eval = oracle.check_proposal(black_box(&forbidden_proposal)).unwrap();
            black_box(eval)
        })
    });

    group.finish();
}

/// Benchmark: violation severity computation.
///
/// Compares the cost of evaluating a clean proposal (no violations) against
/// proposals that trigger Critical (TypeScript), High (hardcoded secret), and
/// Medium-risk (Nickel concern) code paths.
fn bench_violation_severity(c: &mut Criterion) {
    let oracle = Oracle::with_rsr_defaults();

    let clean = rust_proposal();
    let critical = typescript_proposal();
    let secret = secret_proposal();
    let concern = nickel_proposal();

    let mut group = c.benchmark_group("violation_severity");

    group.bench_function("no_violation_compliant", |b| {
        b.iter(|| black_box(oracle.check_proposal(black_box(&clean)).unwrap()))
    });

    group.bench_function("critical_forbidden_language", |b| {
        b.iter(|| black_box(oracle.check_proposal(black_box(&critical)).unwrap()))
    });

    group.bench_function("high_hardcoded_secret", |b| {
        b.iter(|| black_box(oracle.check_proposal(black_box(&secret)).unwrap()))
    });

    group.bench_function("soft_concern_tier2", |b| {
        b.iter(|| black_box(oracle.check_proposal(black_box(&concern)).unwrap()))
    });

    group.finish();
}

/// Benchmark: rule count queries per language.
///
/// Measures how many rules the oracle checks per call and the cost of
/// accessing `evaluation.rules_checked.len()` on the returned value.
fn bench_rule_count_queries(c: &mut Criterion) {
    let oracle = Oracle::with_rsr_defaults();

    // Build a range of proposals covering different languages to ensure
    // the benchmark exercises the full rule-check loop for each.
    let proposals: Vec<(&str, Proposal)> = vec![
        ("rust", rust_proposal()),
        ("typescript", typescript_proposal()),
        ("nickel", nickel_proposal()),
        ("secret_in_rs", secret_proposal()),
    ];

    let mut group = c.benchmark_group("rule_count_queries");

    for (label, proposal) in &proposals {
        group.bench_function(*label, |b| {
            b.iter(|| {
                let eval = oracle.check_proposal(black_box(proposal)).unwrap();
                // Intentionally access rules_checked so the compiler cannot elide it.
                black_box(eval.rules_checked.len())
            })
        });
    }

    group.finish();
}

/// Benchmark: custom vs. default policy construction cost.
///
/// Policy::rsr_default() builds heap-allocated Vecs. This benchmark
/// measures how much that setup costs relative to evaluation itself.
fn bench_policy_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("policy_construction");

    group.bench_function("rsr_default_policy", |b| {
        b.iter(|| black_box(Policy::rsr_default()))
    });

    group.bench_function("oracle_from_rsr_default", |b| {
        b.iter(|| black_box(Oracle::new(Policy::rsr_default())))
    });

    group.finish();
}

// ── Criterion entry points ───────────────────────────────────────────────────

criterion_group!(
    oracle_benches,
    bench_language_detection_single,
    bench_language_detection_batch,
    bench_tier_classification,
    bench_violation_severity,
    bench_rule_count_queries,
    bench_policy_construction,
);
criterion_main!(oracle_benches);
