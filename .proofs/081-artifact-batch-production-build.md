# Proof Report: 081 — ArtifactBatch Production Build

## Date

2026-08-23

## Branch / Commit

- Repository: `nikstern/temperpaw` on GitHub
- Worktree: `/private/tmp/temperpaw-artifact-batch-followup`
- Branch: `codex/artifact-batch-followup`
- Draft PR: <https://github.com/nikstern/temperpaw/pull/3>
- Verified implementation commit: `7a081890f`

## What Was Done

- Added `paw-fs/wasm/artifact_batch_apply/build.sh` to the production Docker build.
- Added the same module build to CI's required WASM build step.
- Extended the production-build contract test to require all three `paw-fs` WASM modules in both build surfaces.
- Retained and adapted `scripts/pin-temper-dependencies.py` for synchronized fork/revision updates: manifests use the repository's eight-character revision convention while lockfile sources retain the full commit.
- Kept durable reaction-recovery acceptance coverage in Temper. The pinned Temper revision already owns the kernel-level scenario in `crates/temper-server/tests/reaction_recovery_e2e.rs`; copying it into TemperPaw would duplicate coverage and bind this repository to private delivery-journal details.

This is build/package parity and dependency-maintenance work, not an architecture change. No ADR was added.

## Verification Flow

1. Wrote the production-build contract first and observed it fail because Docker did not build `artifact_batch_apply`.
2. Added the Docker and CI build entries and reran the contract successfully.
3. Wrote the synchronized-repin behavior test first and observed it fail because the helper was absent.
4. Added the helper, verified its fixture behavior, and verified the repository's current Temper pins without changing them.
5. Built the module directly, formatted the workspace, ran every locked workspace test, and built every workspace crate.
6. Built the complete production Docker image, started it with telemetry disabled, and exercised `/healthz` and `/readyz`.
7. Inspected runtime startup evidence to confirm the packaged module registered and `paw-fs` reconciled it without a WASM failure.

## Verification Results

| Step | Expected | Actual | Status |
|------|----------|--------|--------|
| Red production contract | Missing Docker build is detected | Failed with `Dockerfile must build required paw-fs module artifact_batch_apply` | PASS |
| Green production contract | Docker and CI require all `paw-fs` modules | `temperpaw_identity_contract` passed | PASS |
| Red repin-helper contract | Missing helper is detected | Behavioral test failed before helper existed | PASS |
| Green repin-helper contract | Short manifest pin and full lock commit update together | `datadog_observability_contract` passed | PASS |
| Pin consistency | Current fork and revision are synchronized | 87 manifest pins and 50 lockfile entries verified; zero changes | PASS |
| Direct WASM build | Guest module compiles | `artifact_batch_apply.wasm` built successfully | PASS |
| Formatting | Workspace is formatted | `cargo fmt --all --check` passed | PASS |
| Workspace tests | All locked workspace tests pass | `cargo test --locked --workspace` passed | PASS |
| Workspace build | All locked workspace crates compile | `cargo build --locked --workspace` passed | PASS |
| Production image | Full image builds with the module | Image digest `aafbff2735f33eacbde62bc9c51d51009b9ca5ccc2c78e6d88f56d10b704c823` | PASS |
| Runtime health | Production image boots cleanly | `/healthz` and `/readyz` returned HTTP 200 | PASS |
| Runtime module registration | Packaged module loads without failure | Registered at SHA-256 `f9c79c7b19ea63052ab6272f72e8df313d73168048614399a2398db984990d95`; `wasm_failures: []` | PASS |

## What Worked

- The Docker layer compiled `artifact-batch-apply` and copied the module-local WASM into the runtime image.
- The runtime registered exactly the hash independently observed for the packaged file.
- `paw-fs` reconciled `artifact_batch_apply`, `blob_adapter`, and `workspace_fs` together.
- The helper can check or update all Temper git pins atomically without rewriting unrelated files.

## What Didn't Work

- An authenticated local browser-cookie request to `/observe/wasm/modules` was denied by Cedar (`no matching permit policy`). This was expected for that human principal and was not bypassed; runtime registration was verified from the production process's structured startup record.
- The first sandboxed workspace-test run could not reach the configured Datadog telemetry endpoint. The same locked workspace suite passed when rerun with network access.

## Limitations

- This is pre-merge verification. Genesis publication and Railway/Datadog production verification must occur after PR #3 merges.
- No duplicate TemperPaw durable-reaction test was added because the pinned Temper kernel already owns that behavior.

## What Still Doesn't Work

- Nothing within the focused build/package contract remains broken.

## Artifacts

- Production image tag: `temperpaw:artifact-batch-followup`
- Runtime module path: `/app/os-apps/paw-fs/wasm/artifact_batch_apply/artifact_batch_apply.wasm`
- Runtime module size: 272,714 bytes
- Runtime module SHA-256: `f9c79c7b19ea63052ab6272f72e8df313d73168048614399a2398db984990d95`

## Architecture Diagram

```text
artifact_batch_apply/build.sh
          |
          +---- CI required-WASM build
          |
          +---- production Docker build
                        |
                        v
       module-local artifact_batch_apply.wasm
                        |
                        v
             paw-fs OS-app installation
                        |
                        v
      ArtifactBatch.Apply WASM integration available
```
