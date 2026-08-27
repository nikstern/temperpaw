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
- Selected Temper commit: `d835447fa300b27cdb6355613f61b813dab90e1e`
- Source change: merge commit for <https://github.com/nikstern/temper/pull/66>

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

## Verification Flow

| Step | Expected | Actual | Status |
|------|----------|--------|--------|
| Red typed execution | Old runtime cannot execute the typed route | Entity remained `Running` instead of reaching `Failed` | Pass |
| Red legacy control | Old runtime still executes `on_failure` | Entity reached `Failed` | Pass |
| Canonical synchronization | All tracked manifests and lockfiles resolve one source/revision | 95 manifest dependencies and 77 lockfile entries verified | Pass |
| Typed conformance | Typed fixture parses, verifies, compiles, and executes | Missing module routed through `failure_v1`; entity reached `Failed` | Pass |
| Legacy compatibility | Legacy callback executes unchanged after upgrade | Entity reached `Failed` | Pass |
| Full workspace build/test | All native targets compile and tests pass from lockfile | Pending | Pending |
| All OS-app WASM builds | Every module with a checked-in manifest builds from lockfile | Pending | Pending |
| Server boot | Upgraded server starts without fatal errors | Pending | Pending |

## Verification Results

Completed commands:

```text
cargo test -p temperpaw --test typed_failure_conformance typed_failure_v1_route_parses_verifies_and_executes -- --exact --nocapture
# RED: expected Failed, actual Running

cargo test -p temperpaw --test typed_failure_conformance legacy_on_failure_callback_still_executes_unchanged -- --exact --nocapture
# PASS

scripts/sync-temper-kernel d835447fa300b27cdb6355613f61b813dab90e1e
scripts/sync-temper-kernel --check
# verified 95 manifest dependencies and 77 lockfile entries

cargo test --locked -p temperpaw --test typed_failure_conformance -- --nocapture
# 2 passed
```

## What Worked

- The red test isolated the missing runtime behavior rather than only checking
  dependency text.
- The same missing-WASM event now routes through the typed integrity category.
- The legacy control proves the upgrade does not require product-spec migration.
- Cargo introduced `temper-failure` consistently into affected lockfiles.

## What Didn't Work

- The first synchronization attempt could not write Cargo's shared Git checkout
  under the sandbox. Re-running the same canonical command with approved access
  succeeded.

## Limitations

- Production deployment, Genesis publication, Railway verification, and
  Datadog verification are post-merge gates and remain pending.
- This change proves the platform boundary with a conformance fixture; it does
  not migrate shipped product flows.

## What Still Doesn't Work

- Full workspace, all-module WASM, and boot verification are still in progress.

## Artifacts

- `.temper-kernel.toml`
- `crates/temperpaw/tests/typed_failure_conformance.rs`
- `crates/temperpaw/tests/fixtures/typed-failure-conformance/`
- `docs/adrs/0066-typed-failure-envelope-platform-boundary.md`
- Durable memory decision note: `201`

## Architecture Diagram

```text
.temper-kernel.toml (nikstern/temper@d835447f)
        |
        +--> server/parser/verifier/JIT/codegen
        |
        +--> all temper-wasm-sdk manifests + lockfiles
        |
        v
FailureRouteProbe.Run --missing WASM--> failure_v1(integrity) --> Fail

Legacy FailureRouteProbe.Run --missing WASM--> on_failure ------> Fail
```
