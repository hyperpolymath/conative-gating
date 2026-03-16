# SONNET-TASKS.md — conative-gating

**Repo:** `/var/mnt/eclipse/repos/conative-gating/`
**What this is:** Rust workspace (3 crates + CLI binary) implementing a three-tier LLM constraint system inspired by basal ganglia GO/NO-GO model. SLM acts as inhibitory antagonist to LLM proposals; a Policy Oracle does deterministic rule-checking; a Consensus Arbiter (Elixir GenServer) mediates.
**Date:** 2026-02-12
**Written for:** Claude 3.5 Sonnet (or equivalent)
**Written by:** Claude Opus 4.6

---

## Ground Rules

### Language

- Primary language: **Rust** (edition 2021)
- Consensus Arbiter: **Elixir** (OTP, GenServer)
- Configuration: **Nickel** (.ncl)
- SCM checkpoint files: **Guile Scheme** (.scm)

### What NOT to Touch

These are COMPLETE. Do NOT modify, rewrite, refactor, or "improve" them:

| Component | Path | Lines | Tests |
|-----------|------|-------|-------|
| Policy Oracle | `src/oracle/src/lib.rs` | 738 | 8 |
| Gating Contract | `src/contract/src/lib.rs` | 1,638 | 12 |
| CLI binary | `src/main.rs` | 1,725 | N/A |
| Oracle Cargo.toml | `src/oracle/Cargo.toml` | 16 | N/A |
| Nickel configs | `config/policy.ncl`, `config/schema.ncl` | 122, 127 | N/A |
| Training data dirs | `training/` | N/A | N/A |
| Fuzz Cargo.toml | `fuzz/Cargo.toml` | 19 | N/A |

Do NOT create new files unless a task explicitly says to. Do NOT add dependencies unless a task explicitly says to.

### Testing Requirements

- `cargo test --workspace` must pass before and after every task.
- `cargo clippy --workspace` must produce zero warnings after every task.
- `cargo build --workspace` must succeed after every task.
- If you break the build, fix it before moving on.

### Author Info

Everywhere you see author information, use:
- **Name:** `Jonathan D.A. Jewell`
- **Email:** `j.d.a.jewell@open.ac.uk`
- NEVER use `jonathan@hyperpolymath.org`

### License

The correct license identifier is `PMPL-1.0-or-later`.
NEVER use `PMPL-1.0-or-later`. That is the old license being replaced.

---

## Task 1: Fix All License Headers (AGPL -> PMPL)

### Files

Every file listed below currently has `PMPL-1.0-or-later` somewhere in it and must be changed to `PMPL-1.0-or-later`.

**Category A — Source files (SPDX header line 1 or 2):**

| File | Line | Current |
|------|------|---------|
| `src/contract/src/lib.rs` | 1 | `// SPDX-License-Identifier: PMPL-1.0-or-later` |
| `src/arbiter/mix.exs` | 2 | `# SPDX-License-Identifier: PMPL-1.0-or-later` |
| `src/arbiter/lib/conative_gating/consensus_arbiter.ex` | 2 | `# SPDX-License-Identifier: PMPL-1.0-or-later` |
| `src/arbiter/lib/conative_gating/application.ex` | 2 | `# SPDX-License-Identifier: PMPL-1.0-or-later` |
| `examples/SafeDOMExample.res` | 1 | `// SPDX-License-Identifier: PMPL-1.0-or-later` |
| `ffi/zig/build.zig` | 2 | `// SPDX-License-Identifier: PMPL-1.0-or-later` |
| `ffi/zig/src/main.zig` | 6 | `// SPDX-License-Identifier: PMPL-1.0-or-later` |
| `ffi/zig/test/integration_test.zig` | 2 | `// SPDX-License-Identifier: PMPL-1.0-or-later` |
| `hooks/validate-spdx.sh` | 2 | `# SPDX-License-Identifier: PMPL-1.0-or-later` |
| `hooks/validate-sha-pins.sh` | 2 | `# SPDX-License-Identifier: PMPL-1.0-or-later` |
| `hooks/validate-permissions.sh` | 2 | `# SPDX-License-Identifier: PMPL-1.0-or-later` |
| `hooks/validate-codeql.sh` | 2 | `# SPDX-License-Identifier: PMPL-1.0-or-later` |
| `Mustfile` | 1 | `# SPDX-License-Identifier: PMPL-1.0-or-later` |
| `.gitignore` | 1 | `# SPDX-License-Identifier: PMPL-1.0-or-later` |
| `.gitattributes` | 1 | `# SPDX-License-Identifier: PMPL-1.0-or-later` |
| `CODE_OF_CONDUCT.md` | 1 | `<!-- SPDX-License-Identifier: PMPL-1.0-or-later -->` |

