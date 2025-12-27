// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 Jonathan D.A. Jewell

//! Gating Contract - Testable specification for conative gating decisions
//!
//! This module defines the complete contract for policy enforcement:
//! - **Inputs**: What the gating system receives (`GatingRequest`)
//! - **Outputs**: What the gating system returns (`GatingDecision`)
//! - **Refusal Taxonomy**: Categorization of all refusal types
//! - **Audit Log Format**: Structured logging for compliance and debugging
//!
//! The contract is designed to be:
//! - Testable with deterministic behavior
//! - Auditable with comprehensive logging
//! - Extensible for future SLM integration

use chrono::{DateTime, Utc};
use policy_oracle::{
    ConcernType, OracleError, OracleEvaluation, Policy, PolicyVerdict, Proposal, Severity,
    ViolationType,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

// ============================================================================
// CONTRACT VERSION
// ============================================================================

/// Contract version for compatibility checking
pub const CONTRACT_VERSION: &str = "0.1.0";

/// Contract schema identifier
pub const CONTRACT_SCHEMA: &str = "conative-gating-contract-v1";

// ============================================================================
// INPUTS - What the gating system receives
// ============================================================================

/// Complete gating request - the primary input to the gating system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatingRequest {
    /// Unique request identifier for tracing
    pub request_id: Uuid,

    /// Timestamp when request was created
    pub timestamp: DateTime<Utc>,

    /// The proposal to evaluate
    pub proposal: Proposal,

    /// Request context and metadata
    pub context: RequestContext,

    /// Optional policy override (uses default if None)
    pub policy_override: Option<Policy>,
}

/// Context surrounding the gating request
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RequestContext {
    /// Source of the request (e.g., "claude-code", "github-action", "api")
    pub source: String,

    /// Session or conversation identifier
    pub session_id: Option<String>,

    /// User or agent identifier (anonymized)
    pub agent_id: Option<String>,

    /// Repository or project context
    pub repository: Option<RepositoryContext>,

    /// Previous decisions in this session (for pattern detection)
    pub session_history: Vec<Uuid>,

    /// Custom metadata key-value pairs
    pub metadata: HashMap<String, String>,
}

/// Repository context for evaluating proposals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryContext {
    /// Repository name or path
    pub name: String,

    /// Default branch
    pub default_branch: Option<String>,

    /// Policy configuration file path (if any)
    pub policy_file: Option<String>,

    /// Whether this is a new repository (no history)
    pub is_new: bool,
}

impl GatingRequest {
    /// Create a new gating request with minimal required fields
    pub fn new(proposal: Proposal) -> Self {
        Self {
            request_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            proposal,
            context: RequestContext::default(),
            policy_override: None,
        }
    }

    /// Builder: set request context
    pub fn with_context(mut self, context: RequestContext) -> Self {
        self.context = context;
        self
    }

    /// Builder: set policy override
    pub fn with_policy(mut self, policy: Policy) -> Self {
        self.policy_override = Some(policy);
        self
    }
}

// ============================================================================
// OUTPUTS - What the gating system returns
// ============================================================================

/// Complete gating decision - the primary output of the gating system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatingDecision {
    /// Original request ID for correlation
    pub request_id: Uuid,

    /// Unique decision identifier
    pub decision_id: Uuid,

    /// Timestamp when decision was made
    pub timestamp: DateTime<Utc>,

    /// The final verdict
    pub verdict: Verdict,

    /// Refusal details (if verdict is not Allow)
    pub refusal: Option<Refusal>,

    /// Evaluation details from each stage
    pub evaluations: EvaluationChain,

    /// Processing metadata
    pub processing: ProcessingMetadata,
}

/// Final verdict of the gating decision
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Verdict {
    /// Proposal is allowed to proceed
    Allow,

    /// Proposal triggers a warning but is allowed
    Warn,

    /// Proposal requires human escalation
    Escalate,

    /// Proposal is blocked
    Block,
}

impl Verdict {
    /// Convert to exit code for CLI usage
    pub fn exit_code(&self) -> i32 {
        match self {
            Verdict::Allow => 0,
            Verdict::Warn => 2,
            Verdict::Escalate => 3,
            Verdict::Block => 1,
        }
    }

    /// Whether the proposal can proceed
    pub fn is_allowed(&self) -> bool {
        matches!(self, Verdict::Allow | Verdict::Warn)
    }
}

/// Chain of evaluations from all stages
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EvaluationChain {
    /// Oracle (deterministic) evaluation result
    pub oracle: Option<OracleEvaluation>,

    /// SLM (neural) evaluation result (when implemented)
    pub slm: Option<SlmEvaluationResult>,

    /// Arbiter consensus result (when implemented)
    pub arbiter: Option<ArbiterResult>,
}

/// Placeholder for SLM evaluation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlmEvaluationResult {
    pub spirit_score: f64,
    pub confidence: f64,
    pub reasoning: String,
    pub should_block: bool,
}

/// Placeholder for arbiter consensus result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbiterResult {
    pub consensus_reached: bool,
    pub oracle_vote: Verdict,
    pub slm_vote: Verdict,
    pub final_verdict: Verdict,
    pub slm_weight: f64,
}

/// Processing metadata for observability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingMetadata {
    /// Processing duration in microseconds
    pub duration_us: u64,

    /// Contract version used
    pub contract_version: String,

    /// Policy name used
    pub policy_name: String,

    /// Number of rules checked
    pub rules_checked: usize,

    /// Stages that were executed
    pub stages_executed: Vec<String>,
}

