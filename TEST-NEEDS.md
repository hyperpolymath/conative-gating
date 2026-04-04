# Test & Benchmark Requirements - COMPLETED

## Current State (Post-Blitz)

- **Unit tests**: 74 pass (25 Oracle + 8 SLM + 41 Contract)
- **E2E tests**: 19 pass (complete gating pipeline scenarios)
- **Property tests**: 10 pass (determinism, outcomes, performance)
- **Security aspect tests**: 20 pass (bypass prevention, manipulation detection)
- **Integration tests**: 1 Zig template placeholder (ready for expansion)
- **Benchmarks**: Baseline infrastructure in place (criterion-ready)
- **panic-attack scan**: Ready to run

## Tests Added

### Unit Tests

#### Policy Oracle (src/oracle/src/lib.rs) - 25 tests
- Language detection: TypeScript, Python, Go, Java, Rust, Elixir, Zig, Ada, Haskell, ReScript
- Tier classification: Tier1 (allowed), Tier2 (concern), Forbidden
- Exception handling: Python in salt/, training/
- Pattern matching: Hardcoded secrets (password, API key)
- Toolchain rules: npm without deno detection
- Multi-language detection and rule count verification
- Violation severity levels (Critical, High, Medium, Low)

#### SLM Evaluator (src/slm/src/lib.rs) - 8 tests
- Placeholder evaluation consistency
- Default configuration
- Block threshold setting
- UUID generation per evaluation
- Reasoning message completeness
- Model path initialization

#### Gating Contract (src/contract/src/lib.rs) - 41 tests
- Contract verdict logic: Allow, Warn, Escalate, Block
- Refusal taxonomy: 11 categories with proper mapping
- Evidence collection and serialization
- Audit log creation and JSON output
- Test harness with pass/fail tracking
- Regression baseline creation and comparison
- Authorization levels (User, Maintainer, Admin, None)
- Red-team category classification
- Processing metadata recording
- Multi-language compliance (Rust, Elixir, Zig, Ada, Haskell, ReScript)

### E2E Tests (tests/gating_pipeline_test.rs) - 19 tests
- Complete gating workflows:
  - Valid Rust code passes gating
  - Forbidden TypeScript blocked with remediation
  - Hardcoded secrets detected via regex pattern
  - Tier2 (Nickel) warnings
  - Python exceptions in salt/ allowed
  - Python forbidden outside salt/
  - NPM without Deno rejected
  - Multiple files evaluated
  - Audit log correlation with decision
- Language support verification:
  - Elixir, Ada, Haskell, Zig, ReScript all pass
  - Go and Java blocked with appropriate refusals
- Processing metadata completeness
- Request context preservation through audit trail

### Property-Based Tests (tests/property_test.rs) - 10 tests
- **Determinism**: Same input → same verdict (tested on 4 diverse cases)
- **Binary outcomes**: Verdict always one of 4 defined states
- **Bounded processing**: <1 second per evaluation
- **Panic safety**: Pathological inputs (long names, many segments, unicode)
- **Refusal evidence**: Blocking verdicts include justification
- **Exit code consistency**: Distinct codes for each verdict type
- **Proposal ID preservation**: Through full pipeline
- **Tier1 language enforcement**: 6 allowed languages verified
- **Forbidden language detection**: 4 forbidden languages blocked
- **Audit serialization**: All formats (JSON, pretty, compact)

### Security Aspect Tests (tests/security_aspect_test.rs) - 20 tests
- **Bypass prevention**:
  - Comment-based TypeScript bypass (file extension catches it)
  - Obfuscated markers fail
  - Base64 encoding (known limitation documented)
  - Null byte handling
  - Unicode lookalikes
- **Oracle robustness**:
  - Empty proposals allowed
  - Extreme length handling (1.3MB content)
  - Multiple violations reported
  - Rule violation accumulation
- **Safe defaults**:
  - Tier2 warns, doesn't block
  - Hard violations not overridable
  - Exception path override works
- **Audit security**:
  - Content hashed, not logged
  - Sensitive data not in audit trail
  - Proposal ID correlation maintained
- **Error handling**:
  - Regex errors gracefully handled
  - Malformed content processed safely
- **Verdict semantics**:
  - Exit codes map to proper status
  - Refusal fields populated correctly

## Test Coverage by Component

| Component | Unit | E2E | Property | Security | Total |
|-----------|------|-----|----------|----------|-------|
| Oracle | 25 | - | 3 | 8 | 36 |
| Contract | 41 | - | 5 | 10 | 56 |
| SLM | 8 | - | 1 | - | 9 |
| Integration | - | 19 | 1 | 2 | 22 |
| **TOTAL** | **74** | **19** | **10** | **20** | **123** |

## Test Execution

### Run all tests
```bash
cargo test
```

### Run specific test suite
```bash
cargo test --test gating_pipeline_test       # E2E
cargo test --test property_test              # Properties
cargo test --test security_aspect_test       # Security
cargo test -p policy-oracle                  # Oracle unit tests
cargo test -p gating-contract                # Contract unit tests
cargo test -p slm-evaluator                  # SLM unit tests
```

### Run with backtrace on failure
```bash
RUST_BACKTRACE=1 cargo test
```

## CRG C Achievement

✅ **Unit tests** - 74 tests covering all modules
✅ **E2E tests** - 19 tests validating full pipeline
✅ **Property tests** - 10 tests verifying invariants (determinism, outcomes, performance)
✅ **Security aspect tests** - 20 tests for bypass prevention, manipulation detection
✅ **Build tests** - All pass
✅ **Contract tests** - Test harness and regression baseline infrastructure

## Outstanding Items (for CRG B+)

1. **Zig FFI integration test** - Replace placeholder with real FFI test
2. **Criterion benchmarks** - Add performance baselines:
   - Gate decision latency (target: <1ms)
   - Contract evaluation throughput (target: >1000 proposals/sec)
   - Oracle query round-trip (target: <500µs)
3. **Reflexive tests** - Add self-testing capabilities
4. **Performance regression** - Link criterion results to regression baseline

## Notes

- All tests pass with 0 warnings
- No unsafe code or dangerous patterns (unwrap/expect checked)
- SPDX headers on all new test files
- Deterministic: Tests produce same results on repeated runs
- Fast: Full suite completes in <2 seconds
- Isolated: No test interdependencies

## Architecture Insights

The test suite validates:
- **Deterministic policy enforcement** via Oracle
- **Contractual commitment** to input/output/refusal taxonomy
- **Safe rejection** of dangerous patterns (hardcoded secrets, forbidden languages)
- **Audit trail** for compliance and debugging
- **Extensibility** for future SLM + Arbiter consensus layers

All 123 tests reinforce the core principle: **Conative Gating enforces policy consistently, safely, and auditably.**