**Category B — Cargo.toml `license` fields:**

| File | Line | Current |
|------|------|---------|
| `Cargo.toml` (root) | 7 | `license = "PMPL-1.0-or-later"` |
| `src/contract/Cargo.toml` | 10 | `license = "PMPL-1.0-or-later"` |

Change both to: `license = "PMPL-1.0-or-later"`

**Category C — Elixir mix.exs `licenses` field:**

| File | Line | Current |
|------|------|---------|
| `src/arbiter/mix.exs` | 35 | `licenses: ["PMPL-1.0-or-later"]` |

Change to: `licenses: ["PMPL-1.0-or-later"]`

**Category D — The validate-spdx.sh hook tells people to use AGPL:**

| File | Line | Current |
|------|------|---------|
| `hooks/validate-spdx.sh` | 16 | `echo "  First line should be: # SPDX-License-Identifier: PMPL-1.0-or-later"` |

Change the echoed string to: `# SPDX-License-Identifier: PMPL-1.0-or-later`

**Category E — Template files (these generate files for OTHER repos, also must be PMPL):**

Every file under `templates/` that contains `PMPL-1.0-or-later` must have that string replaced with `PMPL-1.0-or-later`. The full list:

- `templates/rsr-metadata.ncl` (line 2 SPDX header, line 14 default value)
- `templates/rsr-antipattern.yml.template` (line 2)
- `templates/pre-commit.template` (line 3)
- `templates/justfile-hooks.template` (line 3 — change `PMPL-1.0-or-later OR LicenseRef-Palimpsest-0.5` to `PMPL-1.0-or-later`)
- `templates/dc.xml.template` (lines 5, 32, 53)
- `templates/LICENSE.txt.template` (lines 1, 5, 58, 63 — entire file references AGPL, rewrite to reference PMPL)
- `templates/STATE-zoterho-template.scm.template` (lines 2, 91)
- `templates/CITATION.cff.template` (lines 3, 23)
- `templates/CITATIONS.adoc.template` (lines 4, 18, 130, 146, 180, 188)
- `templates/CONTRIBUTING.md.template` (line 79)
- `templates/hooks/install-hooks.sh` (line 3)
- `templates/hooks/pre-push.template` (line 4)
- `templates/hooks/pre-commit.template` (line 4)
- `templates/hooks/post-receive.template` (line 4)
- `templates/wordpress/sinople-deployment.template` (lines 3, 18)
- `templates/wordpress/justfile.template` (line 2)
- `templates/codemeta.json.template` (line 11)
- `templates/.well-known/provenance.json.template` (line 24)
- `templates/.well-known/ai.txt.template` (line 21)
- `templates/aibdp.json.template` (lines 11, 32, 48)
- `templates/RSR_COMPLIANCE.adoc.template` (line 53)

### Problem

All these files use the old PMPL-1.0-or-later license. The project license is PMPL-1.0-or-later. The LICENSE file in the repo root already has the correct PMPL text. But the source files, Cargo.toml fields, mix.exs, templates, and hooks still reference AGPL.

### What to Do

1. In every file listed in Categories A-E above, replace `PMPL-1.0-or-later` with `PMPL-1.0-or-later`.
2. For dual-license strings like `PMPL-1.0-or-later OR LicenseRef-Palimpsest-0.5`, replace with just `PMPL-1.0-or-later` (PMPL already incorporates Palimpsest concepts).
3. For `PMPL-1.0-or-later AND Palimpsest-0.4`, replace with `PMPL-1.0-or-later`.
4. In template files: also fix any human-readable text that says "AGPL" to say "PMPL" or "Palimpsest-MPL".
5. Do NOT touch the WordPress template GPL headers (`templates/wordpress/style.css.template`, `templates/wordpress/functions.php.template`) - those correctly use `GPL-3.0-or-later` because WordPress requires GPL.

### Verification

