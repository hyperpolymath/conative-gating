// SPDX-License-Identifier: PMPL-1.0-or-later
//! End-to-End Gating Pipeline Tests
//!
//! Tests the complete gating workflow from proposal input through
//! contract evaluation to audit logging.

use gating_contract::{ContractRunner, GatingRequest, Verdict};
use policy_oracle::{ActionType, Proposal};
use uuid::Uuid;

/// Helper to create test proposals
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
fn e2e_valid_rust_passes_gating() {
    let runner = ContractRunner::new();
    let proposal = create_proposal(
        "src/main.rs",
        r#"
fn main() {
    println!("Hello, Rust!");
}
"#,
    );
    let request = GatingRequest::new(proposal);

    let decision = runner.evaluate(&request).expect("should evaluate");

    assert_eq!(decision.verdict, Verdict::Allow);
    assert!(decision.refusal.is_none());
    assert!(decision.evaluations.oracle.is_some());
    assert!(decision.processing.duration_us > 0);
}

#[test]
fn e2e_forbidden_typescript_blocked_at_gate() {
    let runner = ContractRunner::new();
    let proposal = create_proposal(
        "src/utils.ts",
        r#"
export interface User {
  id: number;
  name: string;
}

export const getUser = (id: number): User => ({
  id,
  name: "Test User"
});
"#,
    );
    let request = GatingRequest::new(proposal);

    let decision = runner.evaluate(&request).expect("should evaluate");

    assert_eq!(decision.verdict, Verdict::Block);
    assert!(decision.refusal.is_some());

    let refusal = decision.refusal.unwrap();
    assert!(!refusal.evidence.is_empty());
    assert!(refusal.remediation.is_some());
    assert!(refusal.remediation.unwrap().contains("ReScript"));
}

#[test]
fn e2e_hardcoded_secret_blocked() {
    let runner = ContractRunner::new();
    let proposal = create_proposal(
        "config.rs",
        r#"const password = "supersecretpassword123456789abcde""#,
    );
    let request = GatingRequest::new(proposal);

    let decision = runner.evaluate(&request).expect("should evaluate");

    // Should detect the hardcoded secret via regex pattern
    assert_eq!(decision.verdict, Verdict::Block);

    let refusal = decision.refusal.unwrap();
    assert_eq!(
        refusal.category,
        gating_contract::RefusalCategory::ForbiddenPattern
    );
}

#[test]
fn e2e_tier2_language_warns() {
    let runner = ContractRunner::new();
    // Nickel doesn't have strong markers, so it's detected by extension only
    let proposal = create_proposal(
        "config.ncl",
        r#"{port = 8080}"#,
    );
    let request = GatingRequest::new(proposal);

    let decision = runner.evaluate(&request).expect("should evaluate");

    // Tier2 languages (Nickel, Racket) generate concerns but allow
    assert!(
        matches!(decision.verdict, Verdict::Allow | Verdict::Warn),
        "Tier2 language should allow or warn, got {:?}",
        decision.verdict
    );
}

#[test]
fn e2e_python_exception_allowed_in_salt() {
    let runner = ContractRunner::new();
    let proposal = create_proposal(
        "salt/states/webserver.py",
        r#"
import os
def install_nginx():
    os.system('apt-get install nginx')
"#,
    );
    let request = GatingRequest::new(proposal);

    let decision = runner.evaluate(&request).expect("should evaluate");

    assert_eq!(decision.verdict, Verdict::Allow);
    assert!(decision.refusal.is_none());
}

#[test]
fn e2e_python_forbidden_outside_salt() {
    let runner = ContractRunner::new();
    let proposal = create_proposal(
        "scripts/analyze.py",
        r#"
import pandas as pd

df = pd.read_csv('data.csv')
print(df.head())
"#,
    );
    let request = GatingRequest::new(proposal);

    let decision = runner.evaluate(&request).expect("should evaluate");

    assert_eq!(decision.verdict, Verdict::Block);
    assert!(decision.refusal.is_some());
}

#[test]
fn e2e_npm_without_deno_rejected() {
    let runner = ContractRunner::new();
    let proposal = create_proposal(
        "package.json",
        r#"
{
  "name": "my-app",
  "version": "1.0.0",
  "dependencies": {
    "react": "^18.0.0"
  }
}
"#,
    );
    let request = GatingRequest::new(proposal);

    let decision = runner.evaluate(&request).expect("should evaluate");

    assert_eq!(decision.verdict, Verdict::Block);

    let refusal = decision.refusal.unwrap();
    assert_eq!(
        refusal.category,
        gating_contract::RefusalCategory::ForbiddenToolchain
    );
}

#[test]
fn e2e_multiple_files_evaluated() {
    let runner = ContractRunner::new();

    let mut proposal = create_proposal("main.rs", "fn main() {}");
    proposal.files_affected.push("config.toml".to_string());

    let request = GatingRequest::new(proposal);
    let decision = runner.evaluate(&request).expect("should evaluate");

    // Compliant because main.rs is Rust
    assert_eq!(decision.verdict, Verdict::Allow);
}

#[test]
fn e2e_audit_log_correlation() {
    let runner = ContractRunner::new();
    let proposal = create_proposal("lib.rs", "pub fn helper() {}");
    let request = GatingRequest::new(proposal);

    let decision = runner.evaluate(&request).expect("should evaluate");
    let audit = runner.audit(&request, &decision);

    // Verify correlation
    assert_eq!(audit.request_id, request.request_id);
    assert_eq!(audit.decision_id, decision.decision_id);
    assert_eq!(audit.verdict, decision.verdict);

    // Verify audit is serializable
    let json = audit.to_json();
    assert!(json.is_ok());
}

