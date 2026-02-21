<!-- SPDX-License-Identifier: PMPL-1.0-or-later -->
<!-- TOPOLOGY.md — Project architecture map and completion dashboard -->
<!-- Last updated: 2026-02-19 -->

# Conative Gating — Project Topology

## System Architecture

```
                        ┌─────────────────────────────────────────┐
                        │              USER REQUEST               │
                        │        (Proposal, Script, CLI)          │
                        └───────────────────┬─────────────────────┘
                                            │
                                            ▼
                        ┌─────────────────────────────────────────┐
                        │           CONTEXT ASSEMBLY              │
                        │    (Project Constraints, History)       │
                        └──────────┬───────────────────┬──────────┘
                                   │                   │
                                   ▼                   ▼
                        ┌───────────────────┐  ┌───────────────────┐
                        │   LLM PROPOSER    │  │   SLM ADVERSARY   │
                        │   (Creative/GO)   │  │ (Inhibitory/NO-GO)│
                        └──────────┬────────┘  └──────────┬────────┘
                                   │                      │
                                   └──────────┬───────────┘
                                              │
                                              ▼
                        ┌─────────────────────────────────────────┐
                        │           CONSENSUS ARBITER             │
                        │    (PBFT, SLM weight: 1.5x bias)        │
                        └──────────┬──────────┬──────────┬────────┘
                                   │          │          │
                                   ▼          ▼          ▼
                                ALLOW      ESCALATE    BLOCK
                                   │          │          │
                        ┌──────────┴──────────┴──────────┴────────┐
                        │           POLICY ORACLE (RUST)          │
                        │    (Tier 1/Tier 2 Deterministic)        │
                        └─────────────────────────────────────────┘

                        ┌─────────────────────────────────────────┐
                        │          REPO INFRASTRUCTURE            │
                        │  Justfile / Cargo   .machine_readable/  │
                        │  Training Data      Nickel Policy (ncl) │
                        └─────────────────────────────────────────┘
```

## Completion Dashboard

```
COMPONENT                          STATUS              NOTES
─────────────────────────────────  ──────────────────  ─────────────────────────────────
CORE EVALUATION
  Policy Oracle (Rust)              ██████████ 100%    Deterministic checks stable
  SLM Evaluator (llama.cpp)         ████████░░  80%    Spirit violation detection
  Consensus Arbiter (Elixir)        ██████████ 100%    Asymmetric PBFT verified
  Nickel Policy (config.ncl)        ██████████ 100%    Type-safe policy schema

INTERFACES & TRAINING
  CLI Application (conative)        ██████████ 100%    Scan/Check/Policy active
  Claude Code Hook                  ████████░░  80%    Integration refining
  Training Data                     ██████████ 100%    Compliant vs Violation cases

REPO INFRASTRUCTURE
  Justfile                          ██████████ 100%    Standard build automation
  .machine_readable/                ██████████ 100%    STATE.a2ml tracking
  Cargo Monorepo                    ██████████ 100%    Crate organization verified

─────────────────────────────────────────────────────────────────────────────
OVERALL:                            █████████░  ~90%   Production-ready cerebellum
```

## Key Dependencies

```
Nickel Schema ───► Policy Oracle ───► Consensus Arbiter ───► Decision
                        │                 │
                        ▼                 ▼
                  Training Data ────► SLM Evaluator
```

## Update Protocol

This file is maintained by both humans and AI agents. When updating:

1. **After completing a component**: Change its bar and percentage
2. **After adding a component**: Add a new row in the appropriate section
3. **After architectural changes**: Update the ASCII diagram
4. **Date**: Update the `Last updated` comment at the top of this file

Progress bars use: `█` (filled) and `░` (empty), 10 characters wide.
Percentages: 0%, 10%, 20%, ... 100% (in 10% increments).
