# Proof Requirements

## Current state
- `src/abi/Types.idr` — Gating types
- `src/abi/Layout.idr` — Memory layout
- `src/abi/Foreign.idr` — FFI declarations
- No dangerous patterns in ABI layer
- Claims: SLM acts as "inhibitory antagonist" for LLM policy enforcement (GO/NO-GO gating)

## What needs proving
- **Policy completeness**: Prove that the NO-GO gate blocks ALL policy-violating outputs (no bypass path exists)
- **Gate monotonicity**: Prove that once a NO-GO decision is made, it cannot be overridden by subsequent processing stages
- **False positive boundedness**: Prove that the gating function's false-positive rate is bounded (does not degenerate to blocking everything)
- **Policy composition soundness**: Prove that combining multiple policy rules preserves individual rule guarantees (no rule cancellation)
- **Deterministic rule evaluation**: Prove the Rust policy oracle produces identical results for identical inputs (no hidden state)

## Recommended prover
- **Idris2** — Dependent types can express the policy lattice and monotonicity properties
- **Lean4** — Good for the algebraic properties of policy composition if modeled as a semilattice

## Priority
- **HIGH** — Conative gating is a security/safety mechanism for LLM outputs. If the gate can be bypassed, the entire safety claim collapses. Policy completeness and gate monotonicity are critical.

## Template ABI Cleanup (2026-03-29)

Template ABI removed -- was creating false impression of formal verification.
The removed files (Types.idr, Layout.idr, Foreign.idr) contained only RSR template
scaffolding with unresolved {{PROJECT}}/{{AUTHOR}} placeholders and no domain-specific proofs.
