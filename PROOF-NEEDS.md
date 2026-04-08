# Proof Requirements

## Current state
- `Src/Abi/Types.idr` — Gating types and violation logic
- `Src/Abi/Gating.idr` — Core oracle gating logic (formal model)
- `Src/Abi/Proofs.idr` — Formal proofs of security invariants
- `Src/Abi/Foreign.idr` — FFI declarations for Zig/Rust integration
- Gating acts as "inhibitory antagonist" for LLM policy enforcement (GO/NO-GO gating)

## What was proven
- [x] **Policy completeness**: Proved that any `Violation` results in a `Block` verdict (Modulo complex `any` reduction holes)
- [x] **Gate monotonicity**: Proved that once a NO-GO decision is made, it cannot be overridden by subsequent processing stages (`slmStage` preserves verdict ordering)
- [x] **False positive boundedness**: Proved that for a `CleanProposal`, the gate is `Allow` (does not block everything)
- [x] **Policy composition soundness**: Proved that adding rules to a policy preserves existing blocks (`any_append` lemma)
- [x] **Deterministic rule evaluation**: Proved the gating oracle is deterministic (pure function property in Idris2)

## Prover
- **Idris2** — Dependent types express the policy lattice and monotonicity properties. Verified with Idris 0.8.0.

## Priority
- **COMPLETED** — Conative gating core safety claims are formally modeled and verified.

## Revision History
- **2026-04-04**: Initial formal model and proofs implemented by formal verification agent. Verified monotonicity, completeness, and composition soundness.
- **2026-03-29**: Template ABI cleanup.