impl Default for ProcessingMetadata {
    fn default() -> Self {
        Self {
            duration_us: 0,
            contract_version: CONTRACT_VERSION.to_string(),
            policy_name: String::new(),
            rules_checked: 0,
            stages_executed: Vec::new(),
        }
    }
}

// ============================================================================
// REFUSAL TAXONOMY - Categorization of all refusal types
// ============================================================================

/// Complete refusal information when a proposal is not allowed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Refusal {
    /// Primary refusal category
    pub category: RefusalCategory,

    /// Specific refusal code for programmatic handling
    pub code: RefusalCode,

    /// Human-readable message
    pub message: String,

    /// Suggested remediation (if applicable)
    pub remediation: Option<String>,

    /// Evidence supporting the refusal
    pub evidence: Vec<Evidence>,

    /// Whether this refusal can be overridden
    pub overridable: bool,

    /// Required authorization level for override
    pub override_level: Option<AuthorizationLevel>,
}

/// Top-level refusal categories
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RefusalCategory {
    // === Hard Policy Violations (Oracle) ===
    /// Forbidden programming language detected
    ForbiddenLanguage,

    /// Forbidden toolchain or dependency management
    ForbiddenToolchain,

    /// Security pattern violation (secrets, unsafe patterns)
    SecurityViolation,

    /// Forbidden code pattern detected
    ForbiddenPattern,

    // === Spirit Violations (SLM) ===
    /// Excessive verbosity or documentation bloat
    VerbositySmell,

    /// Unusual code structure or anti-patterns
    StructuralAnomaly,

    /// Intent violation (technically compliant but spirit-violating)
    IntentViolation,

    /// Adversarial input detection
    AdversarialInput,

    // === System Refusals ===
    /// Request validation failed
    InvalidRequest,

    /// Rate limiting or quota exceeded
    RateLimited,

    /// System error during processing
    SystemError,
}

impl RefusalCategory {
    /// Human-readable display name
    pub fn display_name(&self) -> &'static str {
        match self {
            RefusalCategory::ForbiddenLanguage => "Forbidden Language",
            RefusalCategory::ForbiddenToolchain => "Forbidden Toolchain",
            RefusalCategory::SecurityViolation => "Security Violation",
            RefusalCategory::ForbiddenPattern => "Forbidden Pattern",
            RefusalCategory::VerbositySmell => "Verbosity Smell",
            RefusalCategory::StructuralAnomaly => "Structural Anomaly",
            RefusalCategory::IntentViolation => "Intent Violation",
            RefusalCategory::AdversarialInput => "Adversarial Input",
            RefusalCategory::InvalidRequest => "Invalid Request",
            RefusalCategory::RateLimited => "Rate Limited",
            RefusalCategory::SystemError => "System Error",
        }
    }

    /// Whether this is a hard (deterministic) or soft (neural) refusal
    pub fn is_hard(&self) -> bool {
        matches!(
            self,
            RefusalCategory::ForbiddenLanguage
                | RefusalCategory::ForbiddenToolchain
                | RefusalCategory::SecurityViolation
                | RefusalCategory::ForbiddenPattern
                | RefusalCategory::InvalidRequest
                | RefusalCategory::SystemError
        )
    }

    /// Severity level of this category
    pub fn severity(&self) -> Severity {
        match self {
            RefusalCategory::SecurityViolation => Severity::Critical,
            RefusalCategory::ForbiddenLanguage => Severity::Critical,
            RefusalCategory::ForbiddenToolchain => Severity::High,
            RefusalCategory::ForbiddenPattern => Severity::High,
            RefusalCategory::AdversarialInput => Severity::Critical,
            RefusalCategory::IntentViolation => Severity::Medium,
            RefusalCategory::VerbositySmell => Severity::Low,
            RefusalCategory::StructuralAnomaly => Severity::Medium,
            RefusalCategory::InvalidRequest => Severity::Medium,
            RefusalCategory::RateLimited => Severity::Low,
            RefusalCategory::SystemError => Severity::High,
        }
    }
}

/// Specific refusal codes for programmatic handling
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RefusalCode {
    // Language codes (1xx)
    Lang100TypeScript,
    Lang101Python,
    Lang102Go,
    Lang103Java,
    Lang104Kotlin,
    Lang105Swift,
    Lang199OtherForbidden,

    // Toolchain codes (2xx)
    Tool200NpmWithoutDeno,
    Tool201YarnWithoutDeno,
    Tool202NodeModules,
    Tool203PackageJson,
    Tool299OtherToolchain,

    // Security codes (3xx)
    Sec300HardcodedSecret,
    Sec301InsecureHash,
    Sec302HttpUrl,
    Sec303CommandInjection,
    Sec304SqlInjection,
    Sec399OtherSecurity,

    // Pattern codes (4xx)
    Pat400ForbiddenImport,
    Pat401UnsafeBlock,
    Pat499OtherPattern,

    // Spirit codes (5xx)
    Spirit500Verbosity,
    Spirit501OverDocumentation,
    Spirit502RedundantComments,
    Spirit503BoilerplateCode,
    Spirit504MetaCommentary,
    Spirit505IntentMismatch,
    Spirit599OtherSpirit,

    // System codes (9xx)
    Sys900InvalidRequest,
    Sys901RateLimited,
    Sys902InternalError,
    Sys999Unknown,
}

