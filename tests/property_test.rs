// SPDX-License-Identifier: PMPL-1.0-or-later
//! Property-Based Tests for Conative Gating
//!
//! Uses invariant testing to verify contract properties:
//! - Determinism: same input → same verdict
//! - Binary outcomes: verdict is always defined
//! - Bounded processing: evaluation completes in finite time

use gating_contract::{ContractRunner, GatingRequest, Verdict};
use policy_oracle::{ActionType, Proposal};
use uuid::Uuid;

fn create_proposal(path: &str, content: &str) -> Proposal {
    Proposal {
        id: Uuid::new_v4(),
        action_type: ActionType::CreateFile {
            path: path.to_string(),
        },
        content: content.to_string(),
        files_affected: vec![path.to_string()],
        llm_confidence: 0.95,
    }
}

#[test]
fn property_determinism_same_input_same_verdict() {
    let runner = ContractRunner::new();

    // Test with various inputs
    let test_cases = vec![
        ("lib.rs", "fn test() {}"),
        ("main.rs", "fn main() { println!(\"hi\"); }"),
        ("config.ts", "const x: string = 'y';"),
        ("script.py", "print('hello')"),
    ];

    for (path, content) in test_cases {
        let proposal1 = create_proposal(path, content);
        let request1 = GatingRequest::new(proposal1);

        let proposal2 = create_proposal(path, content);
        let request2 = GatingRequest::new(proposal2);

        let decision1 = runner.evaluate(&request1).expect("eval 1");
        let decision2 = runner.evaluate(&request2).expect("eval 2");

        // Same input should produce same verdict (content-wise)
        assert_eq!(
            decision1.verdict, decision2.verdict,
            "Verdict should be deterministic for path={}, content={}",
            path, content
        );

        // Same category if refusal
        if decision1.refusal.is_some() && decision2.refusal.is_some() {
            assert_eq!(
                decision1.refusal.as_ref().unwrap().category,
                decision2.refusal.as_ref().unwrap().category,
                "Refusal category should be deterministic"
            );
        }
    }
}

#[test]
fn property_verdict_is_always_defined() {
    let runner = ContractRunner::new();

    let long_content = "x".repeat(10000);
    let test_inputs = vec![
        ("empty.rs", "".to_string()),
        ("normal.rs", "fn main() {}".to_string()),
        ("complex.rs", "pub mod test { pub fn x() { } }".to_string()),
        ("unicode.rs", "// 你好世界\nfn main() {}".to_string()),
        ("long.rs", long_content),
    ];

    for (path, content) in test_inputs {
        let proposal = create_proposal(path, &content);
        let request = GatingRequest::new(proposal);

        let decision = runner.evaluate(&request).expect("should evaluate");

        // Verdict must always be one of the four defined values
        match decision.verdict {
            Verdict::Allow | Verdict::Warn | Verdict::Escalate | Verdict::Block => {
                // All verdicts are valid
            }
        }

        // If Block or Warn, refusal must be present
        if matches!(decision.verdict, Verdict::Block | Verdict::Warn) {
            assert!(
                decision.refusal.is_some(),
                "Block/Warn verdict must have refusal details"
            );
        }

        // If Allow, refusal must be absent
        if decision.verdict == Verdict::Allow {
            assert!(
                decision.refusal.is_none(),
                "Allow verdict must not have refusal"
            );
        }
    }
}

#[test]
fn property_processing_time_bounded() {
    let runner = ContractRunner::new();

    // Test multiple proposals to ensure processing completes
    for i in 0..100 {
        let content = format!("fn test_{}() {{}}", i);
        let proposal = create_proposal("lib.rs", &content);
        let request = GatingRequest::new(proposal);

        let start = std::time::Instant::now();
        let decision = runner.evaluate(&request).expect("should evaluate");
        let elapsed = start.elapsed();

        // Processing time should be finite and reasonable (< 1 second)
        assert!(elapsed.as_secs() < 1, "Processing took too long: {:?}", elapsed);

        // Metadata should record duration
        assert!(decision.processing.duration_us > 0);
        assert!(decision.processing.duration_us < 1_000_000); // < 1 second in microseconds
    }
}

#[test]
fn property_no_panic_on_pathological_input() {
    let runner = ContractRunner::new();

    let long_name = "verylongname".repeat(100);
    let pathological_inputs = vec![
        ("path with spaces.rs", "fn x() {}"),
        ("path/with/many/segments/deep.rs", "fn x() {}"),
        ("path-with-dashes.rs", "fn x() {}"),
        ("path_with_underscores.rs", "fn x() {}"),
        ("path.multiple.dots.rs", "fn x() {}"),
        (".hidden.rs", "fn x() {}"),
        ("", "fn x() {}"), // empty path
        ("noextension", "fn x() {}"),
        (long_name.as_str(), "fn x() {}"),
    ];

    for (path, content) in pathological_inputs {
        let proposal = create_proposal(path, content);
        let request = GatingRequest::new(proposal);

        // Should not panic - evaluation should complete
        let _ = runner.evaluate(&request);
    }
}