#[test]
fn e2e_elixir_proposal_allowed() {
    let runner = ContractRunner::new();
    // Elixir files with proper markers should be allowed
    let proposal = create_proposal(
        "mymodule.ex",
        "defmodule MyApp do\nend",
    );
    let request = GatingRequest::new(proposal);

    let decision = runner.evaluate(&request).expect("should evaluate");

    // Elixir with defmodule marker is tier1
    assert!(
        matches!(decision.verdict, Verdict::Allow | Verdict::Warn),
        "Elixir should be allowed or warned, got {:?}",
        decision.verdict
    );
}

#[test]
fn e2e_go_proposal_blocked() {
    let runner = ContractRunner::new();
    let proposal = create_proposal(
        "main.go",
        r#"package main
func main() {}"#,
    );
    let request = GatingRequest::new(proposal);

    let decision = runner.evaluate(&request).expect("should evaluate");

    assert_eq!(decision.verdict, Verdict::Block);

    let refusal = decision.refusal.unwrap();
    // Refusal should suggest alternative
    assert!(refusal.remediation.is_some());
}

#[test]
fn e2e_java_proposal_blocked() {
    let runner = ContractRunner::new();
    let proposal = create_proposal(
        "Main.java",
        r#"
public class Main {
    public static void main(String[] args) {
        System.out.println("Hello, Java!");
    }
}
"#,
    );
    let request = GatingRequest::new(proposal);

    let decision = runner.evaluate(&request).expect("should evaluate");

    assert_eq!(decision.verdict, Verdict::Block);
}

#[test]
fn e2e_contract_processing_metadata_complete() {
    let runner = ContractRunner::new();
    let proposal = create_proposal("src/lib.rs", "pub struct Test;");
    let request = GatingRequest::new(proposal);

    let decision = runner.evaluate(&request).expect("should evaluate");

    let metadata = &decision.processing;
    assert!(!metadata.contract_version.is_empty());
    assert!(!metadata.policy_name.is_empty());
    assert!(metadata.rules_checked > 0);
    assert!(!metadata.stages_executed.is_empty());
    assert!(metadata.stages_executed.contains(&"oracle".to_string()));
}

#[test]
fn e2e_zig_proposal_allowed() {
    let runner = ContractRunner::new();
    let proposal = create_proposal(
        "src/main.zig",
        r#"
const std = @import("std");

pub fn main() void {
    std.debug.print("Hello, Zig!\n", .{});
}
"#,
    );
    let request = GatingRequest::new(proposal);

    let decision = runner.evaluate(&request).expect("should evaluate");

    assert_eq!(decision.verdict, Verdict::Allow);
}

#[test]
fn e2e_haskell_proposal_allowed() {
    let runner = ContractRunner::new();
    let proposal = create_proposal(
        "Main.hs",
        r#"
module Main where

main :: IO ()
main = putStrLn "Hello, Haskell!"
"#,
    );
    let request = GatingRequest::new(proposal);

    let decision = runner.evaluate(&request).expect("should evaluate");

    assert_eq!(decision.verdict, Verdict::Allow);
}

#[test]
fn e2e_ada_proposal_allowed() {
    let runner = ContractRunner::new();
    let proposal = create_proposal(
        "hello.adb",
        r#"
with Ada.Text_IO;

procedure Hello is
begin
   Ada.Text_IO.Put_Line ("Hello, Ada!");
end Hello;
"#,
    );
    let request = GatingRequest::new(proposal);

    let decision = runner.evaluate(&request).expect("should evaluate");

    assert_eq!(decision.verdict, Verdict::Allow);
}

#[test]
fn e2e_rescript_component_allowed() {
    let runner = ContractRunner::new();
    let proposal = create_proposal(
        "Button.res",
        r#"
@react.component
let make = (~label, ~onClick) => {
  <button onClick={onClick}>
    {React.string(label)}
  </button>
}
"#,
    );
    let request = GatingRequest::new(proposal);

    let decision = runner.evaluate(&request).expect("should evaluate");

    assert_eq!(decision.verdict, Verdict::Allow);
}

#[test]
fn e2e_multiple_violations_in_single_proposal() {
    let runner = ContractRunner::new();
    let proposal = create_proposal(
        "config.ts",
        r#"
// TypeScript config with hardcoded secret
export interface Config {
    apiKey: string;
    dbPassword: string;
}

const config: Config = {
    apiKey: "sk-abcdef1234567890",
    dbPassword: "supersecret123456"
};
"#,
    );
    let request = GatingRequest::new(proposal);

    let decision = runner.evaluate(&request).expect("should evaluate");

    // Should be blocked on the first violation (TypeScript)
    assert_eq!(decision.verdict, Verdict::Block);
    assert!(decision.refusal.is_some());

    let oracle_eval = decision.evaluations.oracle.expect("oracle should evaluate");
    // Should detect multiple rule violations
    assert!(!oracle_eval.violations.is_empty());
}

#[test]
fn e2e_request_context_preserved_in_audit() {
    use gating_contract::RequestContext;

    let runner = ContractRunner::new();
    let proposal = create_proposal("lib.rs", "pub fn test() {}");

    let mut request = GatingRequest::new(proposal);
    request.context = RequestContext {
        source: "test-suite".to_string(),
        session_id: Some("session-abc123".to_string()),
        agent_id: Some("agent-xyz789".to_string()),
        ..Default::default()
    };

    let decision = runner.evaluate(&request).expect("should evaluate");
    let audit = runner.audit(&request, &decision);

    assert_eq!(audit.source, "test-suite");
    assert_eq!(audit.session_id, Some("session-abc123".to_string()));
}
