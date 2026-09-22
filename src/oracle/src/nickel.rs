// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
//! Native Nickel (`.ncl`) policy loading via the vendored Bunsenite evaluator.
//!
//! ## Fail-closed design
//!
//! * This module only exists with `--features nickel`. Without the feature,
//!   `.ncl` dispatch in [`crate::Policy::from_policy_file`] returns an error
//!   instead of silently falling back to any default policy.
//! * Nickel `import` statements are **rejected before evaluation**
//!   ([`OracleError::NickelImportUnsupported`]). The vendored Bunsenite
//!   revision builds its Nickel program from only the file name, so import
//!   resolution relies on ambient evaluator behaviour. Rather than accept
//!   partially-resolved or ambient-dependent policies, multi-file imports are
//!   not supported: inline the policy, or export it to JSON. See
//!   `docs/NICKEL-POLICY.adoc`.
//!
//! ## Import scan
//!
//! [`reject_imports`] is a conservative single-pass scanner: it finds the
//! `import` keyword applied to a string literal (`import "foo.ncl"` or
//! `import 'foo.ncl'`, where Nickel's inter-string form is also quoted),
//! ignoring `#` line comments. Multiline/embedded occurrences inside string
//! *values* may cause false positives; those fail closed (policy rejected),
//! which is the safe direction, and they are documented in
//! `docs/NICKEL-POLICY.adoc`.

use crate::{OracleError, Policy};
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

/// `import` applied to a quoted path, e.g. `let base = import "./lib.ncl" in`.
fn import_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"(?:^|[\s=(,\[{])import\s+["']"#).expect("static regex compiles")
    })
}

/// Strip a trailing `#` line comment, respecting simple double-quoted ranges.
/// Nickel multiline strings (`m#"..."#`) are not fully tokenised; an
/// `import "..."` fragment inside one is treated as code (conservative).
fn strip_line_comment(line: &str) -> &str {
    let mut in_string = false;
    let mut prev = '\0';
    for (idx, ch) in line.char_indices() {
        match ch {
            '"' if prev != '\\' => in_string = !in_string,
            '#' if !in_string => return &line[..idx],
            _ => {}
        }
        prev = ch;
    }
    line
}

/// Find the first `import`-of-a-path statement outside line comments.
fn find_import(content: &str) -> Option<String> {
    for line in content.lines() {
        let code = strip_line_comment(line);
        if let Some(found) = import_pattern().find(code) {
            let excerpt: String = code[found.start()..]
                .trim_start()
                .chars()
                .take(48)
                .collect();
            return Some(excerpt);
        }
    }
    None
}

/// Reject any Nickel source that imports another file.
pub fn reject_imports(content: &str) -> Result<(), OracleError> {
    match find_import(content) {
        Some(excerpt) => Err(OracleError::NickelImportUnsupported(excerpt)),
        None => Ok(()),
    }
}

/// Evaluate Nickel source to a [`Policy`] via the vendored Bunsenite evaluator.
///
/// `source_name` is only used for evaluator/error diagnostics (e.g. the file
/// name or `<embedded>`); evaluation does not perform filesystem access beyond
/// what the evaluator itself requires.
pub fn policy_from_nickel_source(content: &str, source_name: &str) -> Result<Policy, OracleError> {
    reject_imports(content)?;

    let loader = bunsenite::NickelLoader::new();
    let value = loader
        .parse_string(content, source_name)
        .map_err(|error| OracleError::NickelEvaluation(error.to_string()))?;

    serde_json::from_value::<Policy>(value).map_err(|error| {
        OracleError::NickelEvaluation(format!(
            "evaluated Nickel policy {source_name} does not match the Policy contract: {error}"
        ))
    })
}

/// Load and evaluate a `.ncl` policy file.
pub fn policy_from_nickel_file(path: &Path) -> Result<Policy, OracleError> {
    let content = std::fs::read_to_string(path).map_err(OracleError::IoError)?;
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    policy_from_nickel_source(&content, &name)
}