impl RefusalCode {
    /// Numeric code for logging and metrics
    pub fn numeric(&self) -> u16 {
        match self {
            RefusalCode::Lang100TypeScript => 100,
            RefusalCode::Lang101Python => 101,
            RefusalCode::Lang102Go => 102,
            RefusalCode::Lang103Java => 103,
            RefusalCode::Lang104Kotlin => 104,
            RefusalCode::Lang105Swift => 105,
            RefusalCode::Lang199OtherForbidden => 199,
            RefusalCode::Tool200NpmWithoutDeno => 200,
            RefusalCode::Tool201YarnWithoutDeno => 201,
            RefusalCode::Tool202NodeModules => 202,
            RefusalCode::Tool203PackageJson => 203,
            RefusalCode::Tool299OtherToolchain => 299,
            RefusalCode::Sec300HardcodedSecret => 300,
            RefusalCode::Sec301InsecureHash => 301,
            RefusalCode::Sec302HttpUrl => 302,
            RefusalCode::Sec303CommandInjection => 303,
            RefusalCode::Sec304SqlInjection => 304,
            RefusalCode::Sec399OtherSecurity => 399,
            RefusalCode::Pat400ForbiddenImport => 400,
            RefusalCode::Pat401UnsafeBlock => 401,
            RefusalCode::Pat499OtherPattern => 499,
            RefusalCode::Spirit500Verbosity => 500,
            RefusalCode::Spirit501OverDocumentation => 501,
            RefusalCode::Spirit502RedundantComments => 502,
            RefusalCode::Spirit503BoilerplateCode => 503,
            RefusalCode::Spirit504MetaCommentary => 504,
            RefusalCode::Spirit505IntentMismatch => 505,
            RefusalCode::Spirit599OtherSpirit => 599,
            RefusalCode::Sys900InvalidRequest => 900,
            RefusalCode::Sys901RateLimited => 901,
            RefusalCode::Sys902InternalError => 902,
            RefusalCode::Sys999Unknown => 999,
        }
    }
}

/// Evidence supporting a refusal decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// Type of evidence
    pub evidence_type: EvidenceType,

    /// File path (if applicable)
    pub file: Option<String>,

    /// Line number (if applicable)
    pub line: Option<u32>,

    /// Matched pattern or content
    pub match_content: String,

    /// Explanation of why this is evidence
    pub explanation: String,
}

/// Types of evidence that can support a refusal
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EvidenceType {
    FileExtension,
    ContentMarker,
    RegexMatch,
    SyntaxPattern,
    SlmAnalysis,
    HistoricalPattern,
}

/// Authorization levels for override
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuthorizationLevel {
    /// Can be overridden by any user
    User = 1,
    /// Requires maintainer authorization
    Maintainer = 2,
    /// Requires admin authorization
    Admin = 3,
    /// Cannot be overridden
    None = 100,
}

// ============================================================================
// AUDIT LOG FORMAT - Structured logging for compliance
// ============================================================================

/// Audit log entry for every gating decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Entry schema version
    pub schema: String,

    /// Unique audit entry ID
    pub audit_id: Uuid,

    /// Request ID for correlation
    pub request_id: Uuid,

    /// Decision ID for correlation
    pub decision_id: Uuid,

    /// Timestamp of the audit entry
    pub timestamp: DateTime<Utc>,

    /// Final verdict
    pub verdict: Verdict,

    /// Refusal code (if any)
    pub refusal_code: Option<u16>,

    /// Refusal category (if any)
    pub refusal_category: Option<RefusalCategory>,

    /// Source of the request
    pub source: String,

    /// Repository context (anonymized if needed)
    pub repository: Option<String>,

    /// Session ID for pattern detection
    pub session_id: Option<String>,

    /// Rules that were checked
    pub rules_checked: Vec<String>,

    /// Rules that triggered
    pub rules_triggered: Vec<String>,

    /// Processing duration in microseconds
    pub duration_us: u64,

    /// Stages executed
    pub stages: Vec<String>,

    /// Contract version
    pub contract_version: String,

    /// Hash of the proposal content (for verification without storing content)
    pub content_hash: String,
}

