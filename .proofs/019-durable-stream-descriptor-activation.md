# Proof Report: 019 — Durable PawFS Stream Descriptor Activation

## Date

2026-08-27

## Branch / Commit

- Repository: `nikstern/temperpaw` on GitHub
- Worktree: `/private/tmp/temperpaw-pr20`
- Branch: `codex/pawfs-stream-descriptors`
- Draft PR: <https://github.com/nikstern/temperpaw/pull/20>
- Temper kernel: `f0c431ed0b2a19e3f7834a95cba884ad15085651`

## What Was Done

- Activated `StreamDescriptorV1` for mutable `Paw.FS.File` and immutable,
  parent-authorized `Paw.FS.FileVersion`.
- Moved every Temper host and guest dependency to the reviewed revision that
  contains governed descriptor migration and activation fencing.
- Added locked PawFS WASM builds and generated typed read capability coverage.
- Exercised bounded, durable migration to completion, including resuming the
  same job after PawFS bootstrap advanced the stream generation.
- Reopened the same Turso database and blob directory through a new server state
  and proved both current File and immutable FileVersion bytes remain readable.
- Kept migration-required startup alive but unready, exposing only liveness and
  the governed stream-descriptor migration API through the startup gate.

## Verification Flow

1. Created a legacy PawFS schema without the descriptor contract markers.
2. Created a File, wrote content through `$value`, and observed FileVersion
   fan-out.
3. Reconciled the activated app and observed `MigrationRequired`.
4. Migrated a bounded page containing File and FileVersion, then submitted an
   empty terminal page to prove inventory exhaustion.
5. Attempted install; PawFS ADR bootstrap advanced the stream generation and
   correctly required migration again.
6. Resumed the same durable job until it completed with zero unresolved rows.
7. Reconciled again and crossed the activation fence.
8. Dropped the platform state, created a new server state over the same durable
   database and object directory, and read File content through OData.
9. Compiled and registered the generated PawFS WASM client, then read current
   File and FileVersion content through its artifact-bound typed APIs.

## Verification Results

| Step | Expected | Actual | Status |
|------|----------|--------|--------|
| Descriptor contract | File mutable; FileVersion immutable and parent-authorized | CSDL and contract tests agree | PASS |
| Pre-migration activation | Activated app is rejected | `MigrationRequired` with semantic/capability/version evidence | PASS |
| Bounded migration | Durable pages finish only after inventory exhaustion | First page migrated two subjects; terminal empty page completed | PASS |
| Generation drift | Existing job can resume after bootstrap writes | Same job resumed and completed with zero unresolved rows | PASS |
| Activation fence | Install proceeds only after current completion evidence | Final reconcile installed/skipped successfully | PASS |
| Restart current read | File bytes survive a new server state | Raw `$value` returned expected bytes | PASS |
| Restart version read | Generated client reads immutable version bytes | Typed FileVersion read returned expected bytes | PASS |
| Authorization | Existing PawFS Cedar corridor remains valid | `corridor_cedar_matrix`: 10 passed | PASS |
| PawFS suites | Hot path, versioning, restart all remain green | 17 + 3 + 1 passed | PASS |
| Full TemperPaw tests | Package suite remains green | All suites passed outside sandbox for loopback binds | PASS |
| Static gates | Pin sync, format, clippy, check, diff | All passed | PASS |
| Locked WASM | All three PawFS modules build with `--locked` | artifact batch, blob adapter, workspace FS passed | PASS |
| Startup maintenance gate | Migration API reachable while readiness/normal API blocked | Targeted server gate test passed | PASS |

## What Worked

- Host-attested descriptor metadata, not application `SizeBytes`, is the read
  authority after activation.
- Migration completion evidence is generation-bound and detects writes that
  occur between migration and install.
- Generated guest reads use the verified capability digest for both direct and
  version streams after a durable restart.

## What Didn't Work

No product behavior remains broken in the verified local scope.

Two expected validation observations are worth retaining:

- The restricted Codex sandbox denies loopback binds. The full suite, including
  those tests, passed when run with loopback permission; this is an execution
  environment constraint, not a TemperPaw defect.
- The first installation attempt intentionally remained fenced because PawFS
  bootstrap content advanced the stream generation. Resuming the durable job
  completed migration and allowed activation, demonstrating the safety contract.

## Limitations

This report proves the branch locally. PR merge, Genesis publication, Railway
deployment, installed `owner/app@hash` verification, and Datadog production
confirmation are intentionally pending review and merge.

## What Still Doesn't Work

Production remains on the previously deployed PawFS contract until PR #20 is
reviewed, merged, published to Genesis, and verified live.

## Artifacts

- `os-apps/paw-fs/specs/model.csdl.xml`
- `crates/temperpaw/tests/paw_fs_typed_restart.rs`
- `crates/temperpaw/src/startup.rs`
- `os-apps/paw-fs/adrs/003-durable-stream-descriptor-activation.md`

## Architecture Diagram

```text
legacy File + FileVersion
          |
          v
bounded governed migration ---- generation drift ----+
          |                                           |
          +<--------------- resume same job ----------+
          |
          v
zero unresolved + terminal inventory proof
          |
          v
installed-app activation fence
          |
          v
new server state -> generated typed File/FileVersion reads
```