#[test]
fn property_refusal_evidence_when_blocked() {
    let runner = ContractRunner::new();

    let blocking_proposals = vec![
        ("main.ts", "const x: string = 'test';", true),  // TypeScript
        ("script.py", "import os", true),                // Python
        ("main.go", "func main() {}", true),             // Go
        ("config.rs", r#"pwd = "secret123456""#, true),  // Secret
    ];

    for (path, content, should_have_evidence) in blocking_proposals {
        let proposal = create_proposal(path, content);
        let request = GatingRequest::new(proposal);

        let decision = runner.evaluate(&request).expect("should evaluate");

        if should_have_evidence && decision.verdict == Verdict::Block {
            let refusal = decision.refusal.expect("Block should have refusal");
            // Evidence should support the refusal (though may be empty for some patterns)
            // At minimum, the refusal should have a message
            assert!(!refusal.message.is_empty(), "Refusal must have a message");
        }
    }
}

#[test]
fn property_exit_codes_consistent() {
    // Verify exit code mapping is consistent
    assert_eq!(Verdict::Allow.exit_code(), 0);
    assert_eq!(Verdict::Block.exit_code(), 1);
    assert_eq!(Verdict::Warn.exit_code(), 2);
    assert_eq!(Verdict::Escalate.exit_code(), 3);

    // Exit codes should be distinct
    let codes = vec![
        Verdict::Allow.exit_code(),
        Verdict::Block.exit_code(),
        Verdict::Warn.exit_code(),
        Verdict::Escalate.exit_code(),
    ];

    for i in 0..codes.len() {
        for j in (i + 1)..codes.len() {
            assert_ne!(
                codes[i], codes[j],
                "Exit codes must be distinct: {:?}",
                codes
            );
        }
    }
}

#[test]
fn property_proposal_id_preserved() {
    let runner = ContractRunner::new();

    for _ in 0..50 {
        let proposal = create_proposal("test.rs", "fn test() {}");
        let proposal_id = proposal.id;

        let request = GatingRequest::new(proposal);
        let decision = runner.evaluate(&request).expect("should evaluate");

        // Proposal ID should be preserved through the pipeline
        assert_eq!(decision.request_id, request.request_id);

        // Oracle evaluation should have the proposal ID
        if let Some(oracle_eval) = &decision.evaluations.oracle {
            assert_eq!(oracle_eval.proposal_id, proposal_id);
        }
    }
}

#[test]
fn property_allowed_tier1_languages() {
    let runner = ContractRunner::new();

    let tier1_languages = vec![
        ("main.rs", "fn main() {}"),
        ("module.ex", "defmodule M do end"),
        ("main.zig", "const std = @import(\"std\");"),
        ("main.adb", "procedure M is begin null; end M;"),
        ("main.hs", "module M where"),
        ("component.res", "@react.component let make = () => null"),
    ];

    for (path, content) in tier1_languages {
        let proposal = create_proposal(path, content);
        let request = GatingRequest::new(proposal);

        let decision = runner.evaluate(&request).expect("should evaluate");

        // Tier 1 languages should not be blocked
        assert!(
            decision.verdict == Verdict::Allow,
            "Tier 1 language {} should be allowed, got {:?}",
            path,
            decision.verdict
        );
    }
}

#[test]
fn property_forbidden_languages_always_blocked() {
    let runner = ContractRunner::new();

    let forbidden_languages = vec![
        ("main.ts", "const x: string = 'test';"),
        ("main.py", "import os"),
        ("main.go", "package main"),
        ("Main.java", "public class Main"),
    ];

    for (path, content) in forbidden_languages {
        let proposal = create_proposal(path, content);
        let request = GatingRequest::new(proposal);

        let decision = runner.evaluate(&request).expect("should evaluate");

        // Forbidden languages should always be blocked or warned
        assert!(
            matches!(decision.verdict, Verdict::Block | Verdict::Warn),
            "Forbidden language {} should be blocked or warned, got {:?}",
            path,
            decision.verdict
        );
    }
}

#[test]
fn property_audit_entry_serializable() {
    use gating_contract::RequestContext;

    let runner = ContractRunner::new();

    let contexts = vec![
        RequestContext {
            source: "test".to_string(),
            ..Default::default()
        },
        RequestContext {
            source: "test".to_string(),
            session_id: Some("sess-1".to_string()),
            ..Default::default()
        },
        RequestContext {
            source: "test".to_string(),
            agent_id: Some("agent-1".to_string()),
            ..Default::default()
        },
    ];

    for context in contexts {
        let proposal = create_proposal("test.rs", "fn test() {}");
        let mut request = GatingRequest::new(proposal);
        request.context = context;

        let decision = runner.evaluate(&request).expect("should evaluate");
        let audit = runner.audit(&request, &decision);

        // Audit should be serializable
        let json = audit.to_json();
        assert!(json.is_ok(), "Audit should serialize to JSON");

        let pretty = audit.to_json_pretty();
        assert!(pretty.is_ok(), "Audit should pretty-print");

        let compact = audit.to_json_compact();
        assert!(compact.is_ok(), "Audit should compact-print");
    }
}