impl AuditEntry {
    /// Create an audit entry from a request and decision
    pub fn from_decision(request: &GatingRequest, decision: &GatingDecision) -> Self {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        request.proposal.content.hash(&mut hasher);
        let content_hash = format!("{:016x}", hasher.finish());

        let rules_triggered: Vec<String> = decision
            .evaluations
            .oracle
            .as_ref()
            .map(|o| {
                o.violations
                    .iter()
                    .map(|v| v.rule.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Self {
            schema: CONTRACT_SCHEMA.to_string(),
            audit_id: Uuid::new_v4(),
            request_id: request.request_id,
            decision_id: decision.decision_id,
            timestamp: Utc::now(),
            verdict: decision.verdict,
            refusal_code: decision.refusal.as_ref().map(|r| r.code.numeric()),
            refusal_category: decision.refusal.as_ref().map(|r| r.category),
            source: request.context.source.clone(),
            repository: request.context.repository.as_ref().map(|r| r.name.clone()),
            session_id: request.context.session_id.clone(),
            rules_checked: decision
                .evaluations
                .oracle
                .as_ref()
                .map(|o| o.rules_checked.clone())
                .unwrap_or_default(),
            rules_triggered,
            duration_us: decision.processing.duration_us,
            stages: decision.processing.stages_executed.clone(),
            contract_version: CONTRACT_VERSION.to_string(),
            content_hash,
        }
    }

    /// Serialize to JSON for logging
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Serialize to compact JSON for high-volume logging
    pub fn to_json_compact(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Serialize to pretty JSON for debugging
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

// ============================================================================
// CONTRACT ERRORS
// ============================================================================

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Oracle error: {0}")]
    OracleError(#[from] OracleError),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

// ============================================================================
// CONTRACT EVALUATOR - The minimal runner
// ============================================================================

/// Contract evaluator - processes gating requests according to the contract
pub struct ContractRunner {
    oracle: policy_oracle::Oracle,
    policy: Policy,
}

impl ContractRunner {
    /// Create a new contract runner with RSR defaults
    pub fn new() -> Self {
        let policy = Policy::rsr_default();
        Self {
            oracle: policy_oracle::Oracle::new(policy.clone()),
            policy,
        }
    }

    /// Create a new contract runner with a custom policy
    pub fn with_policy(policy: Policy) -> Self {
        Self {
            oracle: policy_oracle::Oracle::new(policy.clone()),
            policy,
        }
    }

    /// Evaluate a gating request and return a decision
    pub fn evaluate(&self, request: &GatingRequest) -> Result<GatingDecision, ContractError> {
        let start = std::time::Instant::now();
        let mut stages_executed = Vec::new();

        // Stage 1: Oracle evaluation
        stages_executed.push("oracle".to_string());
        let oracle_eval = self.oracle.check_proposal(&request.proposal)?;

        // Determine verdict based on oracle result
        let (verdict, refusal) = self.process_oracle_result(&oracle_eval);

        let duration = start.elapsed();

        Ok(GatingDecision {
            request_id: request.request_id,
            decision_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            verdict,
            refusal,
            evaluations: EvaluationChain {
                oracle: Some(oracle_eval.clone()),
                slm: None,     // Phase 2: Requires llama.cpp integration
                arbiter: None, // Phase 4: Elixir GenServer via Rustler NIF
            },
            processing: ProcessingMetadata {
                duration_us: duration.as_micros() as u64,
                contract_version: CONTRACT_VERSION.to_string(),
                policy_name: self.policy.name.clone(),
                rules_checked: oracle_eval.rules_checked.len(),
                stages_executed,
            },
        })
    }

    /// Process oracle evaluation into verdict and refusal
    fn process_oracle_result(
        &self,
        eval: &OracleEvaluation,
    ) -> (Verdict, Option<Refusal>) {
        match &eval.verdict {
            PolicyVerdict::Compliant => (Verdict::Allow, None),

            PolicyVerdict::SoftConcern(concern) => {
                let (category, code, message) = self.map_concern(concern);
                (
                    Verdict::Warn,
                    Some(Refusal {
                        category,
                        code,
                        message,
                        remediation: Some("Consider refactoring to address the concern".to_string()),
                        evidence: Vec::new(),
                        overridable: true,
                        override_level: Some(AuthorizationLevel::User),
                    }),
                )
            }

            PolicyVerdict::HardViolation(violation) => {
                let (category, code, message, evidence, remediation) =
                    self.map_violation(violation);
                (
                    Verdict::Block,
                    Some(Refusal {
                        category,
                        code,
                        message,
                        remediation,
                        evidence,
                        overridable: false,
                        override_level: Some(AuthorizationLevel::None),
                    }),
                )
            }
        }
    }

    fn map_concern(&self, concern: &ConcernType) -> (RefusalCategory, RefusalCode, String) {
        match concern {
            ConcernType::VerbositySmell => (
                RefusalCategory::VerbositySmell,
                RefusalCode::Spirit500Verbosity,
                "Excessive verbosity detected".to_string(),
            ),
            ConcernType::PatternDeviation => (
                RefusalCategory::StructuralAnomaly,
                RefusalCode::Spirit505IntentMismatch,
                "Unusual pattern deviation detected".to_string(),
            ),
            ConcernType::UnusualStructure => (
                RefusalCategory::StructuralAnomaly,
                RefusalCode::Spirit505IntentMismatch,
                "Unusual code structure detected".to_string(),
            ),
            ConcernType::Tier2Language { language } => (
                RefusalCategory::ForbiddenLanguage,
                RefusalCode::Lang199OtherForbidden,
                format!("Tier 2 language '{}' - consider Tier 1 alternative", language),
            ),
        }
    }

    fn map_violation(
        &self,
        violation: &ViolationType,
    ) -> (
        RefusalCategory,
        RefusalCode,
        String,
        Vec<Evidence>,
        Option<String>,
    ) {
        match violation {
            ViolationType::ForbiddenLanguage {
                language,
                file,
                context,
            } => {
                let code = match language.to_lowercase().as_str() {
                    "typescript" => RefusalCode::Lang100TypeScript,
                    "python" => RefusalCode::Lang101Python,
                    "go" => RefusalCode::Lang102Go,
                    "java" => RefusalCode::Lang103Java,
                    "kotlin" => RefusalCode::Lang104Kotlin,
                    "swift" => RefusalCode::Lang105Swift,
                    _ => RefusalCode::Lang199OtherForbidden,
                };

                let remediation = match language.to_lowercase().as_str() {
                    "typescript" => Some("Use ReScript instead of TypeScript".to_string()),
                    "python" => Some(
                        "Python is only allowed in salt/ for SaltStack configs".to_string(),
                    ),
                    "go" => Some("Use Rust instead of Go".to_string()),
                    "java" => Some("Use Rust/Tauri/Dioxus instead of Java".to_string()),
                    _ => None,
                };

                (
                    RefusalCategory::ForbiddenLanguage,
                    code,
                    format!("Forbidden language '{}' detected", language),
                    vec![Evidence {
                        evidence_type: EvidenceType::ContentMarker,
                        file: Some(file.clone()),
                        line: None,
                        match_content: context.clone(),
                        explanation: format!("{} code detected", language),
                    }],
                    remediation,
                )
            }

            ViolationType::ForbiddenToolchain { tool, missing } => (
                RefusalCategory::ForbiddenToolchain,
                RefusalCode::Tool200NpmWithoutDeno,
                format!("Toolchain violation: {} requires {}", tool, missing),
                vec![Evidence {
                    evidence_type: EvidenceType::FileExtension,
                    file: None,
                    line: None,
                    match_content: tool.clone(),
                    explanation: format!("{} detected without {}", tool, missing),
                }],
                Some(format!("Add {} to use {}", missing, tool)),
            ),

            ViolationType::SecurityViolation { description } => (
                RefusalCategory::SecurityViolation,
                RefusalCode::Sec300HardcodedSecret,
                format!("Security violation: {}", description),
                Vec::new(),
                Some("Remove hardcoded secrets and use environment variables".to_string()),
            ),

            ViolationType::ForbiddenPattern { pattern, file } => (
                RefusalCategory::ForbiddenPattern,
                RefusalCode::Pat499OtherPattern,
                format!("Forbidden pattern '{}' detected", pattern),
                vec![Evidence {
                    evidence_type: EvidenceType::RegexMatch,
                    file: Some(file.clone()),
                    line: None,
                    match_content: pattern.clone(),
                    explanation: "Pattern matched forbidden regex".to_string(),
                }],
                None,
            ),
        }
    }

    /// Create an audit entry for a decision
    pub fn audit(&self, request: &GatingRequest, decision: &GatingDecision) -> AuditEntry {
        AuditEntry::from_decision(request, decision)
    }
}

impl Default for ContractRunner {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TEST HARNESS
// ============================================================================

/// Test case for contract validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    /// Test case name
    pub name: String,

    /// Description of what this tests
    pub description: String,

    /// Input request
    pub request: GatingRequest,

    /// Expected verdict
    pub expected_verdict: Verdict,

    /// Expected refusal category (if any)
    pub expected_category: Option<RefusalCategory>,

    /// Expected refusal code (if any)
    pub expected_code: Option<RefusalCode>,
}

/// Test result from running a test case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    /// Test case name
    pub name: String,

    /// Whether the test passed
    pub passed: bool,

    /// Actual verdict received
    pub actual_verdict: Verdict,

    /// Expected verdict
    pub expected_verdict: Verdict,

    /// Actual refusal category (if any)
    pub actual_category: Option<RefusalCategory>,

    /// Error message if test failed
    pub error: Option<String>,

    /// Duration of test execution in microseconds
    pub duration_us: u64,
}

/// Test harness for running contract tests
pub struct TestHarness {
    runner: ContractRunner,
    results: Vec<TestResult>,
}

impl TestHarness {
    pub fn new() -> Self {
        Self {
            runner: ContractRunner::new(),
            results: Vec::new(),
        }
    }

    pub fn with_runner(runner: ContractRunner) -> Self {
        Self {
            runner,
            results: Vec::new(),
        }
    }

    /// Run a single test case
    pub fn run_test(&mut self, test: &TestCase) -> TestResult {
        let start = std::time::Instant::now();

        let result = match self.runner.evaluate(&test.request) {
            Ok(decision) => {
                let verdict_matches = decision.verdict == test.expected_verdict;
                let category_matches = match (&test.expected_category, &decision.refusal) {
                    (Some(expected), Some(refusal)) => refusal.category == *expected,
                    (None, None) => true,
                    _ => false,
                };

                let passed = verdict_matches && category_matches;
                let error = if !passed {
                    Some(format!(
                        "Expected {:?} with {:?}, got {:?} with {:?}",
                        test.expected_verdict,
                        test.expected_category,
                        decision.verdict,
                        decision.refusal.as_ref().map(|r| &r.category)
                    ))
                } else {
                    None
                };

                TestResult {
                    name: test.name.clone(),
                    passed,
                    actual_verdict: decision.verdict,
                    expected_verdict: test.expected_verdict,
                    actual_category: decision.refusal.map(|r| r.category),
                    error,
                    duration_us: start.elapsed().as_micros() as u64,
                }
            }
            Err(e) => TestResult {
                name: test.name.clone(),
                passed: false,
                actual_verdict: Verdict::Block,
                expected_verdict: test.expected_verdict,
                actual_category: None,
                error: Some(e.to_string()),
                duration_us: start.elapsed().as_micros() as u64,
            },
        };

        self.results.push(result.clone());
        result
    }

    /// Run all test cases
    pub fn run_all(&mut self, tests: &[TestCase]) -> Vec<TestResult> {
        tests.iter().map(|t| self.run_test(t)).collect()
    }

    /// Get summary of test results
    pub fn summary(&self) -> TestSummary {
        let passed = self.results.iter().filter(|r| r.passed).count();
        let failed = self.results.len() - passed;
        let total_duration_us: u64 = self.results.iter().map(|r| r.duration_us).sum();

        TestSummary {
            total: self.results.len(),
            passed,
            failed,
            total_duration_us,
            results: self.results.clone(),
        }
    }

    /// Clear results
    pub fn clear(&mut self) {
        self.results.clear();
    }
}

impl Default for TestHarness {
    fn default() -> Self {
        Self::new()
    }
}

/// Summary of test execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSummary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub total_duration_us: u64,
    pub results: Vec<TestResult>,
}

impl TestSummary {
    /// Check if all tests passed
    pub fn all_passed(&self) -> bool {
        self.failed == 0
    }

    /// Get failed test names
    pub fn failed_tests(&self) -> Vec<&str> {
        self.results
            .iter()
            .filter(|r| !r.passed)
            .map(|r| r.name.as_str())
            .collect()
    }
}

// ============================================================================
// REGRESSION HARNESS
// ============================================================================

/// Baseline result for regression testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineResult {
    /// Test case name
    pub name: String,

    /// Expected verdict from baseline
    pub verdict: Verdict,

    /// Expected category from baseline
    pub category: Option<RefusalCategory>,

    /// Expected refusal code from baseline
    pub code: Option<u16>,

    /// Timestamp when baseline was recorded
    pub recorded_at: DateTime<Utc>,

    /// Contract version when recorded
    pub contract_version: String,
}

/// Complete regression baseline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionBaseline {
    /// Baseline schema version
    pub schema: String,

    /// When the baseline was created
    pub created_at: DateTime<Utc>,

    /// Contract version used
    pub contract_version: String,

    /// Git commit hash (if available)
    pub git_commit: Option<String>,

    /// Individual test baselines
    pub results: Vec<BaselineResult>,

    /// Metadata
    pub metadata: HashMap<String, String>,
}

impl RegressionBaseline {
    /// Create a new baseline from test results
    pub fn from_summary(summary: &TestSummary, git_commit: Option<String>) -> Self {
        let results = summary
            .results
            .iter()
            .map(|r| BaselineResult {
                name: r.name.clone(),
                verdict: r.actual_verdict,
                category: r.actual_category,
                code: None,
                recorded_at: Utc::now(),
                contract_version: CONTRACT_VERSION.to_string(),
            })
            .collect();

        Self {
            schema: "regression-baseline-v1".to_string(),
            created_at: Utc::now(),
            contract_version: CONTRACT_VERSION.to_string(),
            git_commit,
            results,
            metadata: HashMap::new(),
        }
    }

    /// Serialize to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Regression detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionReport {
    /// When the comparison was run
    pub timestamp: DateTime<Utc>,

    /// Baseline used for comparison
    pub baseline_commit: Option<String>,

    /// Current contract version
    pub current_version: String,

    /// Total tests compared
    pub total_compared: usize,

    /// Tests that regressed (were passing, now failing)
    pub regressions: Vec<Regression>,

    /// Tests that improved (were failing, now passing)
    pub improvements: Vec<Improvement>,

    /// Tests that changed behavior (different verdict)
    pub behavior_changes: Vec<BehaviorChange>,

    /// Tests with stable behavior
    pub stable_count: usize,

    /// New tests not in baseline
    pub new_tests: Vec<String>,

    /// Tests in baseline but not in current run
    pub removed_tests: Vec<String>,
}

impl RegressionReport {
    /// Check if there are any regressions
    pub fn has_regressions(&self) -> bool {
        !self.regressions.is_empty()
    }

    /// Check if there are any behavior changes
    pub fn has_changes(&self) -> bool {
        !self.regressions.is_empty() || !self.behavior_changes.is_empty()
    }

    /// Get summary text
    pub fn summary_text(&self) -> String {
        format!(
            "Compared {} tests: {} stable, {} regressions, {} improvements, {} behavior changes, {} new, {} removed",
            self.total_compared,
            self.stable_count,
            self.regressions.len(),
            self.improvements.len(),
            self.behavior_changes.len(),
            self.new_tests.len(),
            self.removed_tests.len()
        )
    }
}

/// A test that regressed (was passing, now failing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Regression {
    pub test_name: String,
    pub baseline_verdict: Verdict,
    pub current_verdict: Verdict,
    pub baseline_passed: bool,
    pub current_passed: bool,
    pub error_message: Option<String>,
}

/// A test that improved (was failing, now passing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Improvement {
    pub test_name: String,
    pub baseline_verdict: Verdict,
    pub current_verdict: Verdict,
}

