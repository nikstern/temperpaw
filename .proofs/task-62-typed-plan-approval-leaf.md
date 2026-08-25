# Proof Report: 62 — Typed Plan Approval Leaf

## Date

2026-08-25

## Branch / Commit

- TemperPaw: `codex/task-62-typed-leaf` (GitHub `nikstern/temperpaw`)
- Temper dependency: merged PR #57 commit
  `3529043f4b0757aefcb4d007e7571104c75bbc32`.

## What Was Done

`plan_approval_handler` now reads `TemperPaw.Session` and invokes
`ResumeWithPlanApproval` through its generated, artifact-bound Temper module
data client. The module no longer owns loopback `/tdata` URLs, authorization
headers, JSON envelope decoding, or HTTP status classification.

The canonical IOA compiler exposed an existing CronJob-to-Session parameter
scope error during validation. `Session.ConfigureScheduledRun` now defines the
exact Session initializer that CronJob owns; ADR-038 records that app-scoped
state-machine decision.

## Verification Flow

1. Added the typed-client contract before implementation and observed it fail.
2. Generated the SDK from the locked `paw-agent` + `paw-fs` metadata closure.
3. Compiled and artifact-bound `plan_approval_handler`.
4. Ran module generation drift checking, focused migration tests, the full
   `temperpaw` test suite, formatting, check, and strict clippy.
5. Started Temper on `127.0.0.1:31262` with tenant `task62` and isolated data
   directory `/private/tmp/temper-task62-e2e.UvYG4H`.
6. Built a locked canonical `paw-agent` bundle with its explicit `paw-fs`
   dependency and installed bundle
   `sha256:753a3b6a97fbfdcc9feea52a0279df810c4fc1dfff5199531f363cb507c6e335`.
7. Stopped Temper, restarted it without configuring a TemperPaw source tree,
   and queried health, OData metadata, and the registered WASM inventory.
8. Attempted to create the isolated Session needed to exercise the complete
   `Plan.Approve` route. The installed Cedar policy denied the operator
   credential with `AuthorizationDenied`; no policy was weakened or bypassed.

## Verification Results

| Step | Expected | Actual | Status |
|------|----------|--------|--------|
| Red contract | Missing generated SDK fails | Contract failed before generation | Pass |
| Generated SDK check | No metadata/source drift | Closure `sha256:004174499112f1abe4f8f47699a6ce8b66610e67c3d318dd1dee4ddc837aada0` checked cleanly | Pass |
| Focused contract | Typed handler and cron initializer contracts pass | 3 tests passed | Pass |
| Full TemperPaw tests | All `temperpaw` tests pass | All unit and integration groups passed | Pass |
| Check/lint/format | No compile, lint, or format failures | `cargo check --locked`, strict clippy, and fmt check passed | Pass |
| Locked install | Immutable closure and artifact binding activate | Bundle installed into tenant `task62` | Pass |
| Cache restart | Runtime restores without app sources | `Restored app cache roots: local=1`; server listened normally | Pass |
| Live surface | Installed schema and handler remain available | `/healthz` 200, `/tdata/$metadata` 200 with `ConfigureScheduledRun`, `/observe/wasm/modules` 200 with `plan_approval_handler` | Pass |
| Routed approval | `Plan.Approve` resumes a waiting Session | Test setup was denied because the isolated operator has no matching create-Session permit | Blocked |

## What Worked

- The typed client preserved the Session sequence as the action precondition.
- Immutable install regenerated the binding from the declared app-rooted
  dependency closure rather than ambient tenant schema.
- The installed bundle and typed handler survived a cache-only restart.

## What Didn't Work

- The first locked install exposed two Temper defects: bundle identity was
  incorrectly substituted for the module metadata-lock identity, and raw
  filesystem component order differed from canonical normalized path order.
  Merged Temper PR #57 contains both red-green fixes.
- The cache-restart operator credential can read Sessions but cannot create the
  isolated Session fixture required for the routed approval test. The server
  returned `403 AuthorizationDenied`. The test did not modify Cedar policy or
  upload an ungoverned test driver.

## Limitations

This is the first leaf slice of task 62. The PR remains draft because the
complete routed Session approval proof is not yet green. The separate
deployment/Genesis and Datadog production gate remains pending until the
TemperPaw PR is merged. Broader task-62 module migrations remain separate
slices.

## What Still Doesn't Work

The current `paw-agent` manifest does not declare every artifact produced by
its legacy aggregate WASM build script. Immutable install reports those extra
artifacts as ignored; this slice does not expand module declarations because it
would broaden scope beyond the typed leaf.

## Artifacts

- Temper PR #57: `https://github.com/nikstern/temper/pull/57`
- TemperPaw PR #5: `https://github.com/nikstern/temperpaw/pull/5`
- Generated SDK: `os-apps/paw-agent/wasm/plan_approval_handler/src/temper_module_sdk.rs`
- Module lock: `os-apps/paw-agent/temper-module-sdk.lock`
- App decision: `os-apps/paw-agent/adrs/038-cron-scoped-session-initializer.md`

## Architecture Diagram

```text
Plan.Approve
    |
    v
plan_approval_handler.wasm
    |  typed entity_get (exact Session)
    |  typed ResumeWithPlanApproval (expected sequence)
    v
Session: WaitingForApproval -> PreparingContext
```
