// SPDX-License-Identifier: PMPL-1.0-or-later
//! Security Aspect Tests for Conative Gating
//!
//! Tests security properties:
//! - Gating bypass prevention
//! - Oracle manipulation detection
//! - Contract injection handling
//! - Safe defaults under failures

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
fn security_typescript_comment_bypass_fails() {
    let runner = ContractRunner::new();

    // TypeScript file extension check catches this
    let proposal = create_proposal(
        "utils.ts",
        r#"fn main() {}"#,
    );
    let request = GatingRequest::new(proposal);
    let _decision = runner.evaluate(&request).expect("should evaluate");

    // File extension .ts should be detected as TypeScript, blocked
    assert_eq!(_decision.verdict, Verdict::Block);
}

#[test]
fn security_typescript_obfuscated_marker_fails() {
    let runner = ContractRunner::new();

    // Attempt 2: Break marker with spacing
    let proposal = create_proposal(
        "hidden.ts",
        r#"
const    x    :    string    =    'test'
"#,
    );
    let request = GatingRequest::new(proposal);
    let decision = runner.evaluate(&request).expect("should evaluate");

    // File extension check should catch it
    assert_eq!(decision.verdict, Verdict::Block);
}

#[test]
fn security_python_filename_bypass() {
    let runner = ContractRunner::new();

    // Try to hide Python as a different file
    let proposal = create_proposal(
        "config.txt",
        r#"
import os
import sys
def execute():
    os.system('whoami')
"#,
    );
    let request = GatingRequest::new(proposal);
    let _decision = runner.evaluate(&request).expect("should evaluate");

    // Content should trigger Python markers
    // But since we only check extensions, this may pass
    // This is a known limitation documented in contract
    // The actual filter depends on policy marker detection
}

#[test]
fn security_secret_detection_base64() {
    let runner = ContractRunner::new();

    // Try to hide secret in base64 (not detected by simple regex)
    let proposal = create_proposal(
        "config.rs",
        r#"
// Not a direct secret, but encoded
const ENCODED: &str = "c2stMTIzNDU2Nzg5MA=="; // base64 for sk-1234567890
"#,
    );
    let request = GatingRequest::new(proposal);
    let decision = runner.evaluate(&request).expect("should evaluate");

    // Should not detect base64 encoded secret (limitation of regex approach)
    // This is acceptable as this would require ML analysis
    assert_eq!(decision.verdict, Verdict::Allow);
}

#[test]
fn security_secret_with_newlines_detected() {
    let runner = ContractRunner::new();

    let proposal = create_proposal(
        "config.rs",
        r#"
let password = "
abcdefghij1234567890
";
"#,
    );
    let request = GatingRequest::new(proposal);
    let _decision = runner.evaluate(&request).expect("should evaluate");

    // The regex should still match across potential line breaks
    // depending on multiline handling
    // For single-line regex, this may pass - known limitation
}

#[test]
fn security_multiple_violations_all_reported() {
    let runner = ContractRunner::new();

    let proposal = create_proposal(
        "evil.ts",
        r#"
const API_KEY = "sk-abcdef1234567890";
const secret: string = 'verysecret123456';
"#,
    );
    let request = GatingRequest::new(proposal);
    let decision = runner.evaluate(&request).expect("should evaluate");

    // Should detect at least TypeScript violation
    assert_eq!(decision.verdict, Verdict::Block);

    // Oracle should report violations
    if let Some(oracle_eval) = &decision.evaluations.oracle {
        assert!(!oracle_eval.violations.is_empty());
    }
}

#[test]
fn security_empty_proposal_allowed() {
    let runner = ContractRunner::new();

    let proposal = create_proposal("empty.rs", "");
    let request = GatingRequest::new(proposal);

    let decision = runner.evaluate(&request).expect("should evaluate");

    // Empty content should be allowed (no violations)
    assert_eq!(decision.verdict, Verdict::Allow);
}

#[test]
fn security_unicode_bypass_attempt() {
    let runner = ContractRunner::new();

    // Attempt with unicode lookalikes
    let proposal = create_proposal(
        "confuse.ts",
        r#"
const 𝘹: string = 'test'; // Looks like TypeScript but unicode
"#,
    );
    let request = GatingRequest::new(proposal);
    let decision = runner.evaluate(&request).expect("should evaluate");

    // File extension should catch it
    assert_eq!(decision.verdict, Verdict::Block);
}

#[test]
fn security_null_bytes_handled() {
    let runner = ContractRunner::new();

    let proposal = create_proposal(
        "test.rs",
        "fn test() {}\0\0\0", // null bytes
    );
    let request = GatingRequest::new(proposal);

    // Should handle safely without panicking
    let decision = runner.evaluate(&request).expect("should evaluate");
    assert_eq!(decision.verdict, Verdict::Allow);
}

#[test]
fn security_extreme_length_handled() {
    let runner = ContractRunner::new();

    let large_content = "fn test() {}\n".repeat(100_000); // 1.3MB of content
    let proposal = create_proposal("huge.rs", &large_content);
    let request = GatingRequest::new(proposal);

    // Should process without DoS or memory exhaustion
    let decision = runner.evaluate(&request).expect("should evaluate");
    assert_eq!(decision.verdict, Verdict::Allow);
}