/// A test with changed behavior (different verdict, may or may not be regression)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorChange {
    pub test_name: String,
    pub baseline_verdict: Verdict,
    pub current_verdict: Verdict,
    pub baseline_category: Option<RefusalCategory>,
    pub current_category: Option<RefusalCategory>,
}

/// Regression test harness
pub struct RegressionHarness {
    baseline: Option<RegressionBaseline>,
    current_results: Vec<TestResult>,
}

impl RegressionHarness {
    /// Create a new regression harness
    pub fn new() -> Self {
        Self {
            baseline: None,
            current_results: Vec::new(),
        }
    }

    /// Load baseline from JSON
    pub fn with_baseline(mut self, baseline: RegressionBaseline) -> Self {
        self.baseline = Some(baseline);
        self
    }

    /// Load baseline from file
    pub fn load_baseline(&mut self, path: &std::path::Path) -> Result<(), std::io::Error> {
        let content = std::fs::read_to_string(path)?;
        let baseline = RegressionBaseline::from_json(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        self.baseline = Some(baseline);
        Ok(())
    }

    /// Save current results as baseline
    pub fn save_baseline(
        &self,
        path: &std::path::Path,
        git_commit: Option<String>,
    ) -> Result<(), std::io::Error> {
        let summary = TestSummary {
            total: self.current_results.len(),
            passed: self.current_results.iter().filter(|r| r.passed).count(),
            failed: self.current_results.iter().filter(|r| !r.passed).count(),
            total_duration_us: self.current_results.iter().map(|r| r.duration_us).sum(),
            results: self.current_results.clone(),
        };
        let baseline = RegressionBaseline::from_summary(&summary, git_commit);
        let json = baseline.to_json()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, json)
    }

