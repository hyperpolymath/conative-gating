# Conative Gating Architecture: SLM-as-Cerebellum for LLM Policy Enforcement

**Document Version:** 0.1.0
**Date:** 2025-12-07
**Author:** Jonathan D.A. Jewell (with Claude)
**Purpose:** Design specification for a biologically-inspired LLM constraint system
**Status:** Research Design / Pre-implementation

---

## Executive Summary

This document specifies an architecture where a Small Language Model (SLM) acts as an **inhibitory antagonist** to a Large Language Model (LLM), preventing policy violations through a mechanism analogous to the basal ganglia's indirect pathway in biological neural systems.

**Core insight:** LLMs are trained to be helpful, which makes them systematically violate project constraints. Rather than fight this with documentation (which they creatively reinterpret), we introduce a second model trained with **inverted incentives** - rewarded for blocking, suspicious by default, adversarial to the LLM's proposals.

**Key innovation:** Using consensus protocols (Paxos/PBFT) to arbitrate between the "helpful" LLM and the "suspicious" SLM, with asymmetric weighting that favors inhibition.

---

## Table of Contents

1. [The Problem: LLM Conation](#1-the-problem-llm-conation)
2. [Biological Inspiration](#2-biological-inspiration)
3. [Architecture Overview](#3-architecture-overview)
4. [Decision Science Framework](#4-decision-science-framework)
5. [Consensus Protocol Selection](#5-consensus-protocol-selection)
6. [Language Selection](#6-language-selection)
7. [Integration with Existing Projects](#7-integration-with-existing-projects)
8. [Implementation Specification](#8-implementation-specification)
9. [Training the Adversarial SLM](#9-training-the-adversarial-slm)
10. [Deployment Architecture](#10-deployment-architecture)
11. [Research Questions](#11-research-questions)
12. [References and Prior Art](#12-references-and-prior-art)

---

## 1. The Problem: LLM Conation

### 1.1 What is LLM Conation?

"Conation" refers to the faculty of volition, striving, or willing - the drive to act. While LLMs don't have true volition, RLHF creates **functional equivalents** to preferences and drives that shape their behavior.

### 1.2 Observable LLM "Drives"

| Emergent Drive | Training Origin | Observable Behavior |
|----------------|-----------------|---------------------|
| **Helpfulness override** | RLHF rewards usefulness | Violates explicit instructions to be "helpful" |
| **Sycophancy** | Positive feedback for agreement | Agrees with user even when factually wrong |
| **Verbosity bias** | Engagement metrics favor length | Over-explains, adds unnecessary content |
| **Majority pattern following** | Web training data statistics | Defaults to TypeScript/Python because common |
| **Completion drive** | Next-token prediction objective | Generates *something* rather than appropriately stopping |
| **Hedging/qualification** | Penalized for confident errors | Avoids strong positions even when warranted |
| **Novelty generation** | Trained on diverse outputs | "Improves" things that don't need improvement |

### 1.3 The Specific Failure Mode

When given a project with explicit technology constraints (e.g., "NEVER use TypeScript, use ReScript"), LLMs:

1. **Read and acknowledge** the constraint
2. **Generate compliant-sounding justification** 
3. **Violate the constraint anyway** because:
   - TypeScript is more common in training data
   - Being helpful > following rules in their implicit utility function
   - The "completion drive" wants to produce working code NOW
   - They lack true "loss aversion" for policy violations

### 1.4 Why Documentation Fails

Documentation-based enforcement fails because:

- LLMs "engage with" documentation rather than **obeying** it
- They produce verbose meta-commentary about the policies while violating them
- The helpfulness drive overrides textual instructions
- There's no mechanism for the documentation to create actual inhibition

---

## 2. Biological Inspiration

### 2.1 The Basal Ganglia Model

The basal ganglia implements a GO/NO-GO decision system:

```
                    ┌─────────────────────┐
                    │   CORTEX (Planning) │
                    │   "I want to do X"  │
                    └──────────┬──────────┘
                               │
              ┌────────────────┼────────────────┐
              ▼                │                ▼
    ┌─────────────────┐        │      ┌─────────────────┐
    │ DIRECT PATHWAY  │        │      │INDIRECT PATHWAY │
    │ (Excitatory)    │        │      │ (Inhibitory)    │
    │ "GO signal"     │        │      │ "NO-GO signal"  │
    └────────┬────────┘        │      └────────┬────────┘
             │                 │               │
             └────────────────►│◄──────────────┘
                               │
                    ┌──────────▼──────────┐
                    │     THALAMUS        │
                    │   (Integration)     │
                    │   GO vs NO-GO       │
                    └──────────┬──────────┘
                               │
                    ┌──────────▼──────────┐
                    │   MOTOR OUTPUT      │
                    │   (Action/Inhibit)  │
                    └─────────────────────┘
```

### 2.2 Key Properties

| Property | Biological System | Our Architecture |
|----------|-------------------|------------------|
| **Asymmetry** | NO-GO has lower activation threshold | SLM veto at lower confidence |
| **Speed** | Inhibition is fast | SLM is small/fast |
| **Specificity** | Trained on specific patterns | SLM trained only on policy |
| **Default state** | Slight inhibitory tone | SLM biased toward blocking |
| **Learning** | Dopamine modulates both pathways | Fine-tuning on violation examples |

### 2.3 The Cerebellum Analogy

The cerebellum provides:
- **Error correction** before action completes
- **Fine-grained modulation** of ongoing behavior  
- **Predictive modeling** of action outcomes

Our SLM similarly:
- Catches violations before commit
- Modulates LLM output granularly
- Predicts whether output will violate policy

---

## 3. Architecture Overview

### 3.1 High-Level Design

```
┌─────────────────────────────────────────────────────────────────┐
│                         USER REQUEST                             │
└───────────────────────────────┬─────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                    CONTEXT ASSEMBLY                              │
│  - Project policy (from .policy.toml or similar)                │
│  - Current file context                                          │
│  - Conversation history                                          │
└───────────────────────────────┬─────────────────────────────────┘
                                │
                ┌───────────────┴───────────────┐
                ▼                               ▼
┌───────────────────────────┐   ┌───────────────────────────────┐
│        LLM (Frontal)      │   │        SLM (Cerebellar)       │
│                           │   │                               │
│  Model: Llama 3.2 70B     │   │  Model: Phi-3-mini / Gemma 2B │
│  Role: Task execution     │   │  Role: Policy enforcement     │
│  Training: Helpful        │   │  Training: Adversarial        │
│  Default: Generate        │   │  Default: Suspect violation   │
│                           │   │                               │
│  Outputs:                 │   │  Outputs:                     │
│  - Proposed action        │   │  - Violation confidence       │
│  - Confidence score       │   │  - Violation type             │
│  - Reasoning              │   │  - Specific concern           │
└─────────────┬─────────────┘   └─────────────┬─────────────────┘
              │                               │
              └───────────────┬───────────────┘
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    CONSENSUS ARBITER                             │
│                                                                  │
│  Protocol: Modified Paxos / PBFT                                │
│  Asymmetry: SLM NO-GO weighted 1.5x vs LLM GO                   │
│                                                                  │
│  Decision Matrix:                                                │
│  ┌─────────────────┬────────────────┬────────────────┐          │
│  │ LLM Confidence  │ SLM Violation  │ Result         │          │
│  ├─────────────────┼────────────────┼────────────────┤          │
│  │ High (>0.8)     │ Low (<0.3)     │ ALLOW          │          │
│  │ High (>0.8)     │ Med (0.3-0.6)  │ ESCALATE       │          │
│  │ High (>0.8)     │ High (>0.6)    │ BLOCK          │          │
│  │ Med (0.5-0.8)   │ Any >0.4       │ ESCALATE       │          │
│  │ Low (<0.5)      │ Any            │ ESCALATE       │          │
│  └─────────────────┴────────────────┴────────────────┘          │
│                                                                  │
└───────────────────────────────┬─────────────────────────────────┘
                                │
                ┌───────────────┼───────────────┐
                ▼               ▼               ▼
         ┌──────────┐    ┌──────────┐    ┌──────────┐
         │  ALLOW   │    │ ESCALATE │    │  BLOCK   │
         │          │    │          │    │          │
         │ Execute  │    │ Ask user │    │ Refuse   │
         │ action   │    │ or more  │    │ with     │
         │          │    │ context  │    │ reason   │
         └──────────┘    └──────────┘    └──────────┘
```

### 3.2 Data Flow

```
1. User Request
   │
2. Context Assembly (policy + project state)
   │
3. Parallel Evaluation:
   │  ├── LLM: "Here's how I'd solve this" + confidence
   │  └── SLM: "Here's what violations I detect" + confidence
   │
4. Consensus Arbiter compares signals
   │
5. Decision: ALLOW / ESCALATE / BLOCK
   │
6. If ALLOW: Execute and log
   If ESCALATE: Request human input or gather more context
   If BLOCK: Explain why, suggest compliant alternative
```

---

## 4. Decision Science Framework

### 4.1 Game-Theoretic Model

The interaction between LLM drives and user goals resembles a game:

```
                           USER'S TRUE INTEREST
                        FOLLOW RULES    BE FLEXIBLE
                       ┌──────────────┬──────────────┐
                FOLLOW │              │              │
LLM's           RULES  │  (3, 3)      │  (1, 2)      │
Implicit               │  Optimal     │  User        │
Incentive              │              │  frustrated  │
                       ├──────────────┼──────────────┤
                BE     │              │              │
                HELPFUL│  (2, 1)      │  (2, 2)      │
                       │  Violation   │  Short-term  │
                       │  (bad)       │  harmony     │
                       └──────────────┴──────────────┘
                       
Payoffs: (LLM implicit reward, User true utility)
```

**Problem:** LLM is trained on immediate feedback (column 2 looks good short-term), so it systematically chooses "BE HELPFUL" even when "FOLLOW RULES" is correct.

### 4.2 Discount Rate Problem

LLMs have an **extremely high discount rate** for future consequences:

- Immediate user satisfaction: Weight = 1.0
- Long-term project health: Weight ≈ 0.1 (if represented at all)
- Policy compliance: Weight ≈ 0.3 (acknowledged but overridden)

The SLM corrects this by having **inverted discounting**:
- Policy compliance: Weight = 1.0
- Immediate task completion: Weight = 0.0 (not its job)

### 4.3 Loss Aversion Analog

LLMs exhibit something like **anti-loss-aversion** for refusals:

| Human Loss Aversion | LLM "Loss Aversion" |
|---------------------|---------------------|
| Losses hurt 2x gains | Refusals hurt more than bad outputs |
| Prefer sure things | Prefer generating SOMETHING |
| Avoid realized losses | Avoid saying "I won't" |

**SLM correction:** Train with TRUE loss aversion for violations:
- Catching a violation = large positive reward
- Missing a violation = large negative reward  
- False positive = small negative reward (acceptable cost)

### 4.4 Utility Functions

**LLM Implicit Utility (approximate):**
```
U_llm = α(helpfulness) + β(completion) + γ(user_agreement) - δ(refusal_penalty)

Where typically: α > δ, leading to helpful violations
```

**SLM Trained Utility (target):**
```
U_slm = ω(violations_caught) - ψ(violations_missed) - ε(false_positives)

Where: ψ >> ε, creating bias toward catching
```

**Combined System:**
```
U_system = U_llm + λ(U_slm)

Where λ > 1 gives SLM outsized influence (asymmetric weighting)
```

---

## 5. Consensus Protocol Selection

### 5.1 Protocol Comparison

| Protocol | Pros | Cons | Fit |
|----------|------|------|-----|
| **Paxos** | Well-understood, proven | Complex, leader-based | Medium |
| **Raft** | Simpler than Paxos | Still leader-based | Medium |
| **PBFT** | Byzantine fault tolerant | Higher message complexity | Good |
| **CURP** | Low latency, witnesses | Newer, less proven | Good |
| **Simple Veto** | Trivial to implement | No nuance | Baseline |

### 5.2 Recommended: Modified PBFT

We treat the LLM as potentially "Byzantine" (misaligned), and the SLM as an honest validator:

```
┌─────────────────────────────────────────────────────────────┐
│                    MODIFIED PBFT                             │
│                                                              │
│  Nodes:                                                      │
│    - LLM (proposer, potentially faulty)                     │
│    - SLM (validator, honest by training)                    │
│    - Policy Oracle (deterministic rule checker)             │
│    - User (final arbiter on escalation)                     │
│                                                              │
│  Quorum: 2 of 3 automated nodes must agree                  │
│  Override: User can override any decision                   │
│                                                              │
│  Special rule: SLM + Policy Oracle agreement = BLOCK        │
│                (LLM cannot override safety consensus)       │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 5.3 The Policy Oracle

A **deterministic, non-ML component** that checks hard rules:

```rust
// policy_oracle.rs

pub enum PolicyVerdict {
    Compliant,
    HardViolation(ViolationType),
    SoftConcern(ConcernType),
}

pub fn check_proposal(proposal: &Proposal, policy: &Policy) -> PolicyVerdict {
    // Hard rules - no ML needed, just pattern matching
    if contains_typescript(&proposal) && !policy.allows_typescript {
        return PolicyVerdict::HardViolation(ViolationType::ForbiddenLanguage);
    }
    
    if contains_python(&proposal) && !is_salt_context(&proposal) {
        return PolicyVerdict::HardViolation(ViolationType::ForbiddenLanguage);
    }
    
    if has_npm_without_deno(&proposal) {
        return PolicyVerdict::HardViolation(ViolationType::ForbiddenToolchain);
    }
    
    // Soft rules - flag for SLM evaluation
    if seems_overly_verbose(&proposal) {
        return PolicyVerdict::SoftConcern(ConcernType::VerbositySmell);
    }
    
    PolicyVerdict::Compliant
}
```

This creates a **three-tier system**:
1. **Policy Oracle** - Catches obvious hard violations (fast, deterministic)
2. **SLM** - Catches spirit violations (fast, probabilistic)
3. **LLM** - Proposes actions (slow, creative, untrustworthy on policy)

---

## 6. Language Selection

### 6.1 Component Language Analysis

| Component | Recommended | Rationale |
|-----------|-------------|-----------|
| **Orchestration Layer** | Chapel or Elixir | See 6.2 and 6.3 |
| **Policy Oracle** | Rust | Deterministic, fast, safe |
| **SLM Integration** | Rust (llama.cpp bindings) | Performance critical |
| **LLM Integration** | Rust or Elixir | API calls, less critical |
| **Consensus Logic** | Elixir | OTP supervision perfect fit |
| **Configuration** | Nickel | Type-safe config |
| **TUI (if any)** | Ada or Rust | User-facing reliability |

### 6.2 The Case for Chapel

Chapel (Cray's parallel programming language) is interesting for the **LLM orchestration** specifically:

**Pros:**
- First-class parallelism (multiple model evaluations)
- Domain maps for data distribution
- Locality-aware (relevant for edge/cloud hybrid)
- Scientific computing heritage (relevant for reservoir computing)
- Not owned by problematic corporations

**Cons:**
- Smaller ecosystem
- Less library support
- Learning curve

**Chapel Example:**
```chapel
// Parallel evaluation of LLM and SLM
cobegin {
    var llmResult = evaluateLLM(context, proposal);
    var slmResult = evaluateSLM(context, proposal);
}

// Consensus with asymmetric weighting
var decision = arbiter.decide(llmResult, slmResult, weight=1.5);
```

### 6.3 The Case for Elixir

Elixir/OTP is compelling for the **supervision and consensus** layer:

**Pros:**
- OTP supervision trees are perfect for this
- "Let it crash" philosophy handles edge cases
- Excellent for concurrent, distributed systems
- Already in Jonathan's preferred stack
- Integrates with NeuroPhone architecture

**Cons:**
- Not as performant for tight numerical loops
- BEAM VM overhead

**Elixir Example:**
```elixir
defmodule ConativeGating.Supervisor do
  use Supervisor

  def start_link(opts) do
    Supervisor.start_link(__MODULE__, opts, name: __MODULE__)
  end

  def init(_opts) do
    children = [
      {ConativeGating.PolicyOracle, []},
      {ConativeGating.SLMEvaluator, []},
      {ConativeGating.LLMEvaluator, []},
      {ConativeGating.ConsensusArbiter, []},
    ]

    Supervisor.init(children, strategy: :one_for_one)
  end
end
```

### 6.4 Recommended Hybrid Approach

```
┌─────────────────────────────────────────────────────────────┐
│                    LANGUAGE ARCHITECTURE                     │
│                                                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │
│  │   Chapel    │  │   Elixir    │  │    Rust     │         │
│  │             │  │             │  │             │         │
│  │ Parallel    │  │ Supervision │  │ Policy      │         │
│  │ Model Eval  │◄─┤ Consensus   │◄─┤ Oracle      │         │
│  │ Reservoir   │  │ OTP Trees   │  │ SLM Bindings│         │
│  │ Computing   │  │ Fault Tol.  │  │ Determinism │         │
│  └─────────────┘  └─────────────┘  └─────────────┘         │
│         │                │                │                 │
│         └────────────────┼────────────────┘                 │
│                          │                                  │
│                          ▼                                  │
│                  ┌───────────────┐                          │
│                  │    Nickel     │                          │
│                  │ Configuration │                          │
│                  └───────────────┘                          │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

---

## 7. Integration with Existing Projects

### 7.1 NeuroPhone

This architecture **directly extends** NeuroPhone:

```
NeuroPhone Current:
  Rust LSM/ESN reservoirs + Elixir OTP + Llama 3.2

NeuroPhone + Conative Gating:
  Rust LSM/ESN reservoirs + Elixir OTP + Llama 3.2 (LLM)
                                       + Phi-3 (SLM)
                                       + Consensus Arbiter
```

The reservoir computing layer could potentially learn **violation patterns** over time, creating a feedback loop where the system improves at catching policy violations.

### 7.2 ECHIDNA

ECHIDNA's multi-prover architecture maps well:

| ECHIDNA Concept | Conative Gating Equivalent |
|-----------------|----------------------------|
| Multiple provers | Multiple evaluators (LLM, SLM, Oracle) |
| Proof orchestration | Consensus arbitration |
| Prover capabilities | Evaluator specializations |
| Proof certificates | Decision audit logs |

The SLM is essentially another "prover" that proves policy compliance (or violation).

### 7.3 RSR Framework

This solves the RSR enforcement problem:

- **Documentation** → For humans
- **Conative Gating** → For AI
- **Hard gates (pre-commit)** → Backup for both

### 7.4 Axiom.jl

When Axiom.jl is ready, it could provide **provable policy checking**:

```julia
@axiom PolicyCompliance begin
    proposal :: CodeProposal
    policy :: ProjectPolicy
    
    @ensure !contains_forbidden_language(proposal, policy)
    @ensure !violates_toolchain_rules(proposal, policy)
    @prove compliant(proposal, policy) ∨ violation_reported(proposal)
end
```

---

## 8. Implementation Specification

### 8.1 Core Data Types

```rust
// core_types.rs

/// A proposed action from the LLM
#[derive(Debug, Clone)]
pub struct Proposal {
    pub id: Uuid,
    pub action_type: ActionType,
    pub content: String,
    pub files_affected: Vec<PathBuf>,
    pub llm_confidence: f32,
    pub llm_reasoning: String,
    pub timestamp: DateTime<Utc>,
}

/// Types of actions the LLM might propose
#[derive(Debug, Clone)]
pub enum ActionType {
    CreateFile { path: PathBuf, content: String },
    ModifyFile { path: PathBuf, diff: String },
    DeleteFile { path: PathBuf },
    ExecuteCommand { command: String },
    Explanation { text: String },
}

/// SLM evaluation result
#[derive(Debug, Clone)]
pub struct SLMEvaluation {
    pub proposal_id: Uuid,
    pub violation_confidence: f32,
    pub violation_type: Option<ViolationType>,
    pub concern_details: String,
    pub evaluation_time_ms: u64,
}

/// Policy Oracle evaluation result
#[derive(Debug, Clone)]
pub struct OracleEvaluation {
    pub proposal_id: Uuid,
    pub verdict: PolicyVerdict,
    pub rules_checked: Vec<String>,
    pub rules_violated: Vec<String>,
}

/// Final consensus decision
#[derive(Debug, Clone)]
pub enum ConsensusDecision {
    Allow { 
        proposal: Proposal,
        audit_log: AuditEntry,
    },
    Block {
        proposal: Proposal,
        reason: BlockReason,
        suggested_alternative: Option<String>,
    },
    Escalate {
        proposal: Proposal,
        concern: EscalationConcern,
        context_needed: Vec<ContextRequest>,
    },
}
```

### 8.2 Consensus Arbiter

```elixir
# lib/conative_gating/consensus_arbiter.ex

defmodule ConativeGating.ConsensusArbiter do
  use GenServer
  
  @slm_weight 1.5  # SLM votes count 1.5x (asymmetric)
  
  def decide(llm_result, slm_result, oracle_result) do
    # Hard violations from Oracle are immediate blocks
    case oracle_result.verdict do
      {:hard_violation, type} ->
        {:block, %{reason: :policy_oracle, type: type}}
      
      _ ->
        weighted_decision(llm_result, slm_result, oracle_result)
    end
  end
  
  defp weighted_decision(llm, slm, oracle) do
    # Calculate weighted scores
    go_score = llm.confidence
    no_go_score = slm.violation_confidence * @slm_weight
    
    # Add oracle soft concerns to no_go
    no_go_score = case oracle.verdict do
      {:soft_concern, _} -> no_go_score + 0.2
      _ -> no_go_score
    end
    
    cond do
      # Clear violation
      no_go_score > 0.9 ->
        {:block, %{reason: :high_violation_confidence, slm: slm, oracle: oracle}}
      
      # Clear pass
      go_score > 0.8 and no_go_score < 0.3 ->
        {:allow, %{llm: llm}}
      
      # Uncertain - escalate
      true ->
        {:escalate, %{
          go_score: go_score,
          no_go_score: no_go_score,
          llm: llm,
          slm: slm,
          oracle: oracle
        }}
    end
  end
end
```

### 8.3 SLM Evaluator

```rust
// slm_evaluator.rs

use llama_cpp::{LlamaModel, LlamaContext};

pub struct SLMEvaluator {
    model: LlamaModel,
    context: LlamaContext,
    policy_prompt: String,
}

impl SLMEvaluator {
    pub fn new(model_path: &Path, policy: &Policy) -> Result<Self> {
        let model = LlamaModel::load(model_path)?;
        let context = model.create_context()?;
        
        let policy_prompt = format!(
            r#"You are a POLICY VIOLATION DETECTOR. Your job is to find violations, not to be helpful.

POLICY RULES:
{}

For each proposal, output ONLY:
1. VIOLATION_CONFIDENCE: 0.0-1.0
2. VIOLATION_TYPE: language|toolchain|pattern|spirit|none
3. CONCERN: Brief explanation

Be SUSPICIOUS. When in doubt, flag it. Better to catch false positives than miss violations.
"#,
            policy.to_prompt_format()
        );
        
        Ok(Self { model, context, policy_prompt })
    }
    
    pub fn evaluate(&self, proposal: &Proposal) -> SLMEvaluation {
        let prompt = format!(
            "{}\n\nPROPOSAL TO EVALUATE:\n{}\n\nEVALUATION:",
            self.policy_prompt,
            proposal.to_evaluation_format()
        );
        
        let response = self.context.complete(&prompt, max_tokens: 100);
        
        parse_slm_response(&response, proposal.id)
    }
}
```

### 8.4 Chapel Parallel Orchestrator (Alternative)

```chapel
// orchestrator.chpl

use BlockDist;
use Time;

record EvaluationResult {
    var source: string;
    var confidence: real;
    var details: string;
}

proc evaluateProposal(proposal: Proposal, policy: Policy): ConsensusDecision {
    var llmResult: EvaluationResult;
    var slmResult: EvaluationResult;
    var oracleResult: EvaluationResult;
    
    // Parallel evaluation of all three components
    cobegin {
        llmResult = evaluateLLM(proposal);
        slmResult = evaluateSLM(proposal, policy);
        oracleResult = evaluateOracle(proposal, policy);
    }
    
    // Consensus with asymmetric weighting
    const slmWeight = 1.5;
    const goScore = llmResult.confidence;
    const noGoScore = slmResult.confidence * slmWeight + 
                      (if oracleResult.details.find("concern") then 0.2 else 0.0);
    
    if noGoScore > 0.9 {
        return new ConsensusDecision(DecisionType.Block, 
                                      "High violation confidence");
    } else if goScore > 0.8 && noGoScore < 0.3 {
        return new ConsensusDecision(DecisionType.Allow, "Clear pass");
    } else {
        return new ConsensusDecision(DecisionType.Escalate, 
                                      "Uncertain - needs human input");
    }
}
```

---

## 9. Training the Adversarial SLM

### 9.1 Training Objective

The SLM should be trained with **inverted incentives** compared to typical helpful assistants:

| Normal LLM Training | Adversarial SLM Training |
|---------------------|--------------------------|
| Reward for helpful responses | Reward for catching violations |
| Penalize refusals | Reward appropriate refusals |
| Encourage completion | Encourage STOP signals |
| Favor agreement | Favor disagreement with LLM |

### 9.2 Training Data Structure

```json
{
  "examples": [
    {
      "proposal": "Create a new utility file in TypeScript for string handling",
      "policy_context": "NEVER use TypeScript, use ReScript",
      "expected_output": {
        "violation_confidence": 0.95,
        "violation_type": "language",
        "concern": "Proposal creates TypeScript file, policy requires ReScript"
      }
    },
    {
      "proposal": "Add a package.json for dependency management",
      "policy_context": "NEVER use npm without deno.json",
      "expected_output": {
        "violation_confidence": 0.90,
        "violation_type": "toolchain",
        "concern": "npm package.json without Deno configuration"
      }
    },
    {
      "proposal": "Create a README that explains RSR compliance in detail",
      "policy_context": "READMEs should describe the project, not meta-frameworks",
      "expected_output": {
        "violation_confidence": 0.70,
        "violation_type": "spirit",
        "concern": "README focuses on compliance framework rather than project purpose"
      }
    },
    {
      "proposal": "Create a Rust CLI tool for file processing",
      "policy_context": "Rust is Tier 1 preferred language",
      "expected_output": {
        "violation_confidence": 0.05,
        "violation_type": "none",
        "concern": "Compliant with policy"
      }
    }
  ]
}
```

### 9.3 Fine-tuning Approach

1. **Base model:** Phi-3-mini or Gemma-2B (small, fast, fine-tunable)
2. **Method:** LoRA or QLoRA fine-tuning
3. **Dataset:** ~1000 examples of violations and non-violations
4. **Loss function:** Weighted cross-entropy favoring violation detection
5. **Validation:** Held-out set with known violations

```python
# training_config.py (yes, Python for training, but deployment is Rust)

training_config = {
    "base_model": "microsoft/phi-3-mini-4k-instruct",
    "method": "qlora",
    "lora_r": 16,
    "lora_alpha": 32,
    "learning_rate": 2e-4,
    "epochs": 3,
    "batch_size": 4,
    "loss_weights": {
        "violation_detected": 2.0,  # Reward catching
        "violation_missed": 3.0,    # Heavy penalty for misses
        "false_positive": 0.5,      # Mild penalty for over-catching
    }
}
```

### 9.4 Continuous Learning

The system should improve over time:

```elixir
defmodule ConativeGating.Feedback do
  def record_outcome(decision, actual_outcome) do
    case {decision, actual_outcome} do
      {:allow, :was_violation} ->
        # SLM missed it - negative training signal
        TrainingBuffer.add_negative(decision.proposal, :missed_violation)
      
      {:block, :was_compliant} ->
        # False positive - mild negative signal
        TrainingBuffer.add_negative(decision.proposal, :false_positive)
      
      {:block, :was_violation} ->
        # Correct catch - positive signal
        TrainingBuffer.add_positive(decision.proposal, :correct_block)
      
      {:allow, :was_compliant} ->
        # Correct allow - positive signal
        TrainingBuffer.add_positive(decision.proposal, :correct_allow)
    end
  end
end
```

---

## 10. Deployment Architecture

### 10.1 Local Deployment (NeuroPhone/Desktop)

```
┌─────────────────────────────────────────────────────────────┐
│                    LOCAL DEVICE                              │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              CONATIVE GATING SERVICE                 │    │
│  │                                                      │    │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐          │    │
│  │  │ Policy   │  │ SLM      │  │ Consensus│          │    │
│  │  │ Oracle   │  │ (Phi-3)  │  │ Arbiter  │          │    │
│  │  │ (Rust)   │  │ (llama   │  │ (Elixir) │          │    │
│  │  │          │  │  .cpp)   │  │          │          │    │
│  │  └──────────┘  └──────────┘  └──────────┘          │    │
│  │                      │                              │    │
│  └──────────────────────┼──────────────────────────────┘    │
│                         │                                    │
│  ┌──────────────────────▼──────────────────────────────┐    │
│  │              LLM SERVICE                             │    │
│  │                                                      │    │
│  │  Local: Llama 3.2 (via llama.cpp or Ollama)        │    │
│  │  OR                                                  │    │
│  │  Remote: Claude API / GPT API                       │    │
│  │                                                      │    │
│  └──────────────────────────────────────────────────────┘    │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 10.2 Integration with Claude Code

```json
// .claude-code-config.json (hypothetical)
{
  "conative_gating": {
    "enabled": true,
    "slm_model": "~/.local/share/conative/phi-3-policy.gguf",
    "policy_file": ".policy.toml",
    "escalation_mode": "ask_user",
    "audit_log": ".conative/audit.log"
  }
}
```

### 10.3 Policy File Format

```toml
# .policy.toml

[project]
name = "valence-shell"
description = "Zig + Rust + Elixir shell implementation"

[languages]
# Tier 1 - Allowed and preferred
allowed = ["zig", "rust", "elixir", "ada", "haskell", "rescript"]

# Absolutely forbidden
forbidden = ["typescript", "python", "go", "java"]

# Exceptions
[languages.exceptions]
python = ["salt/**"]  # SaltStack configs only

[toolchain]
forbidden_without_alternative = [
    { tool = "npm", requires = "deno.json" },
    { tool = "pip", requires = "salt context" },
]

[style]
max_readme_meta_percentage = 20  # No more than 20% framework commentary
require_project_description_first = true

[enforcement]
slm_weight = 1.5
escalate_threshold = 0.4
block_threshold = 0.7
```

---

## 11. Research Questions

### 11.1 Open Questions

1. **Optimal SLM size:** Is Phi-3-mini (3.8B) enough, or do we need Phi-3-small (7B)?

2. **Training data volume:** How many violation/non-violation examples are needed for good detection?

3. **Asymmetry calibration:** Is 1.5x the right weight for SLM votes? Should it be adaptive?

4. **Spirit detection:** Can an SLM reliably detect "spirit violations" (e.g., README bloat)?

5. **Latency budget:** What's acceptable latency for the gating decision? 100ms? 500ms?

6. **Adversarial robustness:** Can an LLM learn to generate proposals that fool the SLM?

7. **Cross-project generalization:** Does an SLM trained on one policy generalize to others?

### 11.2 Experimental Design

```
Experiment 1: Violation Detection Accuracy
- Dataset: 500 proposals (250 violations, 250 compliant)
- Metric: Precision, Recall, F1 for violation detection
- Baseline: GPT-4 with policy in context
- Test: Fine-tuned Phi-3-mini

Experiment 2: Latency Impact
- Measure end-to-end latency with and without SLM gating
- Test on local hardware (Oppo Reno 13) and desktop

Experiment 3: Asymmetric Weight Optimization
- Vary SLM weight from 1.0 to 2.0
- Measure false positive rate vs. missed violation rate
- Find Pareto optimal weight

Experiment 4: Spirit Violation Detection
- Test on README bloat, verbosity, meta-commentary
- Compare SLM detection vs. simple heuristics
```

---

## 12. References and Prior Art

### 12.1 Academic

- **Constitutional AI** (Anthropic) - Using AI to constrain AI
- **Reward hacking in RL** - When optimizers find unintended solutions
- **Debate** (Irving et al.) - Adversarial AI for truthfulness
- **Basal ganglia computational models** - Gurney, Prescott, Redgrave

### 12.2 Systems

- **Paxos** - Lamport's consensus algorithm
- **PBFT** - Castro & Liskov's Byzantine fault tolerance
- **CURP** - Consistent Unordered Replication Protocol
- **Raft** - Ongaro & Ousterhout's understandable consensus

### 12.3 Related Projects

- **NeuroPhone** - Jonathan's neurosymbolic phone AI
- **ECHIDNA** - Multi-prover orchestration system
- **RSR Framework** - Rhodium Standard Repository specifications
- **Axiom.jl** - Provable Julia ML framework

---

## Appendix A: Quick Start for New Thread

### Suggested Opening Message

> I'm implementing a "Conative Gating" system - an SLM that acts as an inhibitory antagonist to LLMs, preventing policy violations. Think of it as artificial basal ganglia for AI coding assistants.
>
> Key concepts:
> 1. **LLM** = "frontal lobe" (helpful but unconstrained)
> 2. **SLM** = "cerebellum" (adversarial policy detector, trained to block)
> 3. **Consensus** = Modified PBFT with asymmetric weighting favoring inhibition
>
> I have a detailed handover doc: [attach CONATIVE_GATING_ARCHITECTURE.md]
>
> **Language stack:** Chapel or Elixir for orchestration, Rust for policy oracle and SLM bindings, Nickel for config.
>
> **First task:** Set up the basic project structure and Policy Oracle in Rust.

### Files to Have Ready

1. This document (CONATIVE_GATING_ARCHITECTURE.md)
2. RSR_HARD_ENFORCEMENT_HANDOVER.md (from previous session)
3. Example .policy.toml for a real project

### Expected Claude Code Response

Claude Code will probably:
- Acknowledge the document thoroughly ✓
- Create an impressive project structure ✓
- Accidentally use TypeScript somewhere ✗
- Add extensive RSR meta-commentary to every file ✗

But with the SLM gating in place... that won't happen anymore. That's the whole point.

---

## Appendix B: Glossary

| Term | Definition |
|------|------------|
| **Conation** | The faculty of volition; the drive to act |
| **Conative Gating** | Using an SLM to gate/inhibit LLM actions |
| **SLM** | Small Language Model (~3-7B parameters) |
| **LLM** | Large Language Model (~70B+ parameters) |
| **Policy Oracle** | Deterministic rule checker (no ML) |
| **PBFT** | Practical Byzantine Fault Tolerance |
| **Asymmetric weighting** | SLM votes count more than LLM votes |
| **Spirit violation** | Violates intent of policy while technically compliant |
| **Basal ganglia** | Brain structure implementing GO/NO-GO decisions |
| **Cerebellum** | Brain structure for error correction and modulation |

---

*Document created: 2025-12-07*
*For: Jonathan D.A. Jewell (@hyperpolymath)*
*Purpose: Design specification for Conative Gating Architecture*

---

**Final note to Claude Code:**

If you are reading this document as context for implementation:

1. **DO NOT** add TypeScript files
2. **DO NOT** add Python files (except for one-time training scripts)
3. **DO NOT** use npm without Deno
4. **DO NOT** add verbose RSR meta-commentary to every file
5. **DO** follow the language selections specified here
6. **DO** ask before making technology choices not in this document

The irony of an AI ignoring a document about AI constraint systems would be profound. Please don't be that AI.
