# Proof Report: 011 — Typed Failure Envelope Support

## Date

2026-08-27

## Branch / Commit

- Repository: `nikstern/temperpaw` on GitHub (`github-commercial` remote host)
- Worktree: `/Users/nik.stern/projects/temperpaw/.local/worktrees/issue-11-typed-failure-pin`
- Branch: `codex/issue-11-typed-failure-pin`
- Draft PR: <https://github.com/nikstern/temperpaw/pull/21>
- Red contract commit: `3d12954a`
- Selected Temper source: `https://github.com/nikstern/temper.git`
- Selected Temper commit: `e3dfe852e7a7373cef8bddfe2e3b8bcad8f94a0a`
- Required Temper changes: merged <https://github.com/nikstern/temper/pull/66>
  plus draft dependency <https://github.com/nikstern/temper/pull/75>

## What Was Done

- Added a typed `failure_routes` fixture with the exact `failure_v1` callback
  ABI and a separate unchanged legacy `on_failure` fixture.
- Added executable conformance coverage across parsing, the Temper verification
  model, registry/JIT compilation, and runtime callback dispatch.
- Advanced every tracked Temper dependency and all checked-in nested WASM
  lockfiles to one reviewed commit through `scripts/sync-temper-kernel`.
- Expanded canonical pin discovery to tracked reference-project manifests so no
  Temper dependency remains outside the check.
- Recorded the incremental compatibility boundary in ADR-0066. No product flow
  was migrated.
- Fixed Temper's local-catalog verifier to use the same dependency-complete
  module closure as SDK generation and artifact binding.
- Declared the existing `Paw.FS.File` stream mutable and made startup fail
  closed with actionable digest details when Temper fences activation behind a
  governed stream migration.

## Verification Flow

| Step | Expected | Actual | Status |
|------|----------|--------|--------|
| Red typed execution | Old runtime cannot execute the typed route | Entity remained `Running` instead of reaching `Failed` | Pass |
| Red legacy control | Old runtime still executes `on_failure` | Entity reached `Failed` | Pass |
| Canonical synchronization | All tracked manifests and lockfiles resolve one source/revision | 95 manifest dependencies and 159 lockfile entries verified | Pass |
| Typed conformance | Typed fixture parses, verifies, compiles, and executes | Missing module routed through `failure_v1`; entity reached `Failed` | Pass |
| Legacy compatibility | Legacy callback executes unchanged after upgrade | Entity reached `Failed` | Pass |
| Full workspace build/test | All native targets compile and tests pass from lockfile | `cargo build --locked --workspace` and `cargo test --locked --workspace` passed | Pass |
| All OS-app WASM builds | Checked-in locks and the exact CI packaging surface build | 29 locked modules plus all 12 CI build scripts and route verifier passed | Pass |
| Server boot | Upgraded server installs local apps and becomes ready | Fresh Turso boot completed; `/healthz` and `/readyz` returned 200 | Pass |

## Verification Results

Completed commands:

```text
cargo test -p temperpaw --test typed_failure_conformance typed_failure_v1_route_parses_verifies_and_executes -- --exact --nocapture
# RED: expected Failed, actual Running

cargo test -p temperpaw --test typed_failure_conformance legacy_on_failure_callback_still_executes_unchanged -- --exact --nocapture
# PASS

scripts/sync-temper-kernel e3dfe852e7a7373cef8bddfe2e3b8bcad8f94a0a
scripts/sync-temper-kernel --check
# verified 95 manifest dependencies and 159 lockfile entries

cargo test --locked -p temperpaw --test typed_failure_conformance -- --nocapture
# 2 passed

cargo build --locked --workspace
cargo test --locked --workspace
# passed

# Each of 29 tracked os-app lockfile directories:
cargo build --locked --release --target wasm32-unknown-unknown --manifest-path <module>/Cargo.toml
# passed

# Exact `.github/workflows/ci.yml` WASM build-script list, followed by:
bash scripts/verify_route_message_wasm.sh
# hash=56a168df23ba81edb0b153b15ad3915b682d8631ab0db2f30dfd5c5fa8977c91
# size_bytes=544131

python3 -m unittest scripts.tests.test_sync_temper_kernel -v
cargo clippy --locked -p temperpaw -p paw-codex-worker --all-targets -- -D warnings
# passed

GET http://127.0.0.1:34795/healthz
# 200 OK
GET http://127.0.0.1:34795/readyz
# 200 OK, {"status":"ready", ...}
```

## What Worked

- The red test isolated the missing runtime behavior rather than only checking
  dependency text.
- The same missing-WASM event now routes through the typed integrity category.
- The legacy control proves the upgrade does not require product-spec migration.
- Cargo introduced `temper-failure` consistently into affected lockfiles.
- The final module SDK closure and dependency lock digest are
  `sha256:242511156105b4614dfc15c0bbe5e19d00d9b17e75bc88cabe9c94c6b0054230`.
- The fresh boot installed all ten startup apps, including the bound
  `plan_approval_handler`, before readiness.

## What Didn't Work

- The first synchronization attempt could not write Cargo's shared Git checkout
  under the sandbox. Re-running the same canonical command with approved access
  succeeded.
- The PR #66 merge commit exposed a pre-existing mismatch between local app
  installation's root-only verification closure and the canonical closure used
  by SDK generation. Temper PR #75 fixes the root cause; no TemperPaw bypass was
  added.
- Temper PR #75's first implementation exceeded the repository readability
  ratchet by 11 lines. Moving closure resolution into the existing data-binding
  module restored the maximum file size to the 2,779-line baseline; the ratchet,
  focused regression, full platform tests, and clippy all pass at the final pin.
- The first post-build boot correctly rejected `plan_approval_handler` after
  the CI build script overwrote the bound artifact with raw compiler output.
  Rebinding the final CI-built artifact restored the exact digest proof, and a
  new isolated boot reached readiness.

## Limitations

- Production deployment, Genesis publication, Railway verification, and
  Datadog verification are post-merge gates and remain pending.
- Temper PR #75 is still a draft dependency. TemperPaw PR #21 cannot be merged
  or deployed until that upstream change is reviewed and available at the
  pinned immutable commit.
- This change proves the platform boundary with a conformance fixture; it does
  not migrate shipped product flows.

## What Still Doesn't Work

- Nothing remains for local conformance. Review, merge, Genesis publication,
  Railway deployment, and Datadog live verification remain external gates.

## Artifacts

- `.temper-kernel.toml`
- `crates/temperpaw/tests/typed_failure_conformance.rs`
- `crates/temperpaw/tests/fixtures/typed-failure-conformance/`
- `docs/adrs/0066-typed-failure-envelope-platform-boundary.md`
- `os-apps/paw-fs/adrs/003-durable-stream-descriptor-activation.md`
- Durable memory decision note: `201`

## Architecture Diagram

```text
.temper-kernel.toml (nikstern/temper@e3dfe852)
        |
        +--> server/parser/verifier/JIT/codegen
        |
        +--> all temper-wasm-sdk manifests + lockfiles
        |
        v
FailureRouteProbe.Run --missing WASM--> failure_v1(integrity) --> Fail

Legacy FailureRouteProbe.Run --missing WASM--> on_failure ------> Fail
```