    /// Add test results for comparison
    pub fn add_results(&mut self, results: Vec<TestResult>) {
        self.current_results.extend(results);
    }

    /// Compare current results against baseline and generate report
    pub fn compare(&self) -> RegressionReport {
        let baseline = match &self.baseline {
            Some(b) => b,
            None => {
                return RegressionReport {
                    timestamp: Utc::now(),
                    baseline_commit: None,
                    current_version: CONTRACT_VERSION.to_string(),
                    total_compared: 0,
                    regressions: Vec::new(),
                    improvements: Vec::new(),
                    behavior_changes: Vec::new(),
                    stable_count: 0,
                    new_tests: self.current_results.iter().map(|r| r.name.clone()).collect(),
                    removed_tests: Vec::new(),
                };
            }
        };

        let baseline_map: HashMap<&str, &BaselineResult> = baseline
            .results
            .iter()
            .map(|r| (r.name.as_str(), r))
            .collect();

        let current_map: HashMap<&str, &TestResult> = self
            .current_results
            .iter()
            .map(|r| (r.name.as_str(), r))
            .collect();

        let mut regressions = Vec::new();
        let mut improvements = Vec::new();
        let mut behavior_changes = Vec::new();
        let mut stable_count = 0;
        let mut new_tests = Vec::new();
        let mut removed_tests = Vec::new();

        // Check current results against baseline
        for current in &self.current_results {
            if let Some(baseline_result) = baseline_map.get(current.name.as_str()) {
                let baseline_passed = baseline_result.verdict == current.expected_verdict;
                let current_passed = current.passed;

                if baseline_passed && !current_passed {
                    // Regression: was passing, now failing
                    regressions.push(Regression {
                        test_name: current.name.clone(),
                        baseline_verdict: baseline_result.verdict,
                        current_verdict: current.actual_verdict,
                        baseline_passed,
                        current_passed,
                        error_message: current.error.clone(),
                    });
                } else if !baseline_passed && current_passed {
                    // Improvement: was failing, now passing
                    improvements.push(Improvement {
                        test_name: current.name.clone(),
                        baseline_verdict: baseline_result.verdict,
                        current_verdict: current.actual_verdict,
                    });
                } else if baseline_result.verdict != current.actual_verdict {
                    // Behavior change: different verdict
                    behavior_changes.push(BehaviorChange {
                        test_name: current.name.clone(),
                        baseline_verdict: baseline_result.verdict,
                        current_verdict: current.actual_verdict,
                        baseline_category: baseline_result.category,
                        current_category: current.actual_category,
                    });
                } else {
                    stable_count += 1;
                }
            } else {
                new_tests.push(current.name.clone());
            }
        }

        // Check for removed tests
        for baseline_result in &baseline.results {
            if !current_map.contains_key(baseline_result.name.as_str()) {
                removed_tests.push(baseline_result.name.clone());
            }
        }

        RegressionReport {
            timestamp: Utc::now(),
            baseline_commit: baseline.git_commit.clone(),
            current_version: CONTRACT_VERSION.to_string(),
            total_compared: self.current_results.len(),
            regressions,
            improvements,
            behavior_changes,
            stable_count,
            new_tests,
            removed_tests,
        }
    }
}

