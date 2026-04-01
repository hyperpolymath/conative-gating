# Test & Benchmark Requirements

## Current State
- Unit tests: 0 pass / 0 fail (cargo test runs but finds 0 tests)
- Integration tests: 1 Zig integration test (template)
- E2E tests: NONE
- Benchmarks: NONE
- panic-attack scan: NEVER RUN

## What's Missing
### Point-to-Point (P2P)
- src/main.rs — no tests
- src/contract/src/lib.rs — no tests
- src/oracle/src/lib.rs — no tests
- src/slm/src/lib.rs — no tests
- 2 Elixir source files — no tests
- 3 Idris2 ABI files — no verification tests
- 3 Zig FFI files — only template test

### End-to-End (E2E)
- Contract evaluation workflow
- Oracle query/response cycle
- SLM (presumably Small Language Model) interaction
- Gating decision pipeline (input -> evaluate -> gate/pass)

### Aspect Tests
- [ ] Security (gating bypass, oracle manipulation, contract exploitation)
- [ ] Performance (gating decision latency)
- [ ] Concurrency (concurrent gate evaluations)
- [ ] Error handling (oracle unavailability, malformed contracts)
- [ ] Accessibility (N/A)

### Build & Execution
- [x] cargo build — compiles (with 0 tests)
- [x] cargo test — passes (0 tests found)
- [ ] Binary runs and exits cleanly — not verified
- [ ] CLI --help works — not verified
- [ ] Self-diagnostic — none

### Benchmarks Needed
- Gating decision latency
- Contract evaluation throughput
- Oracle round-trip time

### Self-Tests
- [ ] panic-attack assail on own repo
- [ ] Built-in doctor/check command (if applicable)

## Priority
- **HIGH** — 5 Rust source files + 2 Elixir + Zig/Idris2 with ZERO tests. Cargo test finds literally nothing. This is a complete test vacuum for a component that makes security-relevant gating decisions.