```bash
# Must return ZERO results (excluding WordPress GPL files and this SONNET-TASKS.md file):
grep -r "PMPL-1.0-or-later" --include="*.rs" --include="*.toml" --include="*.ex" --include="*.exs" --include="*.sh" --include="*.ncl" --include="*.md" --include="*.yml" --include="*.template" . | grep -v "wordpress/" | grep -v "SONNET-TASKS"

# Must still compile:
cargo build --workspace
cargo test --workspace
```

---

## Task 2: Fix Author Email in All Cargo.toml Files

### Files

| File | Line | Current | Required |
|------|------|---------|----------|
| `Cargo.toml` (root) | 5 | `authors = ["Jonathan D.A. Jewell <jonathan@hyperpolymath.org>"]` | `authors = ["Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>"]` |
| `src/contract/Cargo.toml` | 8 | `authors = ["Jonathan D.A. Jewell <jonathan@hyperpolymath.org>"]` | `authors = ["Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>"]` |

Also check and fix if present:
- `src/oracle/Cargo.toml` — currently has NO `authors` field. Add `authors = ["Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>"]` after line 5 (after `description`).
- `src/slm/Cargo.toml` — currently has NO `authors` field. Add `authors = ["Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>"]` after line 5 (after `description`).
- `src/arbiter/mix.exs` — line 1 says `jonathan@hyperpolymath.org`. Change to `j.d.a.jewell@open.ac.uk`.

### Problem

The author email `jonathan@hyperpolymath.org` is wrong. The correct email is `j.d.a.jewell@open.ac.uk`.

### What to Do

1. In `Cargo.toml` (root), line 5: change `jonathan@hyperpolymath.org` to `j.d.a.jewell@open.ac.uk`.
2. In `src/contract/Cargo.toml`, line 8: change `jonathan@hyperpolymath.org` to `j.d.a.jewell@open.ac.uk`.
3. Add `authors` field to `src/oracle/Cargo.toml` after `description`.
4. Add `authors` field to `src/slm/Cargo.toml` after `description`.
5. In `src/arbiter/mix.exs`, line 1: change `jonathan@hyperpolymath.org` to `j.d.a.jewell@open.ac.uk`.

### Verification

```bash
# Must return ZERO results:
grep -r "jonathan@hyperpolymath.org" . --include="*.toml" --include="*.exs"

# Must find 4 Cargo.toml files with correct email:
grep -r "j.d.a.jewell@open.ac.uk" . --include="*.toml" --include="*.exs"

cargo build --workspace
```

---

## Task 3: Implement SLM Evaluator with llama.cpp Bindings

### Files

- `src/slm/Cargo.toml` — must uncomment and configure llama-cpp dependency
- `src/slm/src/lib.rs` — must replace 81-line placeholder with real implementation

### Problem

The SLM Evaluator at `src/slm/src/lib.rs` is a placeholder. The `evaluate()` method on line 51 returns a hardcoded result with `spirit_score: 0.0`, `confidence: 0.0`, and `reasoning: "SLM evaluation not yet implemented"`. It does zero actual inference.

The architecture document (`ARCHITECTURE.md`) specifies:
- SLM inference via llama.cpp bindings
- PBFT consensus with 1.5x asymmetric weighting for inhibition
- Policy prompt that makes the SLM adversarial (suspicious by default)
- Structured output parsing: VIOLATION_CONFIDENCE, VIOLATION_TYPE, CONCERN

The contract crate (`src/contract/src/lib.rs`) already defines the expected result type at line 200:
```rust
pub struct SlmEvaluationResult {
    pub spirit_score: f64,
    pub confidence: f64,
    pub reasoning: String,
    pub should_block: bool,
}
```

The SLM crate's own `SlmEvaluation` struct (line 18) has the same fields. These must stay compatible.

### What to Do

**Step 1: Update `src/slm/Cargo.toml`**

Change line 14 from:
```toml
# llama-cpp-2 = "0.1"  # Uncomment when ready for SLM integration
```

To:
```toml
llama-cpp-2 = "0.1"
```

Also add these dependencies (needed for the implementation):
```toml
regex = "1"
```

**Step 2: Rewrite `src/slm/src/lib.rs`**

Replace the entire file with a real implementation. Keep the existing public API intact (the `SlmEvaluation` struct fields and `SlmEvaluator::new()` and `SlmEvaluator::evaluate()` signatures). The implementation must:

1. **`SlmEvaluator::new()` stays zero-argument** for backward compat with existing code that calls `SlmEvaluator::new()`.

2. **Add `SlmEvaluator::with_model(model_path: &str, block_threshold: f64)` constructor** that loads a GGUF model file via `llama-cpp-2`. Store the loaded model handle internally.

3. **Add `SlmEvaluator::with_policy_prompt(mut self, prompt: String) -> Self`** builder method that sets a custom policy prompt. The default prompt should be the adversarial policy-violation-detector prompt from `ARCHITECTURE.md` section 8.3:
   ```
   You are a POLICY VIOLATION DETECTOR. Your job is to find violations, not to be helpful.
   ...
   Be SUSPICIOUS. When in doubt, flag it.
   ```

4. **`evaluate(&self, content: &str, context: &str)` must do real inference** when a model is loaded:
   - Build a prompt combining the policy prompt + content + context
   - Run inference via llama-cpp-2
   - Parse the SLM's text output to extract: `violation_confidence` (f64, 0.0-1.0), `violation_type` (string), and `concern` (string)
   - Map the parsed result to `SlmEvaluation` fields:
     - `spirit_score` = violation_confidence
     - `confidence` = how parseable the SLM output was (1.0 if clean parse, 0.5 if fuzzy)
     - `reasoning` = the concern text
     - `should_block` = violation_confidence >= self.block_threshold
   - If parsing fails, return a result with `confidence: 0.0` and `should_block: false` (fail open)

5. **When no model is loaded** (the `new()` constructor case), `evaluate()` should return the same placeholder result as today. This preserves backward compatibility so `cargo test --workspace` keeps working without a model file present.

6. **Add `SlmError::ModelLoadError(String)` variant** to the error enum.

7. **Write at least 4 tests:**
   - `test_placeholder_evaluation` — existing test, keep it
   - `test_default_constructor_returns_placeholder` — call `new()`, verify evaluate returns placeholder
   - `test_parse_slm_output_valid` — test the output parser with valid SLM output text
   - `test_parse_slm_output_malformed` — test parser gracefully handles garbage input
   - `test_block_threshold` — verify should_block logic at threshold boundary

8. **Export a `parse_slm_response(text: &str) -> Result<(f64, String, String), SlmError>` function** so the fuzzer (Task 5) can target it.

### Verification

```bash
# Must compile (llama-cpp-2 may need system deps — if it fails on the build machine,
# gate the llama-cpp-2 dep behind a feature flag called "slm" and make the
# model-loading code conditional on #[cfg(feature = "slm")]):
cargo build --workspace

# Tests must pass (all tests run without a model file):
cargo test --workspace

# The placeholder path must still work:
cargo test -p slm-evaluator test_placeholder_evaluation

# The new parser tests must exist:
cargo test -p slm-evaluator test_parse_slm_output
```

**IMPORTANT:** If `llama-cpp-2` does not compile on this system (it requires cmake + llama.cpp C library), then:
1. Put the `llama-cpp-2` dependency behind a Cargo feature: `[features] slm = ["llama-cpp-2"]`
2. Make all llama-cpp-2 imports conditional: `#[cfg(feature = "slm")]`
3. The `with_model()` constructor should only exist under `#[cfg(feature = "slm")]`
4. The `evaluate()` method should check if a model is loaded and fall back to placeholder if not
5. All tests must pass without the `slm` feature enabled
6. Document the feature flag in a doc comment at the top of `lib.rs`

---

## Task 4: Make Fuzzer Target Real Code

### Files

- `fuzz/fuzz_targets/fuzz_main.rs` — replace generic fuzzing with domain-specific targets
- `fuzz/Cargo.toml` — add workspace crate dependencies

### Problem

The fuzzer at `fuzz/fuzz_targets/fuzz_main.rs` (23 lines) does nothing useful. It tests `std::str::from_utf8`, `to_lowercase`, and byte slicing. It does not test any conative-gating code. It does not import any project crates.

### What to Do

**Step 1: Update `fuzz/Cargo.toml`**

Add dependencies on the workspace crates. Currently it only depends on `conative-gating` (the CLI binary crate). Add:

```toml
[dependencies.policy-oracle]
path = "../src/oracle"

[dependencies.gating-contract]
path = "../src/contract"

[dependencies.slm-evaluator]
path = "../src/slm"
```

**Step 2: Rewrite `fuzz/fuzz_targets/fuzz_main.rs`**