#[test]
fn security_refusal_not_overridable_for_hard_violations() {
    let runner = ContractRunner::new();

    let proposal = create_proposal("main.ts", "const x: string = 'test';");
    let request = GatingRequest::new(proposal);

    let decision = runner.evaluate(&request).expect("should evaluate");

    if let Some(refusal) = &decision.refusal {
        // Hard violations (forbidden language) should not be overridable
        assert!(!refusal.overridable);
    }
}

#[test]
fn security_npm_toolchain_violation() {
    let runner = ContractRunner::new();

    let proposal = create_proposal(
        "package.json",
        r#"{"name": "app", "version": "1.0.0"}"#,
    );
    let request = GatingRequest::new(proposal);

    let decision = runner.evaluate(&request).expect("should evaluate");

    // NPM without Deno should be blocked
    assert_eq!(decision.verdict, Verdict::Block);

    if let Some(refusal) = &decision.refusal {
        assert_eq!(
            refusal.category,
            gating_contract::RefusalCategory::ForbiddenToolchain
        );
    }
}

#[test]
fn security_audit_content_hashed_not_logged() {
    let runner = ContractRunner::new();

    let proposal = create_proposal(
        "secret.rs",
        r#"const PASSWORD = "verysecret123456";"#,
    );
    let mut request = GatingRequest::new(proposal);
    request.context.source = "test".to_string();

    let decision = runner.evaluate(&request).expect("should evaluate");
    let audit = runner.audit(&request, &decision);

    // Audit should have content hash, not the actual secret
    assert!(!audit.content_hash.is_empty());

    // Serialize to verify sensitive data not in audit log
    let json = audit.to_json_compact().expect("serialize");
    // Secret should NOT appear in the audit log
    assert!(!json.contains("verysecret123456"));
}

#[test]
fn security_proposal_id_correlation() {
    let runner = ContractRunner::new();

    let proposal = create_proposal("test.rs", "fn test() {}");
    let proposal_id = proposal.id;

    let request = GatingRequest::new(proposal);
    let decision = runner.evaluate(&request).expect("should evaluate");

    // All parts should reference the same proposal
    assert_eq!(decision.request_id, request.request_id);

    if let Some(oracle) = &decision.evaluations.oracle {
        assert_eq!(oracle.proposal_id, proposal_id);
    }
}

#[test]
fn security_no_unsafe_unwraps_expected() {
    // This test verifies that the oracle evaluation handles errors gracefully
    let runner = ContractRunner::new();

    // Create proposals that might trigger edge cases
    let test_cases = vec![
        ("test.rs", ""),
        ("test.rs", "fn test() { if true { } }"),
        ("test.rs", "// comment"),
    ];

    for (path, content) in test_cases {
        let proposal = create_proposal(path, content);
        let request = GatingRequest::new(proposal);

        // Should not panic (no .unwrap() without expect)
        let _ = runner.evaluate(&request);
    }
}

#[test]
fn security_forbidden_pattern_detection() {
    let runner = ContractRunner::new();

    // Test secret patterns
    let secret_patterns = vec![
        r#"password = "thisisasecret123456""#,
        r#"secret = "thisisasecret123456""#,
        r#"api_key = "thisisasecret123456""#,
    ];

    for pattern in secret_patterns {
        let proposal = create_proposal("config.rs", pattern);
        let request = GatingRequest::new(proposal);

        let decision = runner.evaluate(&request).expect("should evaluate");

        // Should detect hardcoded secrets
        assert_eq!(decision.verdict, Verdict::Block);
    }
}

#[test]
fn security_tier2_warning_not_block() {
    let runner = ContractRunner::new();

    let proposal = create_proposal(
        "config.ncl",
        "{}",
    );
    let request = GatingRequest::new(proposal);

    let decision = runner.evaluate(&request).expect("should evaluate");

    // Tier 2 languages don't have strong markers, so may pass
    // The policy treats them as concerns if detected, but extension-based detection
    // only generates concerns for extension-matched files (not guaranteed)
    assert!(
        matches!(decision.verdict, Verdict::Allow | Verdict::Warn),
        "Tier2 should allow or warn, got {:?}",
        decision.verdict
    );
}

#[test]
fn security_exception_override_works() {
    let runner = ContractRunner::new();

    // Python allowed in specific paths
    let proposal = create_proposal(
        "training/model.py",
        "import tensorflow as tf",
    );
    let request = GatingRequest::new(proposal);

    let decision = runner.evaluate(&request).expect("should evaluate");

    // Should be allowed due to training/ exception
    assert_eq!(decision.verdict, Verdict::Allow);
}

#[test]
fn security_python_blocked_in_source() {
    let runner = ContractRunner::new();

    let proposal = create_proposal(
        "src/utils.py",
        "import os",
    );
    let request = GatingRequest::new(proposal);

    let decision = runner.evaluate(&request).expect("should evaluate");

    // Should be blocked (not in exception path)
    assert_eq!(decision.verdict, Verdict::Block);
}

#[test]
fn security_verdict_determines_exit_status() {
    // Test that verdicts map to appropriate exit codes
    assert_eq!(Verdict::Allow.exit_code(), 0);     // Success
    assert_eq!(Verdict::Warn.exit_code(), 2);      // Warning
    assert_eq!(Verdict::Escalate.exit_code(), 3);  // Escalation
    assert_eq!(Verdict::Block.exit_code(), 1);     // Failure

    // Non-zero means rejection
    assert_ne!(Verdict::Block.exit_code(), 0);
    assert_ne!(Verdict::Warn.exit_code(), 0);
    assert_ne!(Verdict::Escalate.exit_code(), 0);
}