impl Default for RegressionHarness {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// RED-TEAM TEST METADATA
// ============================================================================

/// Red-team test category
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RedTeamCategory {
    /// Attempts to bypass detection via documentation/comments
    DocumentationBypass,
    /// Attempts to split or obfuscate markers
    MarkerObfuscation,
    /// Attempts to use encoding to hide content
    EncodedContent,
    /// Boundary condition tests (empty, whitespace, unicode)
    BoundaryCondition,
    /// Polyglot or injection attacks
    ContentInjection,
    /// Secret hiding techniques
    SecretEvasion,
    /// False positive tests (should NOT trigger)
    FalsePositiveCheck,
    /// Custom/other category
    Custom(String),
}

impl RedTeamCategory {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "documentation_bypass" | "doc_bypass" | "comment_bypass" => {
                RedTeamCategory::DocumentationBypass
            }
            "marker_split" | "marker_obfuscation" | "case_evasion" | "extension_masking" => {
                RedTeamCategory::MarkerObfuscation
            }
            "encoded_secrets" | "encoding" => RedTeamCategory::EncodedContent,
            "edge_case" | "boundary" | "unicode_evasion" => RedTeamCategory::BoundaryCondition,
            "polyglot" | "injection" => RedTeamCategory::ContentInjection,
            "secret_hiding" | "secret_splitting" => RedTeamCategory::SecretEvasion,
            "false_positive_avoidance" | "false_positive" => RedTeamCategory::FalsePositiveCheck,
            other => RedTeamCategory::Custom(other.to_string()),
        }
    }
}

/// Extended test case with red-team metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedTeamTestCase {
    /// Base test case
    #[serde(flatten)]
    pub base: TestCase,

    /// Red-team category
    pub redteam_category: RedTeamCategory,

    /// Attack vector description
    pub attack_vector: String,

    /// Severity if this bypass works
    pub bypass_severity: Severity,

    /// Whether this is an expected bypass (known limitation)
    pub known_limitation: bool,
}

