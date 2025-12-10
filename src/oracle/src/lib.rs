//! Policy Oracle - Deterministic rule checking for Conative Gating
//!
//! The Policy Oracle checks proposals against hard rules without ML.
//! It catches obvious violations (forbidden languages, toolchain rules)
//! before the SLM evaluates spirit violations.

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

    /// Scan a directory for policy violations
    pub fn scan_directory(&self, path: &Path) -> Result<DirectoryScanResult, OracleError> {
        let mut violations = Vec::new();
        let mut concerns = Vec::new();
        let mut files_scanned = 0;

        for entry in walkdir(path)? {
            files_scanned += 1;
            let file_path = entry.as_path();

            // Check file extension against forbidden languages
            for lang in &self.policy.languages.forbidden {
                if self.file_matches_language(&file_path.to_string_lossy(), lang) {
                    let is_excepted = self
                        .check_exception(&[file_path.to_string_lossy().to_string()], &lang.name);
                    if !is_excepted {
                        violations.push(FileViolation {
                            file: file_path.to_path_buf(),
                            violation: ViolationType::ForbiddenLanguage {
                                language: lang.name.clone(),
                                file: file_path.to_string_lossy().to_string(),
                                context: "File extension".to_string(),
                            },
                        });
                    }
                }
            }

            // Check tier2 languages
            for lang in &self.policy.languages.tier2 {
                if self.file_matches_language(&file_path.to_string_lossy(), lang) {
                    concerns.push(FileConcern {
                        file: file_path.to_path_buf(),
                        concern: ConcernType::Tier2Language {
                            language: lang.name.clone(),
                        },
                    });
                }
            }
        }

        let verdict = if !violations.is_empty() {
            PolicyVerdict::HardViolation(violations[0].violation.clone())
        } else if !concerns.is_empty() {
            PolicyVerdict::SoftConcern(concerns[0].concern.clone())
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

// Simple directory walker
fn walkdir(path: &Path) -> Result<Vec<PathBuf>, OracleError> {
    let mut files = Vec::new();

    if path.is_file() {
        files.push(path.to_path_buf());
        return Ok(files);
    }

    if !path.exists() {
        return Ok(files);
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();

        let name = entry_path.file_name().unwrap_or_default().to_string_lossy();
        if name.starts_with('.') || name == "node_modules" || name == "target" || name == "_build" {
            continue;
        }

        if entry_path.is_dir() {
            files.extend(walkdir(&entry_path)?);
        } else {
            files.push(entry_path);
        }
    }

    Ok(files)
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
            content: r#"let password = "supersecretpassword123""#.to_string(),
            files_affected: vec!["config.rs".to_string()],
            llm_confidence: 0.9,
        };

        let result = oracle.check_proposal(&proposal).unwrap();
        assert!(matches!(result.verdict, PolicyVerdict::HardViolation(_)));
    }
}