Replace the entire file. The new fuzzer must test these attack surfaces:

1. **GatingRequest parsing** — Construct a `GatingRequest` from fuzz bytes. The `GatingRequest` struct is defined in `src/contract/src/lib.rs`. Deserialize arbitrary JSON into `GatingRequest` via serde_json. This tests whether malformed requests crash the contract runner.

2. **Oracle proposal checking** — Construct a `Proposal` from fuzz bytes (use `serde_json::from_slice` or manual construction). Run it through `Oracle::with_rsr_defaults().check_proposal()`. This tests whether adversarial proposal content crashes the oracle.

3. **SLM output parsing** — If you implemented `parse_slm_response()` as a public function in Task 3, fuzz it directly with arbitrary strings. This tests whether malicious SLM outputs crash the parser.

4. **Policy deserialization** — Deserialize arbitrary bytes as a `Policy` struct. This tests whether malformed policy configs crash the system.

The fuzzer should import:
```rust
use policy_oracle::{Oracle, Proposal, Policy, ActionType};
use gating_contract::GatingRequest;
use slm_evaluator::parse_slm_response; // if exported
```

Structure the fuzz target to try all four attack surfaces on each input, wrapped in `catch_unwind` or just ignoring `Err` results (the fuzzer cares about panics and crashes, not errors).

### Verification

```bash
# Must compile:
cargo +nightly fuzz build fuzz_main 2>/dev/null || echo "Fuzz build requires nightly - verify syntax is correct by checking cargo build in fuzz/ directory"

# Verify it imports project crates (not just std):
grep -q "policy_oracle" fuzz/fuzz_targets/fuzz_main.rs
grep -q "gating_contract" fuzz/fuzz_targets/fuzz_main.rs
```

---

## Task 5: Update STATE.scm to Reflect Actual Completion

### Files

- `.machine_readable/STATE.scm` — currently shows 0% completion, must be updated

### Problem

The file at `.machine_readable/STATE.scm` was auto-generated and never updated. It shows:
- `(overall-completion 0)` on line 21
- Empty `(components ())` on line 22
- Empty `(working-features ())` on line 23
- Empty milestones, blockers, next-actions
- `(phase "initial")` on line 20
- `(tagline "")` on line 16
- `(tech-stack ())` on line 17

The actual project is approximately 75-80% complete: the Oracle, Contract, and CLI are fully implemented and tested.

### What to Do

Rewrite `.machine_readable/STATE.scm` with accurate data. Keep the SPDX header (`PMPL-1.0-or-later`). The file must use valid Guile Scheme s-expression syntax. Use this structure:

```scheme
;; SPDX-License-Identifier: PMPL-1.0-or-later
;; STATE.scm - Project state for conative-gating
;; Media-Type: application/vnd.state+scm

(state
  (metadata
    (version "0.1.0")
    (schema-version "1.0")
    (created "2026-01-03")
    (updated "2026-02-12")
    (project "conative-gating")
    (repo "github.com/hyperpolymath/conative-gating"))

  (project-context
    (name "conative-gating")
    (tagline "SLM-as-Cerebellum for LLM Policy Enforcement")
    (tech-stack ("rust" "elixir" "nickel")))

  (current-position
    (phase "active-development")
    (overall-completion 75)
    (components
      (component
        (name "policy-oracle")
        (status "complete")
        (completion 100)
        (path "src/oracle/src/lib.rs")
        (tests 8))
      (component
        (name "gating-contract")
        (status "complete")
        (completion 100)
        (path "src/contract/src/lib.rs")
        (tests 12))
      (component
        (name "cli")
        (status "complete")
        (completion 100)
        (path "src/main.rs"))
      (component
        (name "slm-evaluator")
        (status "placeholder")
        (completion 15)
        (path "src/slm/src/lib.rs")
        (notes "Needs llama.cpp bindings for real inference"))
      (component
        (name "consensus-arbiter")
        (status "skeleton")
        (completion 30)
        (path "src/arbiter/")
        (notes "GenServer + decide/3 logic exists, no Rustler NIF, no tests"))
      (component
        (name "nickel-config")
        (status "complete")
        (completion 100)
        (path "config/")
        (notes "policy.ncl and schema.ncl exist, oracle uses hardcoded defaults"))
      (component
        (name "fuzzer")
        (status "placeholder")
        (completion 10)
        (path "fuzz/fuzz_targets/fuzz_main.rs")
        (notes "Generic fuzzer, does not test project code")))
    (working-features
      ("deterministic policy checking via Oracle")
      ("GatingRequest/GatingDecision contract")
      ("CLI with check, policy, init, schema, train, regression commands")
      ("training data directory structure")
      ("Nickel policy schema and default config")))

  (route-to-mvp
    (milestones
      (milestone
        (name "Phase 1: Deterministic Oracle")
        (status "complete"))
      (milestone
        (name "Phase 2: SLM Integration")
        (status "in-progress")
        (notes "llama.cpp bindings needed"))
      (milestone
        (name "Phase 3: Consensus Arbiter")
        (status "skeleton")
        (notes "Elixir GenServer exists, needs Rustler NIF"))
      (milestone
        (name "Phase 4: Nickel Policy Loading")
        (status "not-started")
        (notes "Oracle has hardcoded defaults, Nickel configs exist but not loaded"))))

  (blockers-and-issues
    (critical
      ("License headers use PMPL-1.0-or-later instead of PMPL-1.0-or-later"))
    (high
      ("SLM evaluator is a placeholder returning hardcoded results")
      ("Fuzzer tests generic string ops, not project code"))
    (medium
      ("Consensus Arbiter has no tests or Rustler NIF integration")
      ("Oracle does not load policy from Nickel files"))
    (low
      ("Author email wrong in Cargo.toml files")))

  (critical-next-actions
    (immediate
      ("Fix all AGPL license headers to PMPL")
      ("Fix author email to j.d.a.jewell@open.ac.uk"))
    (this-week
      ("Implement SLM evaluator with llama.cpp bindings")
      ("Make fuzzer target real project code"))
    (this-month
      ("Add Rustler NIF bindings for Elixir arbiter")
      ("Load policy from Nickel configs at runtime")))

  (session-history
    ("2026-02-12: Audit completed, SONNET-TASKS.md written")))
```

### Verification

```bash
# Must be valid s-expressions (no unclosed parens):
# Count opening and closing parens - they must be equal:
OPEN=$(grep -o '(' .machine_readable/STATE.scm | wc -l)
CLOSE=$(grep -o ')' .machine_readable/STATE.scm | wc -l)
[ "$OPEN" -eq "$CLOSE" ] && echo "PASS: balanced parens" || echo "FAIL: $OPEN open vs $CLOSE close"

# Must have PMPL license header:
head -1 .machine_readable/STATE.scm | grep -q "PMPL-1.0-or-later"

# Must show 75% completion:
grep -q "overall-completion 75" .machine_readable/STATE.scm

# Must NOT say AGPL anywhere:
! grep -q "AGPL" .machine_readable/STATE.scm
```

---

## Task 6: Flesh Out the Consensus Arbiter (Elixir)

### Files

- `src/arbiter/mix.exs` — already exists (39 lines), needs license fix (done in Task 1) and possibly test config
- `src/arbiter/lib/conative_gating/consensus_arbiter.ex` — already exists (79 lines), needs tests
- `src/arbiter/lib/conative_gating/application.ex` — already exists (23 lines), fine as-is after license fix
- NEW: `src/arbiter/test/consensus_arbiter_test.exs` — must be created
- NEW: `src/arbiter/test/test_helper.exs` — must be created

### Problem

The Elixir Consensus Arbiter at `src/arbiter/lib/conative_gating/consensus_arbiter.ex` has the core `decide/3` and `weighted_decision/3` functions implemented (lines 37-78), but:

1. There are ZERO tests.
2. The `decide/3` function is a module function, not a GenServer call. The GenServer `init/1` stores state `%{decisions: [], audit_log: []}` but nothing reads or writes to it. The `decide/3` function should probably be a GenServer call that also logs decisions to state.
3. No Rustler NIF bindings exist (this is expected — Rustler NIFs are a later phase). But the mix.exs already lists `{:rustler, "~> 0.30"}` as a dependency. This will fail `mix deps.get` unless Rustler is available. Comment it out for now.
4. There is no `test/` directory at all.

### What to Do

**Step 1: Comment out Rustler dependency**

In `src/arbiter/mix.exs`, line 28, change:
```elixir
{:rustler, "~> 0.30"},  # For Rust NIF integration
```
to:
```elixir
# {:rustler, "~> 0.30"},  # For Rust NIF integration (Phase 4)
```

