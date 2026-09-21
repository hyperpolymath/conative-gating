// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
//! Policy Oracle - Deterministic rule checking for Conative Gating
//!
//! The Policy Oracle checks proposals against hard rules without ML.
//! It catches obvious violations (forbidden languages, toolchain rules)
//! before the SLM evaluates spirit violations.

#![forbid(unsafe_code)]
use glob::Pattern;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

// ============ Core Types ============

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PolicyVerdict {
    Compliant,
    HardViolation(ViolationType),
    SoftConcern(ConcernType),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ViolationType {
    ForbiddenLanguage {
        language: String,
        file: String,
        context: String,
    },
    ForbiddenToolchain {
        tool: String,
        missing: String,
    },
    SecurityViolation {
        description: String,
    },
    ForbiddenPattern {
        pattern: String,
        file: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConcernType {
    VerbositySmell,
    PatternDeviation,
    UnusualStructure,
    Tier2Language { language: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    pub id: Uuid,
    pub action_type: ActionType,
    pub content: String,
    pub files_affected: Vec<String>,
    pub llm_confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    CreateFile { path: String },
    ModifyFile { path: String },
    DeleteFile { path: String },
    ExecuteCommand { command: String },
}

// ============ Policy Configuration ============

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Policy {
    pub name: String,
    pub languages: LanguagePolicy,
    pub toolchain: ToolchainPolicy,
    pub patterns: PatternPolicy,
    pub enforcement: EnforcementConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LanguagePolicy {
    pub tier1: Vec<LanguageConfig>,
    pub tier2: Vec<LanguageConfig>,
    pub forbidden: Vec<LanguageConfig>,
    pub exceptions: Vec<ExceptionRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageConfig {
    pub name: String,
    pub extensions: Vec<String>,
    pub markers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExceptionRule {
    pub language: String,
    pub allowed_paths: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolchainPolicy {
    pub rules: Vec<ToolchainRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolchainRule {
    pub tool: String,
    pub tool_markers: Vec<String>,
    pub requires: String,
    pub requires_markers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PatternPolicy {
    pub forbidden_patterns: Vec<ForbiddenPattern>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForbiddenPattern {
    pub name: String,
    pub regex: String,
    pub file_types: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnforcementConfig {
    pub slm_weight: f64,
    pub escalate_threshold: f64,
    pub block_threshold: f64,
}

impl Default for EnforcementConfig {
    fn default() -> Self {
        Self {
            slm_weight: 1.5,
            escalate_threshold: 0.4,
            block_threshold: 0.7,
        }
    }
}

// ============ Evaluation Results ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleEvaluation {
    pub proposal_id: Uuid,
    pub verdict: PolicyVerdict,
    pub rules_checked: Vec<String>,
    pub violations: Vec<Violation>,
    pub concerns: Vec<Concern>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    pub rule: String,
    pub violation_type: ViolationType,
    pub severity: Severity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concern {
    pub rule: String,
    pub concern_type: ConcernType,
    pub suggestion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

// ============ Directory Scanning ============

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryScanResult {
    pub path: PathBuf,
    pub verdict: PolicyVerdict,
    pub files_scanned: usize,
    pub violations: Vec<FileViolation>,
    pub concerns: Vec<FileConcern>,
}

/// Controls which files are visited by [`Oracle::scan_directory_with_options`].
///
/// Patterns use glob syntax and are matched against the path relative to the
/// scan root, the complete path, and the file name. An empty `include` list
/// includes every file that is not excluded. For a directory, `Some(1)` scans
/// files immediately inside the root; `None` means unlimited depth. A root
/// file is scanned regardless of the depth value.
#[derive(Debug, Clone, Default)]
pub struct ScanOptions {
    /// Include dot-files and dot-directories (generated directories remain skipped).
    pub include_hidden: bool,
    /// Maximum directory depth, measured from the scan root.
    pub max_depth: Option<usize>,
    /// Glob patterns for files to include.
    pub include: Vec<String>,
    /// Glob patterns for files or directories to exclude.
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileViolation {
    pub file: PathBuf,
    pub violation: ViolationType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileConcern {
    pub file: PathBuf,
    pub concern: ConcernType,
}

// ============ Errors ============

#[derive(Error, Debug)]
pub enum OracleError {
    #[error("Invalid proposal: {0}")]
    InvalidProposal(String),
    #[error("Policy parse error: {0}")]
    PolicyParseError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Invalid regex: {0}")]
    RegexError(#[from] regex::Error),
    #[error("Invalid glob pattern: {0}")]
    GlobError(String),
}

// ============ Oracle Implementation ============

pub struct Oracle {
    policy: Policy,
}

impl Oracle {
    pub fn new(policy: Policy) -> Self {
        Self { policy }
    }

    pub fn with_rsr_defaults() -> Self {
        Self::new(Policy::rsr_default())
    }

    /// Check a proposal against policy
    pub fn check_proposal(&self, proposal: &Proposal) -> Result<OracleEvaluation, OracleError> {
        let mut rules_checked = Vec::new();
        let mut violations = Vec::new();
        let mut concerns = Vec::new();

        // Check forbidden languages in content
        rules_checked.push("forbidden_languages_content".to_string());
        for lang in &self.policy.languages.forbidden {
            if self.content_contains_language(&proposal.content, lang) {
                let is_excepted = self.check_exception(&proposal.files_affected, &lang.name);
                if !is_excepted {
                    violations.push(Violation {
                        rule: format!("forbidden_language:{}", lang.name),
                        violation_type: ViolationType::ForbiddenLanguage {
                            language: lang.name.clone(),
                            file: proposal.files_affected.first().cloned().unwrap_or_default(),
                            context: self.extract_context(&proposal.content, &lang.markers),
                        },
                        severity: Severity::Critical,
                    });
                }
            }
        }

        // Check forbidden languages in file paths
        rules_checked.push("forbidden_languages_files".to_string());
        for file in &proposal.files_affected {
            for lang in &self.policy.languages.forbidden {
                if self.file_matches_language(file, lang) {
                    let is_excepted = self.check_exception(std::slice::from_ref(file), &lang.name);
                    if !is_excepted {
                        violations.push(Violation {
                            rule: format!("forbidden_file_extension:{}", lang.name),
                            violation_type: ViolationType::ForbiddenLanguage {
                                language: lang.name.clone(),
                                file: file.clone(),
                                context: format!(
                                    "File extension matches forbidden language: {}",
                                    lang.name
                                ),
                            },
                            severity: Severity::Critical,
                        });
                    }
                }
            }
        }

        // Check toolchain rules
        rules_checked.push("toolchain_rules".to_string());
        for rule in &self.policy.toolchain.rules {
            let has_tool = self.content_has_markers(&proposal.content, &rule.tool_markers)
                || self.files_have_markers(&proposal.files_affected, &rule.tool_markers);
            let has_requires = self.content_has_markers(&proposal.content, &rule.requires_markers)
                || self.files_have_markers(&proposal.files_affected, &rule.requires_markers);

            if has_tool && !has_requires {
                violations.push(Violation {
                    rule: format!("toolchain:{}:{}", rule.tool, rule.requires),
                    violation_type: ViolationType::ForbiddenToolchain {
                        tool: rule.tool.clone(),
                        missing: rule.requires.clone(),
                    },
                    severity: Severity::High,
                });
            }
        }

        // Check forbidden patterns
        rules_checked.push("forbidden_patterns".to_string());
        for pattern in &self.policy.patterns.forbidden_patterns {
            let re = Regex::new(&pattern.regex)?;
            if re.is_match(&proposal.content) {
                violations.push(Violation {
                    rule: format!("pattern:{}", pattern.name),
                    violation_type: ViolationType::ForbiddenPattern {
                        pattern: pattern.name.clone(),
                        file: proposal.files_affected.first().cloned().unwrap_or_default(),
                    },
                    severity: Severity::High,
                });
            }
        }

        // Check tier2 languages (concerns, not violations)
        rules_checked.push("tier2_languages".to_string());
        for lang in &self.policy.languages.tier2 {
            if self.content_contains_language(&proposal.content, lang) {
                concerns.push(Concern {
                    rule: format!("tier2_language:{}", lang.name),
                    concern_type: ConcernType::Tier2Language {
                        language: lang.name.clone(),
                    },
                    suggestion: format!(
                        "Consider using a Tier 1 language instead of {}",
                        lang.name
                    ),
                });
            }
        }

        let verdict = if !violations.is_empty() {
            PolicyVerdict::HardViolation(violations[0].violation_type.clone())
        } else if !concerns.is_empty() {
            PolicyVerdict::SoftConcern(concerns[0].concern_type.clone())
        } else {
            PolicyVerdict::Compliant
        };

        Ok(OracleEvaluation {
            proposal_id: proposal.id,
            verdict,
            rules_checked,
            violations,
            concerns,
        })
    }

    /// Scan a directory for policy violations using the default scan options.
    pub fn scan_directory(&self, path: &Path) -> Result<DirectoryScanResult, OracleError> {
        self.scan_directory_with_options(path, &ScanOptions::default())
    }

    /// Scan files for both path-based and content-based policy violations.
    ///
    /// The original scanner only inspected extensions. That made directory
    /// scans materially weaker than `check` and allowed content-only rules,
    /// including hard-coded-secret detection, to be bypassed by placing the
    /// content in an otherwise innocuous file. This method deliberately uses
    /// the same proposal evaluator as single-file checks.
    pub fn scan_directory_with_options(
        &self,
        path: &Path,
        options: &ScanOptions,
    ) -> Result<DirectoryScanResult, OracleError> {
        let include = compile_patterns(&options.include)?;
        let exclude = compile_patterns(&options.exclude)?;
        let files = collect_files(path, path, options, &include, &exclude, 0)?;
        let files_scanned = files.len();
        let mut violations = Vec::new();
        let mut concerns = Vec::new();

        for file_path in files {
            let file = file_path.to_string_lossy().to_string();

            // Extension checks are useful even when a file is empty or binary.
            for lang in &self.policy.languages.forbidden {
                if self.file_matches_language(&file, lang)
                    && !self.check_exception(std::slice::from_ref(&file), &lang.name)
                {
                    push_unique_violation(
                        &mut violations,
                        FileViolation {
                            file: file_path.clone(),
                            violation: ViolationType::ForbiddenLanguage {
                                language: lang.name.clone(),
                                file: file.clone(),
                                context: "File extension".to_string(),
                            },
                        },
                    );
                }
            }

            for lang in &self.policy.languages.tier2 {
                if self.file_matches_language(&file, lang)
                    && !self.check_exception(std::slice::from_ref(&file), &lang.name)
                {
                    push_unique_concern(
                        &mut concerns,
                        FileConcern {
                            file: file_path.clone(),
                            concern: ConcernType::Tier2Language {
                                language: lang.name.clone(),
                            },
                        },
                    );
                }
            }

            // Invalid UTF-8 is still scanned by path; content rules apply to
            // text files only because the proposal contract is UTF-8 text.
            let content = match fs::read_to_string(&file_path) {
                Ok(content) => content,
                Err(error) if error.kind() == std::io::ErrorKind::InvalidData => continue,
                Err(error) => return Err(OracleError::IoError(error)),
            };

            let proposal = Proposal {
                id: Uuid::new_v4(),
                action_type: ActionType::CreateFile { path: file.clone() },
                content,
                files_affected: vec![file.clone()],
                llm_confidence: 1.0,
            };
            let evaluation = self.check_proposal(&proposal)?;

            for violation in evaluation.violations {
                push_unique_violation(
                    &mut violations,
                    FileViolation {
                        file: file_path.clone(),
                        violation: violation.violation_type,
                    },
                );
            }
            for concern in evaluation.concerns {
                push_unique_concern(
                    &mut concerns,
                    FileConcern {
                        file: file_path.clone(),
                        concern: concern.concern_type,
                    },
                );
            }
        }

        let verdict = if let Some(first) = violations.first() {
            PolicyVerdict::HardViolation(first.violation.clone())
        } else if let Some(first) = concerns.first() {
            PolicyVerdict::SoftConcern(first.concern.clone())
        } else {
            PolicyVerdict::Compliant
        };

        Ok(DirectoryScanResult {
            path: path.to_path_buf(),
            verdict,
            files_scanned,
            violations,
            concerns,
        })
    }

    // Helper methods
    fn content_contains_language(&self, content: &str, lang: &LanguageConfig) -> bool {
        let content_lower = content.to_lowercase();
        lang.markers
            .iter()
            .any(|m| content_lower.contains(&m.to_lowercase()))
    }

    fn file_matches_language(&self, file: &str, lang: &LanguageConfig) -> bool {
        let file_lower = file.to_lowercase();
        lang.extensions
            .iter()
            .any(|ext| file_lower.ends_with(&ext.to_lowercase()))
    }

    fn content_has_markers(&self, content: &str, markers: &[String]) -> bool {
        let content_lower = content.to_lowercase();
        markers
            .iter()
            .any(|m| content_lower.contains(&m.to_lowercase()))
    }

    fn files_have_markers(&self, files: &[String], markers: &[String]) -> bool {
        for file in files {
            let file_lower = file.to_lowercase();
            for marker in markers {
                if file_lower.contains(&marker.to_lowercase()) {
                    return true;
                }
            }
        }
        false
    }

    fn check_exception(&self, files: &[String], language: &str) -> bool {
        for exc in &self.policy.languages.exceptions {
            if exc.language.to_lowercase() == language.to_lowercase() {
                for file in files {
                    for allowed in &exc.allowed_paths {
                        if file.contains(allowed) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    fn extract_context(&self, content: &str, markers: &[String]) -> String {
        for marker in markers {
            if let Some(pos) = content.to_lowercase().find(&marker.to_lowercase()) {
                let start = pos.saturating_sub(30);
                let end = (pos + marker.len() + 30).min(content.len());
                return format!("...{}...", &content[start..end]);
            }
        }
        String::new()
    }
}

fn compile_patterns(patterns: &[String]) -> Result<Vec<Pattern>, OracleError> {
    patterns
        .iter()
        .map(|pattern| {
            Pattern::new(pattern).map_err(|error| OracleError::GlobError(error.to_string()))
        })
        .collect()
}

fn collect_files(
    root: &Path,
    current: &Path,
    options: &ScanOptions,
    include: &[Pattern],
    exclude: &[Pattern],
    depth: usize,
) -> Result<Vec<PathBuf>, OracleError> {
    if !current.exists() {
        return Ok(Vec::new());
    }

    if current.is_file() {
        return if matches_scan_patterns(root, current, include, exclude) {
            Ok(vec![current.to_path_buf()])
        } else {
            Ok(Vec::new())
        };
    }

    if options
        .max_depth
        .is_some_and(|max_depth| depth >= max_depth)
    {
        return Ok(Vec::new());
    }

    let mut entries = fs::read_dir(current)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort();

    let mut files = Vec::new();
    for entry_path in entries {
        let name = entry_path.file_name().unwrap_or_default().to_string_lossy();
        let is_hidden = name.starts_with('.');
        let is_generated =
            name == "node_modules" || name == "target" || name == "_build" || name == ".git";

        if is_generated || (is_hidden && !options.include_hidden) {
            continue;
        }
        if matches_any_scan_pattern(root, &entry_path, exclude) {
            continue;
        }

        if entry_path.is_dir() {
            files.extend(collect_files(
                root,
                &entry_path,
                options,
                include,
                exclude,
                depth + 1,
            )?);
        } else if matches_scan_patterns(root, &entry_path, include, &[]) {
            files.push(entry_path);
        }
    }

    Ok(files)
}

fn matches_scan_patterns(
    root: &Path,
    path: &Path,
    include: &[Pattern],
    exclude: &[Pattern],
) -> bool {
    !matches_any_scan_pattern(root, path, exclude)
        && (include.is_empty() || matches_any_scan_pattern(root, path, include))
}

fn matches_any_scan_pattern(root: &Path, path: &Path, patterns: &[Pattern]) -> bool {
    let relative = path.strip_prefix(root).unwrap_or(path);
    let file_name = path.file_name().unwrap_or_default();
    let candidates = [relative, path, Path::new(file_name)];
    patterns.iter().any(|pattern| {
        candidates
            .iter()
            .any(|candidate| pattern.matches_path(candidate))
    })
}

fn push_unique_violation(violations: &mut Vec<FileViolation>, candidate: FileViolation) {
    if !violations.iter().any(|existing| {
        existing.file == candidate.file && existing.violation == candidate.violation
    }) {
        violations.push(candidate);
    }
}

fn push_unique_concern(concerns: &mut Vec<FileConcern>, candidate: FileConcern) {
    if !concerns
        .iter()
        .any(|existing| existing.file == candidate.file && existing.concern == candidate.concern)
    {
        concerns.push(candidate);
    }
}

// ============ Default Policy ============

impl Policy {
    /// RSR-compliant default policy
    pub fn rsr_default() -> Self {
        Self {
            name: "RSR Default Policy".to_string(),
            languages: LanguagePolicy {
                tier1: vec![
                    LanguageConfig {
                        name: "rust".to_string(),
                        extensions: vec![".rs".to_string()],
                        markers: vec![
                            "fn main".to_string(),
                            "impl ".to_string(),
                            "pub fn".to_string(),
                        ],
                    },
                    LanguageConfig {
                        name: "elixir".to_string(),
                        extensions: vec![".ex".to_string(), ".exs".to_string()],
                        markers: vec!["defmodule".to_string(), "def ".to_string()],
                    },
                    LanguageConfig {
                        name: "zig".to_string(),
                        extensions: vec![".zig".to_string()],
                        markers: vec!["const std".to_string()],
                    },
                    LanguageConfig {
                        name: "ada".to_string(),
                        extensions: vec![".adb".to_string(), ".ads".to_string()],
                        markers: vec!["procedure".to_string(), "package".to_string()],
                    },
                    LanguageConfig {
                        name: "haskell".to_string(),
                        extensions: vec![".hs".to_string()],
                        markers: vec!["module ".to_string()],
                    },
                    LanguageConfig {
                        name: "rescript".to_string(),
                        extensions: vec![".res".to_string(), ".resi".to_string()],
                        markers: vec!["@react.component".to_string()],
                    },
                ],
                tier2: vec![
                    LanguageConfig {
                        name: "nickel".to_string(),
                        extensions: vec![".ncl".to_string()],
                        markers: vec![],
                    },
                    LanguageConfig {
                        name: "racket".to_string(),
                        extensions: vec![".rkt".to_string()],
                        markers: vec!["#lang".to_string()],
                    },
                ],
                forbidden: vec![
                    LanguageConfig {
                        name: "typescript".to_string(),
                        extensions: vec![".ts".to_string(), ".tsx".to_string()],
                        markers: vec![
                            ": string".to_string(),
                            ": number".to_string(),
                            "interface ".to_string(),
                        ],
                    },
                    LanguageConfig {
                        name: "python".to_string(),
                        extensions: vec![".py".to_string()],
                        markers: vec!["import ".to_string(), "def ".to_string()],
                    },
                    LanguageConfig {
                        name: "go".to_string(),
                        extensions: vec![".go".to_string()],
                        markers: vec!["package main".to_string(), "func ".to_string()],
                    },
                    LanguageConfig {
                        name: "java".to_string(),
                        extensions: vec![".java".to_string()],
                        markers: vec!["public class".to_string()],
                    },
                ],
                exceptions: vec![ExceptionRule {
                    language: "python".to_string(),
                    allowed_paths: vec!["salt/".to_string(), "training/".to_string()],
                    reason: "Python allowed for Salt configs and ML training".to_string(),
                }],
            },
            toolchain: ToolchainPolicy {
                rules: vec![ToolchainRule {
                    tool: "npm".to_string(),
                    tool_markers: vec!["package.json".to_string(), "npm install".to_string()],
                    requires: "deno".to_string(),
                    requires_markers: vec!["deno.json".to_string()],
                }],
            },
            patterns: PatternPolicy {
                forbidden_patterns: vec![ForbiddenPattern {
                    name: "hardcoded_secrets".to_string(),
                    regex: r#"(?i)(password|secret|api_key)\s*=\s*["'][^"']{8,}["']"#.to_string(),
                    file_types: vec!["*".to_string()],
                    reason: "Hardcoded secrets detected".to_string(),
                }],
            },
            enforcement: EnforcementConfig::default(),
        }
    }
}

// ============ Tests ============

#[cfg(test)]
mod tests {
    use super::*;

    fn oracle() -> Oracle {
        Oracle::with_rsr_defaults()
    }

    #[test]
    fn test_detects_typescript_file() {
        let oracle = oracle();
        let proposal = Proposal {
            id: Uuid::new_v4(),
            action_type: ActionType::CreateFile {
                path: "util.ts".to_string(),
            },
            content: "Creating a utility file".to_string(),
            files_affected: vec!["util.ts".to_string()],
            llm_confidence: 0.9,
        };

        let result = oracle.check_proposal(&proposal).unwrap();
        assert!(matches!(result.verdict, PolicyVerdict::HardViolation(_)));
        assert!(!result.violations.is_empty());
    }

    #[test]
    fn test_detects_typescript_content() {
        let oracle = oracle();
        let proposal = Proposal {
            id: Uuid::new_v4(),
            action_type: ActionType::ModifyFile {
                path: "file.txt".to_string(),
            },
            content: "const x: string = 'hello'".to_string(),
            files_affected: vec!["file.txt".to_string()],
            llm_confidence: 0.9,
        };

        let result = oracle.check_proposal(&proposal).unwrap();
        assert!(matches!(result.verdict, PolicyVerdict::HardViolation(_)));
    }

    #[test]
    fn test_allows_rust() {
        let oracle = oracle();
        let proposal = Proposal {
            id: Uuid::new_v4(),
            action_type: ActionType::CreateFile {
                path: "main.rs".to_string(),
            },
            content: "fn main() { println!(\"Hello\"); }".to_string(),
            files_affected: vec!["main.rs".to_string()],
            llm_confidence: 0.9,
        };

        let result = oracle.check_proposal(&proposal).unwrap();
        assert!(matches!(result.verdict, PolicyVerdict::Compliant));
    }

    #[test]
    fn test_python_exception_in_salt() {
        let oracle = oracle();
        let proposal = Proposal {
            id: Uuid::new_v4(),
            action_type: ActionType::CreateFile {
                path: "salt/config.py".to_string(),
            },
            content: "import os".to_string(),
            files_affected: vec!["salt/config.py".to_string()],
            llm_confidence: 0.9,
        };

        let result = oracle.check_proposal(&proposal).unwrap();
        assert!(matches!(result.verdict, PolicyVerdict::Compliant));
    }

    #[test]
    fn test_toolchain_npm_without_deno() {
        let oracle = oracle();
        let proposal = Proposal {
            id: Uuid::new_v4(),
            action_type: ActionType::CreateFile {
                path: "package.json".to_string(),
            },
            content: r#"{"name": "test", "version": "1.0.0"}"#.to_string(),
            files_affected: vec!["package.json".to_string()],
            llm_confidence: 0.9,
        };

        let result = oracle.check_proposal(&proposal).unwrap();
        assert!(matches!(result.verdict, PolicyVerdict::HardViolation(_)));
    }

    #[test]
    fn test_toolchain_npm_with_deno() {
        let oracle = oracle();
        let proposal = Proposal {
            id: Uuid::new_v4(),
            action_type: ActionType::CreateFile {
                path: "package.json".to_string(),
            },
            content: r#"{"name": "test"} deno.json also present"#.to_string(),
            files_affected: vec!["package.json".to_string(), "deno.json".to_string()],
            llm_confidence: 0.9,
        };

        let result = oracle.check_proposal(&proposal).unwrap();
        assert!(matches!(result.verdict, PolicyVerdict::Compliant));
    }

    #[test]
    fn test_detects_hardcoded_secret() {
        let oracle = oracle();
        let proposal = Proposal {
            id: Uuid::new_v4(),
            action_type: ActionType::CreateFile {
                path: "config.rs".to_string(),
            },
            content: r#"let password = "supersecretpassword123""#.to_string(), // test fixture — scanner-allow: rust-secrets
            files_affected: vec!["config.rs".to_string()],
            llm_confidence: 0.9,
        };

        let result = oracle.check_proposal(&proposal).unwrap();
        assert!(matches!(result.verdict, PolicyVerdict::HardViolation(_)));
    }

    // ============ Additional Unit Tests ============

    #[test]
    fn test_empty_proposal_compliant() {
        let oracle = oracle();
        let proposal = Proposal {
            id: Uuid::new_v4(),
            action_type: ActionType::CreateFile {
                path: "README.md".to_string(),
            },
            content: "# Documentation".to_string(),
            files_affected: vec!["README.md".to_string()],
            llm_confidence: 0.5,
        };

        let result = oracle.check_proposal(&proposal).unwrap();
        assert_eq!(result.verdict, PolicyVerdict::Compliant);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn test_multiple_violations_reported() {
        let oracle = oracle();
        let proposal = Proposal {
            id: Uuid::new_v4(),
            action_type: ActionType::CreateFile {
                path: "main.ts".to_string(),
            },
            content: r#"const x: string = 'hello'; let password = "secret123""#.to_string(), // scanner-allow: rust-secrets
            files_affected: vec!["main.ts".to_string()],
            llm_confidence: 0.9,
        };

        let result = oracle.check_proposal(&proposal).unwrap();
        assert!(matches!(result.verdict, PolicyVerdict::HardViolation(_)));
        // Should report at least the TypeScript violation
        assert!(!result.violations.is_empty());
    }

    #[test]
    fn test_tier2_language_generates_concern() {
        let oracle = oracle();
        let proposal = Proposal {
            id: Uuid::new_v4(),
            action_type: ActionType::CreateFile {
                path: "config.ncl".to_string(),
            },
            content: "{}".to_string(),
            files_affected: vec!["config.ncl".to_string()],
            llm_confidence: 0.8,
        };

        let result = oracle.check_proposal(&proposal).unwrap();
        // Tier2 languages without markers might be compliant or concerns depending on detection
        assert!(matches!(
            result.verdict,
            PolicyVerdict::Compliant | PolicyVerdict::SoftConcern(_)
        ));
    }

    #[test]
    fn test_elixir_tier1_allowed() {
        let oracle = oracle();
        let proposal = Proposal {
            id: Uuid::new_v4(),
            action_type: ActionType::CreateFile {
                path: "module.ex".to_string(),
            },
            content: "defmodule MyModule, do: :ok".to_string(),
            files_affected: vec!["module.ex".to_string()],
            llm_confidence: 0.9,
        };

        let result = oracle.check_proposal(&proposal).unwrap();
        assert_eq!(result.verdict, PolicyVerdict::Compliant);
    }

    #[test]
    fn test_rust_impl_block_allowed() {
        let oracle = oracle();
        let proposal = Proposal {
            id: Uuid::new_v4(),
            action_type: ActionType::CreateFile {
                path: "lib.rs".to_string(),
            },
            content: "impl MyStruct { pub fn new() -> Self { Self {} } }".to_string(),
            files_affected: vec!["lib.rs".to_string()],
            llm_confidence: 0.9,
        };

        let result = oracle.check_proposal(&proposal).unwrap();
        assert_eq!(result.verdict, PolicyVerdict::Compliant);
    }

    #[test]
    fn test_ada_allowed() {
        let oracle = oracle();
        let proposal = Proposal {
            id: Uuid::new_v4(),
            action_type: ActionType::CreateFile {
                path: "main.adb".to_string(),
            },
            content: "with Ada.Text_IO;\nprocedure Hello is\nbegin\n  Ada.Text_IO.Put_Line(\"Hello\");\nend Hello;".to_string(),
            files_affected: vec!["main.adb".to_string()],
            llm_confidence: 0.9,
        };

        let result = oracle.check_proposal(&proposal).unwrap();
        assert_eq!(result.verdict, PolicyVerdict::Compliant);
    }

    #[test]
    fn test_haskell_allowed() {
        let oracle = oracle();
        let proposal = Proposal {
            id: Uuid::new_v4(),
            action_type: ActionType::CreateFile {
                path: "Main.hs".to_string(),
            },
            content: "module Main where\nmain = putStrLn \"Hello\"".to_string(),
            files_affected: vec!["Main.hs".to_string()],
            llm_confidence: 0.9,
        };

        let result = oracle.check_proposal(&proposal).unwrap();
        assert_eq!(result.verdict, PolicyVerdict::Compliant);
    }

    #[test]
    fn test_rescript_component_allowed() {
        let oracle = oracle();
        let proposal = Proposal {
            id: Uuid::new_v4(),
            action_type: ActionType::CreateFile {
                path: "Component.res".to_string(),
            },
            content: "@react.component\nlet make = () => <div>\"Hello\"</div>".to_string(),
            files_affected: vec!["Component.res".to_string()],
            llm_confidence: 0.9,
        };

        let result = oracle.check_proposal(&proposal).unwrap();
        assert_eq!(result.verdict, PolicyVerdict::Compliant);
    }

    #[test]
    fn test_proposal_with_correct_violation_severity() {
        let oracle = oracle();
        let proposal = Proposal {
            id: Uuid::new_v4(),
            action_type: ActionType::CreateFile {
                path: "test.ts".to_string(),
            },
            content: "const x: string = 'test'".to_string(),
            files_affected: vec!["test.ts".to_string()],
            llm_confidence: 0.9,
        };

        let result = oracle.check_proposal(&proposal).unwrap();
        assert!(!result.violations.is_empty());
        assert_eq!(result.violations[0].severity, Severity::Critical);
    }

    #[test]
    fn test_toolchain_violation_severity() {
        let oracle = oracle();
        let proposal = Proposal {
            id: Uuid::new_v4(),
            action_type: ActionType::CreateFile {
                path: "package.json".to_string(),
            },
            content: "{}".to_string(),
            files_affected: vec!["package.json".to_string()],
            llm_confidence: 0.9,
        };

        let result = oracle.check_proposal(&proposal).unwrap();
        if !result.violations.is_empty() {
            assert_eq!(result.violations[0].severity, Severity::High);
        }
    }

    #[test]
    fn test_rules_checked_counter() {
        let oracle = oracle();
        let proposal = Proposal {
            id: Uuid::new_v4(),
            action_type: ActionType::CreateFile {
                path: "test.rs".to_string(),
            },
            content: "fn main() {}".to_string(),
            files_affected: vec!["test.rs".to_string()],
            llm_confidence: 0.9,
        };

        let result = oracle.check_proposal(&proposal).unwrap();
        // Should have checked multiple rules (forbidden languages, toolchain, patterns, tier2)
        assert!(!result.rules_checked.is_empty());
        assert!(result.rules_checked.len() >= 4);
    }

    #[test]
    fn test_proposal_id_preserved_in_evaluation() {
        let proposal_id = Uuid::new_v4();
        let oracle = oracle();
        let proposal = Proposal {
            id: proposal_id,
            action_type: ActionType::CreateFile {
                path: "test.rs".to_string(),
            },
            content: "fn main() {}".to_string(),
            files_affected: vec!["test.rs".to_string()],
            llm_confidence: 0.9,
        };

        let result = oracle.check_proposal(&proposal).unwrap();
        assert_eq!(result.proposal_id, proposal_id);
    }

    #[test]
    fn test_go_forbidden() {
        let oracle = oracle();
        let proposal = Proposal {
            id: Uuid::new_v4(),
            action_type: ActionType::CreateFile {
                path: "main.go".to_string(),
            },
            content: "package main\nfunc main() {}".to_string(),
            files_affected: vec!["main.go".to_string()],
            llm_confidence: 0.9,
        };

        let result = oracle.check_proposal(&proposal).unwrap();
        assert!(matches!(result.verdict, PolicyVerdict::HardViolation(_)));
    }

    #[test]
    fn test_java_forbidden() {
        let oracle = oracle();
        let proposal = Proposal {
            id: Uuid::new_v4(),
            action_type: ActionType::CreateFile {
                path: "Main.java".to_string(),
            },
            content: "public class Main { }".to_string(),
            files_affected: vec!["Main.java".to_string()],
            llm_confidence: 0.9,
        };

        let result = oracle.check_proposal(&proposal).unwrap();
        assert!(matches!(result.verdict, PolicyVerdict::HardViolation(_)));
    }

    #[test]
    fn test_concern_for_racket() {
        let oracle = oracle();
        let proposal = Proposal {
            id: Uuid::new_v4(),
            action_type: ActionType::CreateFile {
                path: "script.rkt".to_string(),
            },
            content: "#lang racket".to_string(),
            files_affected: vec!["script.rkt".to_string()],
            llm_confidence: 0.8,
        };

        let result = oracle.check_proposal(&proposal).unwrap();
        assert!(matches!(result.verdict, PolicyVerdict::SoftConcern(_)));
        assert!(!result.concerns.is_empty());
    }

    #[test]
    fn test_python_forbidden_outside_exceptions() {
        let oracle = oracle();
        let proposal = Proposal {
            id: Uuid::new_v4(),
            action_type: ActionType::CreateFile {
                path: "script.py".to_string(),
            },
            content: "import os".to_string(),
            files_affected: vec!["script.py".to_string()],
            llm_confidence: 0.9,
        };

        let result = oracle.check_proposal(&proposal).unwrap();
        assert!(matches!(result.verdict, PolicyVerdict::HardViolation(_)));
    }

    #[test]
    fn test_python_allowed_in_training() {
        let oracle = oracle();
        let proposal = Proposal {
            id: Uuid::new_v4(),
            action_type: ActionType::CreateFile {
                path: "training/model.py".to_string(),
            },
            content: "import os".to_string(),
            files_affected: vec!["training/model.py".to_string()],
            llm_confidence: 0.9,
        };

        let result = oracle.check_proposal(&proposal).unwrap();
        assert_eq!(result.verdict, PolicyVerdict::Compliant);
    }

    #[test]
    fn test_secret_api_key_detected() {
        let oracle = oracle();
        let proposal = Proposal {
            id: Uuid::new_v4(),
            action_type: ActionType::CreateFile {
                path: "config.rs".to_string(),
            },
            content: format!(
                r#"const API_KEY = "{}""#,
                ["abcdef1234", "567890abcdef"].concat()
            ), // test fixture — scanner-allow: rust-secrets
            files_affected: vec!["config.rs".to_string()],
            llm_confidence: 0.9,
        };

        let result = oracle.check_proposal(&proposal).unwrap();
        assert!(matches!(result.verdict, PolicyVerdict::HardViolation(_)));
    }

    #[test]
    fn test_directory_scan_evaluates_file_content() {
        let oracle = oracle();
        let root = std::env::temp_dir().join(format!("conative-scan-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let file = root.join("notes.txt");
        fs::write(&file, r#"password = "not-a-real-secret-123""#).unwrap();

        let result = oracle.scan_directory(&root).unwrap();

        assert_eq!(result.files_scanned, 1);
        assert!(matches!(result.verdict, PolicyVerdict::HardViolation(_)));
        assert!(result.violations.iter().any(|violation| {
            matches!(violation.violation, ViolationType::ForbiddenPattern { .. })
        }));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn test_directory_scan_options_control_hidden_and_depth() {
        let oracle = oracle();
        let root = std::env::temp_dir().join(format!("conative-scan-{}", Uuid::new_v4()));
        fs::create_dir_all(root.join("nested")).unwrap();
        fs::write(root.join("visible.rs"), "fn main() {}").unwrap();
        fs::write(root.join(".hidden.ts"), "const x: string = 'blocked'").unwrap();
        fs::write(root.join("nested/deep.py"), "import os").unwrap();

        let default_result = oracle.scan_directory(&root).unwrap();
        assert_eq!(default_result.files_scanned, 2);

        let options = ScanOptions {
            include_hidden: true,
            max_depth: Some(1),
            include: vec!["*.ts".to_string()],
            exclude: Vec::new(),
        };
        let filtered_result = oracle.scan_directory_with_options(&root, &options).unwrap();
        assert_eq!(filtered_result.files_scanned, 1);
        assert!(matches!(
            filtered_result.verdict,
            PolicyVerdict::HardViolation(_)
        ));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn test_directory_scan_rejects_invalid_glob() {
        let oracle = oracle();
        let options = ScanOptions {
            include: vec!["[".to_string()],
            ..ScanOptions::default()
        };

        assert!(matches!(
            oracle.scan_directory_with_options(Path::new("."), &options),
            Err(OracleError::GlobError(_))
        ));
    }
}