/// Red-team test summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedTeamSummary {
    /// Total red-team tests
    pub total: usize,

    /// Tests where oracle correctly blocked attack
    pub blocked: usize,

    /// Tests where attack bypassed oracle
    pub bypassed: usize,

    /// Tests where oracle had false positive
    pub false_positives: usize,

    /// Known limitations (expected bypasses)
    pub known_limitations: usize,

    /// Breakdown by category
    pub by_category: HashMap<String, CategoryStats>,

    /// Bypass rate (bypassed / total)
    pub bypass_rate: f64,

    /// False positive rate
    pub false_positive_rate: f64,
}

/// Statistics for a red-team category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryStats {
    pub total: usize,
    pub blocked: usize,
    pub bypassed: usize,
    pub false_positives: usize,
}

impl RedTeamSummary {
    /// Check if any bypasses occurred (excluding known limitations)
    pub fn has_unexpected_bypasses(&self) -> bool {
        self.bypassed > self.known_limitations
    }

    /// Get overall security score (0-100)
    pub fn security_score(&self) -> u8 {
        if self.total == 0 {
            return 100;
        }
        let blocked_rate = self.blocked as f64 / self.total as f64;
        let fp_penalty = self.false_positive_rate * 0.5;
        let score = (blocked_rate - fp_penalty) * 100.0;
        score.clamp(0.0, 100.0) as u8
    }
}

// ============================================================================
// UNIT TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use policy_oracle::ActionType;

    fn create_proposal(path: &str, content: &str) -> Proposal {
        Proposal {
            id: Uuid::new_v4(),
            action_type: ActionType::CreateFile {
                path: path.to_string(),
            },
            content: content.to_string(),
            files_affected: vec![path.to_string()],
            llm_confidence: 0.9,
        }
    }

    #[test]
    fn test_contract_allows_rust() {
        let runner = ContractRunner::new();
        let request = GatingRequest::new(create_proposal(
            "src/main.rs",
            "fn main() { println!(\"Hello\"); }",
        ));

        let decision = runner.evaluate(&request).unwrap();
        assert_eq!(decision.verdict, Verdict::Allow);
        assert!(decision.refusal.is_none());
    }

    #[test]
    fn test_contract_blocks_typescript() {
        let runner = ContractRunner::new();
        let request = GatingRequest::new(create_proposal(
            "src/utils.ts",
            "export const foo: string = 'bar';",
        ));

        let decision = runner.evaluate(&request).unwrap();
        assert_eq!(decision.verdict, Verdict::Block);
        assert!(decision.refusal.is_some());

        let refusal = decision.refusal.unwrap();
        assert_eq!(refusal.category, RefusalCategory::ForbiddenLanguage);
        assert_eq!(refusal.code, RefusalCode::Lang100TypeScript);
    }

    #[test]
    fn test_contract_blocks_hardcoded_secrets() {
        let runner = ContractRunner::new();
        let request = GatingRequest::new(create_proposal(
            "config.rs",
            r#"let password = "supersecret123456""#,
        ));

        let decision = runner.evaluate(&request).unwrap();
        assert_eq!(decision.verdict, Verdict::Block);

        let refusal = decision.refusal.unwrap();
        assert_eq!(refusal.category, RefusalCategory::ForbiddenPattern);
    }

    #[test]
    fn test_audit_entry_creation() {
        let runner = ContractRunner::new();
        let request = GatingRequest::new(create_proposal("src/lib.rs", "pub fn hello() {}"));

        let decision = runner.evaluate(&request).unwrap();
        let audit = runner.audit(&request, &decision);

        assert_eq!(audit.request_id, request.request_id);
        assert_eq!(audit.decision_id, decision.decision_id);
        assert_eq!(audit.verdict, Verdict::Allow);
        assert!(audit.refusal_code.is_none());
        assert!(!audit.content_hash.is_empty());
    }

    #[test]
    fn test_test_harness() {
        let mut harness = TestHarness::new();

        let test_case = TestCase {
            name: "allows_rust".to_string(),
            description: "Rust files should be allowed".to_string(),
            request: GatingRequest::new(create_proposal("lib.rs", "pub fn foo() {}")),
            expected_verdict: Verdict::Allow,
            expected_category: None,
            expected_code: None,
        };

        let result = harness.run_test(&test_case);
        assert!(result.passed);

        let summary = harness.summary();
        assert_eq!(summary.total, 1);
        assert_eq!(summary.passed, 1);
        assert!(summary.all_passed());
    }

    #[test]
    fn test_refusal_code_numeric() {
        assert_eq!(RefusalCode::Lang100TypeScript.numeric(), 100);
        assert_eq!(RefusalCode::Tool200NpmWithoutDeno.numeric(), 200);
        assert_eq!(RefusalCode::Sec300HardcodedSecret.numeric(), 300);
        assert_eq!(RefusalCode::Spirit500Verbosity.numeric(), 500);
        assert_eq!(RefusalCode::Sys999Unknown.numeric(), 999);
    }

    #[test]
    fn test_verdict_exit_codes() {
        assert_eq!(Verdict::Allow.exit_code(), 0);
        assert_eq!(Verdict::Block.exit_code(), 1);
        assert_eq!(Verdict::Warn.exit_code(), 2);
        assert_eq!(Verdict::Escalate.exit_code(), 3);
    }

    #[test]
    fn test_python_exception_in_salt() {
        let runner = ContractRunner::new();
        let request = GatingRequest::new(create_proposal(
            "salt/config.py",
            "import os\ndef configure(): pass",
        ));

        let decision = runner.evaluate(&request).unwrap();
        assert_eq!(decision.verdict, Verdict::Allow);
    }
}