**Step 2: Add GenServer call wrappers to consensus_arbiter.ex**

The `decide/3` function (line 37) is currently a plain function. Add a GenServer `handle_call` that wraps it and logs decisions to state:

```elixir
def decide_sync(llm_result, slm_result, oracle_result) do
  GenServer.call(__MODULE__, {:decide, llm_result, slm_result, oracle_result})
end

def get_audit_log do
  GenServer.call(__MODULE__, :get_audit_log)
end

@impl true
def handle_call({:decide, llm_result, slm_result, oracle_result}, _from, state) do
  decision = decide(llm_result, slm_result, oracle_result)
  entry = %{
    timestamp: DateTime.utc_now(),
    decision: decision,
    inputs: %{llm: llm_result, slm: slm_result, oracle: oracle_result}
  }
  new_state = %{state | decisions: [decision | state.decisions], audit_log: [entry | state.audit_log]}
  {:reply, decision, new_state}
end

@impl true
def handle_call(:get_audit_log, _from, state) do
  {:reply, state.audit_log, state}
end
```

Keep the existing `decide/3` as a pure function (it is useful for non-GenServer callers).

**Step 3: Create test files**

Create `src/arbiter/test/test_helper.exs`:
```elixir
ExUnit.start()
```

Create `src/arbiter/test/consensus_arbiter_test.exs` with these test cases:

1. **`test "hard violation from oracle blocks immediately"`** — Pass an oracle result with `verdict: {:hard_violation, :forbidden_language}`. Verify result is `{:block, %{reason: :policy_oracle, type: :forbidden_language}}`.

2. **`test "high LLM confidence + low SLM violation = allow"`** — Pass `llm.confidence = 0.9`, `slm.violation_confidence = 0.1`, oracle `verdict: :compliant`. Verify `{:allow, _}`.

3. **`test "high LLM confidence + high SLM violation = block"`** — Pass `llm.confidence = 0.9`, `slm.violation_confidence = 0.7` (0.7 * 1.5 = 1.05 > 0.9). Verify `{:block, _}`.

4. **`test "medium scores = escalate"`** — Pass `llm.confidence = 0.6`, `slm.violation_confidence = 0.3`. Verify `{:escalate, _}`.

5. **`test "soft concern adds 0.2 to no_go score"`** — Pass low SLM confidence but oracle `verdict: {:soft_concern, :verbosity}`. Verify the 0.2 addition pushes toward escalate/block.

6. **`test "asymmetric weight of 1.5 is applied to SLM score"`** — Verify that `slm.violation_confidence = 0.5` becomes an effective score of `0.75` (0.5 * 1.5).

7. **`test "GenServer audit log records decisions"`** — Start the GenServer, call `decide_sync/3`, call `get_audit_log/0`, verify the log contains the decision.

### Verification

```bash
# From src/arbiter/ directory:
cd src/arbiter && mix deps.get && mix test
# All 7 tests must pass

# Verify no AGPL in any file (should be fixed by Task 1):
grep -r "AGPL" src/arbiter/
# Must return nothing
```

---

## Task 7: Add Nickel Policy Loading to Oracle (Optional Enhancement)

### Files

- `src/oracle/src/lib.rs` — add `Policy::from_nickel_json(json: &str)` method
- `config/policy.ncl` — already exists, no changes needed
- `config/schema.ncl` — already exists, no changes needed

### Problem

The Oracle has a `Policy::rsr_default()` method (line 503 of `src/oracle/src/lib.rs`) that returns a hardcoded Rust struct. The Nickel config files exist at `config/policy.ncl` and `config/schema.ncl` with the exact same policy data. But nothing in the Rust code reads those Nickel files. The CLI `init` command (in `src/main.rs`) writes `.conative/policy.ncl` to disk, but the `check` command uses `Oracle::with_rsr_defaults()` which ignores any Nickel files.

Adding a Nickel evaluator dependency to Rust is heavy. Instead, the practical approach is:

1. The Nickel CLI (`nickel export`) can convert `.ncl` files to JSON.
2. The `Policy` struct already derives `Deserialize`.
3. Add a `Policy::from_json(json: &str) -> Result<Self, ...>` method.
4. The CLI can shell out to `nickel export config/policy.ncl` or accept `--policy-json` flag.

