# Proof Report: 115 — Scoped-schema Temper development image

## Date

2026-08-20

## Branch / Commit

- Repository: `nerdsane/temperpaw` worktree at `/Users/nik.stern/projects/temperpaw/.local/worktrees/task-115-scoped-schema-pin`
- Branch: `codex/task-115-scoped-schema-pin`
- Push remote: `fork` (`github.com/nikstern/temperpaw`)
- Reviewed Temper dependency: `nikstern/temper@6466aa10773ecf5231bb6023b5dcfaccb6bae3e8`
- Change commits: `c706187f`, `6867a4d1`, `ad06600b`

## What Was Done

- Pinned every Temper manifest and checked-in lock entry to the exact reviewed fork commit.
- Added a deterministic scoped-bundle fixture that locks the canonical digest contract used by the runtime smoke.
- Added the missing `artifact_batch_apply` WASM build to the Docker and CI packaging surfaces.
- Built and ran an immutable ARM64 development image.
- Exercised the exact dependency commit's governed submit, verify, activate, migrate, retire, persistence, and restart-recovery contracts.
- Booted and restarted TemperPaw with its packaged installed-app surface.

The temporary fork pin is development-only. Before production merge it must be replaced with an exact `nerdsane/temper` commit, then the pin-contract tests and all lockfiles must be refreshed. Genesis remains the only production app source.

No new ADR was added. This effort changes only a temporary dependency revision, development fixture, and packaging completeness; it does not introduce a new architecture or production deployment behavior. The scoped-schema architecture is owned by the reviewed Temper change.

## Verification Flow

1. Wrote the dependency-pin contract first and observed 86 stale WASM manifest references.
2. Pinned all manifests and lockfiles, then reran the contract to green.
3. Extended the image packaging contract and observed failures first for the Dockerfile and then CI before adding `artifact_batch_apply` to both.
4. Wrote the scoped-bundle digest fixture with a red sentinel, compiled through the pinned `temper-spec`, then locked the emitted canonical digest.
5. Built `temperpaw-local:task-115-6867a4d1`, inspected its digest and platform, started it on local port 3468, and checked `/healthz` and `/readyz`.
6. Submitted the fixture through the live schema endpoint with both explicit admin headers and the disposable bearer identity. Cedar denied both and created pending governance decisions; no bypass was added.
7. Ran the governed lifecycle, Turso, scoped reaction, restart, and retirement tests from a detached worktree at exact Temper commit `6466aa10`.
8. Restarted the same container and verified persisted recovery and readiness.
9. Ran formatting, pin consistency, check, clippy, and the complete TemperPaw test suite.

## Verification Results

| Step | Expected | Actual | Status |
|------|----------|--------|--------|
| Pin contract | One immutable Temper revision everywhere | 88 manifest pins and 48 lock entries at `6466aa10`; zero drift | PASS |
| Red/green pin test | Stale refs fail before rewrite | 86 stale WASM refs reported; exact test passed after rewrite | PASS |
| Red/green image contract | Required paw-fs WASM present in Docker and CI | Failed on Docker, then CI; passed after both included `artifact_batch_apply` | PASS |
| Scoped digest contract | Pinned compiler emits stable identity | `sha256:6e48666e22a3a4c6a6579c76608c2d247b908c092ba779623e8d9eca7253713e` | PASS |
| Immutable image | Identified runnable development artifact | `temperpaw-local@sha256:d51ba53f21b6f36a047debad9caa1a5676ac98489d69e44e695676f457cca761`, Linux ARM64 | PASS |
| Initial runtime boot | Server and installed apps become ready | Ready in 7,747 ms; specs committed and packaged OS apps reconciled | PASS |
| Live schema request | Security policy remains enforced | HTTP 403 for admin and bearer identities; pending decisions created | PASS (security boundary) |
| Governed lifecycle | Submit/get/verify/activate/migrate/retire succeed | 4/4 focused service tests passed | PASS |
| Turso persistence | Deployment and migration records persist and cut over | 2/2 contract tests passed | PASS |
| Scoped restart recovery | Exact pinned config remains recoverable | 3/3 integration tests passed | PASS |
| Retirement pin semantics | New resolution stops; exact pinned reads survive | Focused registry test passed | PASS |
| Container restart | Same state returns to ready | One transient 503 during boot, then ready in 462 ms; recovery failures 0 | PASS |
| Existing app compatibility | Global packaged surface still boots | Full image boot/reconcile passed; full TemperPaw suite passed | PASS |
| Local quality gates | Format/check/clippy/tests green | Check and clippy green; `cargo test --locked -p temperpaw` green after loopback permission escalation | PASS |
| Railway and Datadog | Live development deployment observable | Blocked by absent `RAILWAY_TOKEN`, `RAILWAY_PROJECT_ID`, and `RAILWAY_ENVIRONMENT` | BLOCKED |
| Draft PR | Draft PR exists at start of work | GitHub rejected PR creation for the Enterprise Managed User | BLOCKED |

## What Worked

- The development image compiled the full server and required WASM set from one exact Temper revision.
- The image booted cleanly, retained its state across restart, and recovered materially faster on the second boot.
- Exact-commit tests covered migration cutover, durable storage, scoped reaction recovery, and retirement pinning without weakening Cedar.
- All local code-quality and compatibility gates passed.

## What Didn't Work

- `gh pr create` failed for both the fork and cross-fork target with: `Unauthorized: As an Enterprise Managed User, you cannot access this content (createPullRequest)`.
- The live schema API has no installed Cedar grant for the tested admin or bearer identities. It correctly denied both requests and surfaced pending decisions. Adding a capability policy is outside this dependency-image task and would require a separately reviewed authorization design.
- The first unprivileged full test run had 67 passes and 6 loopback-bind failures (`Operation not permitted`). The same suite passed with permission to bind local test sockets.

## Limitations

- The image is a local ARM64 development artifact, not a production or multi-architecture release.
- Runtime telemetry was deliberately disabled for the local smoke, so it is not Datadog production evidence.
- The image was built from runtime commit `6867a4d1`; `ad06600b` adds only a test dependency and test fixture and does not alter the runtime artifact.

## What Still Doesn't Work

- Railway deployment, live external smoke, and Datadog confirmation remain impossible until the three Railway secrets/config values recorded on Temper task 114 are restored.
- A draft PR cannot be opened by the current managed GitHub identity. The pushed branch is available at `fork/codex/task-115-scoped-schema-pin` for an authorized identity to open.
- Production merge is forbidden while manifests reference `nikstern/temper`; replace the fork pin with an exact upstream commit first.

## Artifacts

- Image tag: `temperpaw-local:task-115-6867a4d1`
- Image digest: `sha256:d51ba53f21b6f36a047debad9caa1a5676ac98489d69e44e695676f457cca761`
- Disposable smoke container: `temperpaw-task115-smoke`
- Durable decision memory: note `103`
- Temper CI evidence inherited from task 114: run `32376067001`
- Skipped Temper deployment evidence inherited from task 114: run `32378585233`

## Architecture Diagram

```text
TemperPaw development image
  -> exact Temper pin 6466aa10
  -> packaged global OS apps (existing compatibility)
  -> governed scoped-schema service
       -> Cedar authorization boundary
       -> immutable bundle registry
       -> verification + migration
       -> Turso durable records
       -> restart recovery + retirement pins
```
