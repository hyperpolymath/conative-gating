# Conative Gating ABI/FFI Documentation

## Overview

This library follows the **Hyperpolymath RSR Standard** for ABI and FFI design:

- **ABI (Application Binary Interface)** defined in **Idris2** with formal proofs
- **FFI (Foreign Function Interface)** implemented in **Zig** for C compatibility
- **Generated C headers** bridge Idris2 ABI to Zig FFI
- **Any language** can call through standard C ABI

## Architecture

```
┌─────────────────────────────────────────────┐
│  ABI Definitions (Idris2)                   │
│  Src/Abi/                                   │
│  - Types.idr      (Type definitions)        │
│  - Gating.idr     (Core logic model)        │
│  - Proofs.idr     (Formal verification)     │
│  - Foreign.idr    (FFI declarations)        │
└─────────────────┬───────────────────────────┘
```

## Directory Structure

```
conative-gating/
├── Src/
│   ├── Abi/                    # ABI definitions (Idris2)
│   │   ├── Types.idr           # Core type definitions with proofs
│   │   ├── Gating.idr          # Formal gating model
│   │   ├── Proofs.idr          # Formal security proofs
│   │   └── Foreign.idr         # FFI function declarations
│   └── lib/                    # Core library (any language)
```

## Formally Verified Invariants

- **Policy completeness**: NO-GO gate blocks ALL policy-violating outputs.
- **Gate monotonicity**: Once a NO-GO decision is made, it cannot be overridden.
- **Deterministic rule evaluation**: Identical inputs produce identical results.
- **False positive boundedness**: The gate does not block clean proposals.

## Building

```bash
# Verify ABI and proofs
idris2 --check Src/Abi/Proofs.idr
```