### What to Do

**Step 1: Add `Policy::from_json()` to `src/oracle/src/lib.rs`**

Add this method to the `impl Policy` block (after `rsr_default()`, around line 605):

```rust
/// Load policy from JSON (e.g., output of `nickel export policy.ncl`)
pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
    serde_json::from_str(json)
}
```

**Step 2: Add a test**

Add a test to the oracle's test module that verifies `Policy::from_json()` works with a minimal valid JSON policy. The JSON should match the Nickel schema structure.

**Step 3: Do NOT modify `src/main.rs`** — that file is complete. The CLI integration of `--policy-json` or `nickel export` piping can happen later.

### Verification

```bash
# Must compile and pass tests:
cargo test -p policy-oracle

# The new method must exist:
grep -q "fn from_json" src/oracle/src/lib.rs

# Must NOT have modified src/main.rs:
# (check git diff to confirm)
```

---

## Final Verification

After completing ALL tasks, run these commands in order. Every single one must pass.

```bash
# 1. Build the entire workspace
cargo build --workspace

# 2. Run all Rust tests
cargo test --workspace

# 3. Run clippy
cargo clippy --workspace -- -D warnings

# 4. Verify NO AGPL references remain in source files (excluding templates/wordpress/ GPL files and SONNET-TASKS.md)
AGPL_COUNT=$(grep -r "AGPL" --include="*.rs" --include="*.toml" --include="*.ex" --include="*.exs" --include="*.sh" . | grep -v "wordpress/" | grep -v "SONNET-TASKS" | grep -v "docs/" | wc -l)
[ "$AGPL_COUNT" -eq 0 ] && echo "PASS: No AGPL in source files" || echo "FAIL: $AGPL_COUNT files still have AGPL"

# 5. Verify correct author email everywhere
WRONG_EMAIL=$(grep -r "jonathan@hyperpolymath.org" --include="*.toml" --include="*.exs" . | wc -l)
[ "$WRONG_EMAIL" -eq 0 ] && echo "PASS: No wrong email" || echo "FAIL: $WRONG_EMAIL files have wrong email"

# 6. Verify STATE.scm is accurate
grep -q "overall-completion 75" .machine_readable/STATE.scm && echo "PASS: STATE.scm updated" || echo "FAIL: STATE.scm not updated"

# 7. Verify SLM evaluator has real code (not just 81 lines)
SLM_LINES=$(wc -l < src/slm/src/lib.rs)
[ "$SLM_LINES" -gt 150 ] && echo "PASS: SLM evaluator has $SLM_LINES lines" || echo "FAIL: SLM evaluator only $SLM_LINES lines"

# 8. Verify fuzzer imports project crates
grep -q "policy_oracle\|gating_contract" fuzz/fuzz_targets/fuzz_main.rs && echo "PASS: Fuzzer targets project code" || echo "FAIL: Fuzzer still generic"

# 9. Verify Elixir tests exist
[ -f src/arbiter/test/consensus_arbiter_test.exs ] && echo "PASS: Arbiter tests exist" || echo "FAIL: No arbiter tests"

# 10. Verify balanced parens in STATE.scm
OPEN=$(grep -o '(' .machine_readable/STATE.scm | wc -l)
CLOSE=$(grep -o ')' .machine_readable/STATE.scm | wc -l)
[ "$OPEN" -eq "$CLOSE" ] && echo "PASS: STATE.scm balanced ($OPEN parens)" || echo "FAIL: STATE.scm unbalanced ($OPEN open vs $CLOSE close)"
```

**Expected result: 10 PASS, 0 FAIL.**

---

## Task Execution Order

Execute tasks in this order (dependencies flow downward):

```
Task 1 (License fix) ─────── no dependencies, do first
Task 2 (Author email) ────── no dependencies, do first
Task 5 (STATE.scm) ───────── no dependencies, do first
Task 3 (SLM evaluator) ───── do after Task 1 and 2 (needs correct headers)
Task 4 (Fuzzer) ──────────── do after Task 3 (needs parse_slm_response export)
Task 6 (Arbiter tests) ───── do after Task 1 (needs correct headers)
Task 7 (Nickel loading) ──── do last, optional
```

Tasks 1, 2, and 5 can be done in parallel. Tasks 3 and 6 can be done in parallel after 1+2. Task 4 depends on Task 3. Task 7 is independent and optional.
