# Proof Report: 096 — Durable reactions through a local-first bundle

## Date

2026-08-24

## Branch / Commit

- TemperPaw repository: `nikstern/temperpaw`
- TemperPaw branch: `codex/task-96-local-first-bundle`
- Draft PR: `https://github.com/nikstern/temperpaw/pull/4`
- Temper fork PR: `https://github.com/nikstern/temper/pull/38`
- Exact merged Temper commit: `086c6ff2bd0feb6ce88b373921caab58291843a6`
- Temper source tree: `824e4a9bcbf49b70415686857f4a2d0c9e7ad2d5`

## What Was Done

- Pinned all 88 Temper manifest dependencies and all 49 checked-in lockfile
  entries to the exact merged fork commit containing durable reactions and the
  local-first bundle implementation.
- Added Turso-backed acceptance coverage for recovery of a reaction intent that
  was committed before process exit.
- Added a minimal TemperPaw test bundle with inline `[[action.triggers]]` entity
  wiring and Cedar authorization for its target action.
- Installed that bundle with `temper app install --locked` into tenant `task96`
  on `temper up`, using the explicit data directory
  `/private/tmp/temperpaw-task96.ZPpVRk/data`.
- Exercised reaction delivery before restart, removed the source workspace,
  restarted the same tenant, and exercised a second reaction entirely from the
  durable content-addressed cache.

This is a dependency validation and test-fixture change, not a material
TemperPaw architecture change. No new repository ADR is required.

## Verification Flow

1. Establish red against the old `9a2bf1fa` pin.
2. Repin every Temper dependency to merged commit `086c6ff2...`.
3. Run the Turso recovery test and locked core build.
4. Start `temper up` on loopback with an explicit data directory and invalid
   remote-storage URLs.
5. Install the fixture with `temper app install --locked`.
6. Dispatch `Order.ConfirmOrder` and read the reacted `Payment`.
7. Stop Temper and rename the source fixture directory so the original
   workspace path does not exist.
8. Restart from the same data directory, verify the existing target, dispatch a
   new order, and verify the new target.
9. Restore the fixture directory after the proof.

## Verification Results

| Step | Expected | Actual | Status |
|------|----------|--------|--------|
| Red | Old pin cannot compile the merged delivery envelope test | `DeliveryKind`, `kind`, `not_before`, and `state_timeout` were absent | Pass |
| Pin consistency | Every Temper dependency resolves one immutable revision | 88 manifest pins and 49 lock entries verified at `086c6ff2...` | Pass |
| Locked core build | TemperPaw and worker compile against the merge | `cargo check --locked -p temperpaw -p paw-codex-worker` passed | Pass |
| Restart recovery | Committed source intent is delivered after state reconstruction | `Payment:order-1` reached `Authorized` in the Turso test | Pass |
| Locked install | Local app is verified and cached immutably | Installed digest `sha256:6d86157a6044a9e643cc86688db19044a08306cec252b12760d6b3ed8f55d4d8` | Pass |
| Before restart | Inline entity reaction authorizes its target | `payment-before` = `Authorized`, receipt `reaction-v1-8cd5...` | Pass |
| Workspace unavailable | Source path is absent during restart | Original fixture path did not exist; it was present only as `.workspace-unavailable` | Pass |
| Cache restore | Restart restores the installed app without source access | Startup reported `Restored app cache roots: local=1` and installed two cached entity specs | Pass |
| State recovery | Pre-restart target remains queryable | `payment-before` = `Authorized` after restart | Pass |
| Post-restart delivery | Cached inline reaction handles new work | `payment-after` = `Authorized`, receipt `reaction-v1-fef5...` | Pass |

## What Worked

- The locked local bundle produced a stable content-addressed manifest and a
  materialized cache view under the explicit data directory.
- Restart did not consult the unavailable source workspace.
- Both pre-restart and post-restart target entities carried durable
  `reaction-v1-*` idempotency receipts.
- The merged Temper commit passed both direct recovery coverage and the live
  local-first install/restart path.

## What Didn't Work

- The first fixture revision allowed only the synthetic reaction principal, so
  the local operator correctly received HTTP 403 when creating the source
  order. A test-only operator policy was added, the bundle was reinstalled, and
  the final digest above passed the full flow.
- The first sandboxed `temper up` could initialize Turso but could not bind the
  loopback port (`Operation not permitted`). The same loopback-only command was
  rerun with approved host execution.

## Limitations

- This proof makes no production, Railway, or Genesis deployment claim.
- The fork pin is development-only until the required Temper lineage is merged
  into the canonical `nerdsane/temper` repository.
- The temporary data directory is retained at
  `/private/tmp/temperpaw-task96.ZPpVRk` for local inspection; it contains a
  generated credential and must not be committed.

## What Still Doesn't Work

- Production cannot use `nikstern/temper` as its permanent dependency source.
  After the change lands upstream, run
  `scripts/pin-temper-dependencies.py nerdsane/temper <exact-upstream-commit>`,
  regenerate the lockfiles, and rerun this same locked recovery and
  workspace-free bundle proof before merge or deployment.

## Artifacts

- `crates/temperpaw/tests/durable_reaction_recovery.rs`
- `crates/temperpaw/tests/fixtures/local-bundle-durable-reactions/`
- `scripts/pin-temper-dependencies.py`
- Local cache manifest:
  `/private/tmp/temperpaw-task96.ZPpVRk/data/bundles/v1/manifests/sha256/6d86157a6044a9e643cc86688db19044a08306cec252b12760d6b3ed8f55d4d8.json`

## Architecture Diagram

```text
TemperPaw fixture workspace
          |
          | temper app install --locked
          v
sha256:6d86157a... immutable bundle
          |
          v
explicit Turso data dir + content-addressed cache
          |
          +--> before restart: Order.ConfirmOrder
          |                         |
          |                         v
          |                 Payment = Authorized
          |
          +--> source workspace unavailable
          |          |
          |          v
          |     temper up restart
          |          |
          |          v
          +--> cached bundle restored
                                    |
                                    v
                           new reaction delivered
```
