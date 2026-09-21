// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
//! SLM Evaluator - Adversarial policy evaluation using Small Language Models
//!
//! This crate will provide SLM-based evaluation for detecting "spirit violations"
//! that the deterministic oracle cannot catch.
//!
//! ## Future Implementation
//!
//! - Integration with llama.cpp for local SLM inference
//! - PBFT consensus with asymmetric weighting (1.5x for inhibition)
//! - Training data from rhodibot categories

#![forbid(unsafe_code)]
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// SLM evaluation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlmEvaluation {
    pub proposal_id: Uuid,
    pub spirit_score: f64,
    pub confidence: f64,
    pub reasoning: String,
    pub should_block: bool,
}

/// SLM evaluator configuration.
///
/// The inference backend is intentionally not bundled yet. Until a model is
/// loaded, evaluation fails closed with [`SlmError::ModelNotLoaded`] instead
/// of returning a false compliant result.
pub struct SlmEvaluator {
    #[allow(dead_code)]
    model_path: Option<String>,
    #[allow(dead_code)]
    block_threshold: f64,
}

#[derive(Error, Debug)]
pub enum SlmError {
    #[error("Model not loaded")]
    ModelNotLoaded,
    #[error("Inference error: {0}")]
    InferenceError(String),
}

impl SlmEvaluator {
    pub fn new() -> Self {
        Self {
            model_path: None,
            block_threshold: 0.7,
        }
    }

    /// Evaluate content with the configured local model.
    ///
    /// The model backend is not implemented in this prototype. Returning an
    /// error is deliberate: a missing evaluator must never be interpreted as
    /// an affirmative policy decision by a downstream arbiter.
    pub fn evaluate(&self, _content: &str, _context: &str) -> Result<SlmEvaluation, SlmError> {
        match self.model_path.as_deref() {
            None => Err(SlmError::ModelNotLoaded),
            Some(path) => Err(SlmError::InferenceError(format!(
                "SLM backend is not available for model {path}"
            ))),
        }
    }
}

impl Default for SlmEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluation_fails_closed_when_model_is_missing() {
        let evaluator = SlmEvaluator::new();
        assert!(matches!(
            evaluator.evaluate("even forbidden content", "context"),
            Err(SlmError::ModelNotLoaded)
        ));
    }

    #[test]
    fn default_evaluator_is_not_silently_compliant() {
        let evaluator = SlmEvaluator::default();
        let result = evaluator.evaluate("test", "ctx");
        assert!(result.is_err());
    }

    #[test]
    fn configured_model_reports_unavailable_backend() {
        let evaluator = SlmEvaluator {
            model_path: Some("model.gguf".to_string()),
            block_threshold: 0.7,
        };
        assert!(matches!(
            evaluator.evaluate("test", "ctx"),
            Err(SlmError::InferenceError(_))
        ));
    }

    #[test]
    fn test_slm_block_threshold_set() {
        let evaluator = SlmEvaluator::new();
        assert_eq!(evaluator.block_threshold, 0.7);
    }

    #[test]
    fn test_slm_no_model_initially() {
        let evaluator = SlmEvaluator::new();
        assert!(evaluator.model_path.is_none());
    }
}
