# Proof Report: 022 — PawFS Required Action Parameters

## Date

2026-08-31

## Branch / Commit

- Repository: `nikstern/temperpaw` on GitHub (`origin` = `git@github-commercial:nikstern/temperpaw.git`)
- Worktree: `/private/tmp/temperpaw-issue22`
- Branch: `codex/issue-22-required-params`
- Draft PR: https://github.com/nikstern/temperpaw/pull/23
- Implementation commit: `2688ba6d`

## What Was Done

- Made PawFS CSDL action bindings non-nullable and aligned every callable IOA action with an exact CSDL action twin.
- Kept only three intentionally optional action inputs: `Directory.Create.parent_id`, `WorkspaceUsageBucket.Create.artifact_batch_id`, and `WorkspaceUsageBucket.ApplyDelta.artifact_batch_id`.
- Added an exhaustive IOA/CSDL contract test and ADR 004 documenting the required-by-default decision.
- Rebuilt all three PawFS WASM integrations.

## Verification Flow

1. Added the contract test before changing PawFS. It failed because the three optional IOA values were implicit and `ArtifactBatch.Apply` still had a nullable CSDL binding.
2. Updated the IOA and CSDL contracts until the new test passed.
3. Built Temper CLI from PR 90 implementation commit `3352587d554ea7bf04fedb9af42b309dac87cd74` and generated a module SDK over the real PawFS dependency closure.
4. Loaded the real PawFS CSDL and `ArtifactBatch` IOA into a temporary in-process PR 90 runtime proof. Exercised missing, explicit-null, wrong-type, and valid `Submit` calls and inspected state/events after each call.
5. Ran PR 90's generic no-side-effect, schema-gate, restart, and compatibility-direction tests.
6. Rebuilt PawFS WASM modules, ran the TemperPaw focused tests, then ran formatting, workspace check, strict clippy, and the complete workspace test suite.

## Verification Results

| Step | Expected | Actual | Status |
|------|----------|--------|--------|
| Red contract test | Expose mismatched requiredness | Failed on implicit optionals and nullable `ArtifactBatch.Apply` binding | Pass |
| PawFS contract test | Exact CSDL twins and only three explicit nullable inputs | 2 passed | Pass |
| PR 90 SDK closure generation | Generate against PawFS without nullable-binding failure | Generated; closure digest `sha256:fa821180315d05044d4fbd7b714e5129b498d7b9883eb2c62aa602dc9cb97178` | Pass |
| Missing required input | Reject without state/event effects | `MissingActionParameter`; state `Submitted`, zero events, count 0 | Pass |
| Explicit null required input | Reject without state/event effects | `MissingActionParameter`; state `Submitted`, zero events, count 0 | Pass |
| Wrong type | Reject without state/event effects | `ActionParameterTypeMismatch`; state `Submitted`, zero events, count 0 | Pass |
| Valid input | Transition exactly once | Accepted; one event, count 1 | Pass |
| Runtime generic rejection tests | Reject before transition/effects | Actor and router tests passed | Pass |
| Restart persistence | Preserve typed values after reopen | Turso reopen test and PawFS typed restart test passed | Pass |
| Upgrade directions | Gate nullable-to-required; permit required-to-nullable with proof | Both PR 90 compatibility tests passed | Pass |
| PawFS WASM | All integrations compile | `artifact_batch_apply`, `blob_adapter`, and `workspace_fs` built | Pass |
| IOA verification | L0-L3 verification succeeds for all PawFS specs | 6 specs passed symbolic, model, 5-seed simulation, and 100 property cases | Pass |
| Formatting | No changes required | `cargo fmt --all -- --check` passed | Pass |
| Workspace check | All targets compile | `cargo check --locked --workspace --all-targets` passed | Pass |
| Strict clippy | No warnings | `cargo clippy --locked --workspace --all-targets -- -D warnings` passed | Pass |
| Full tests | No regressions | `cargo test --locked --workspace` passed | Pass |

## What Worked

- The PR 90 generator emitted `Option<String>` only for the three deliberate nullable inputs. Required inputs such as `ArtifactBatch.Complete.usage_bucket_id`, `ArtifactBatch.Submit.submitted_by`, and `File.StreamUpdated.previous_version_id` were plain `String`.
- Schema rejection occurred before state transitions or event emission, while valid input transitioned once.
- The current-kernel typed PawFS migration/restart path remained green.

## What Didn't Work

- The legacy `temper serve --app NAME=specs` loader verified the IOAs but did not expose the supplied PawFS CSDL through its tenant OData service. The exact-CSDL runtime cases therefore used Temper's in-process router harness rather than an external HTTP call.
- The available ARC branch still contains other nullable bindings (beginning with `ArcAnswerKey.Register`), so its full SDK closure cannot yet demonstrate PawFS independently. The minimal real-PawFS dependency closure isolates and passes this issue's boundary.

## Limitations

- Temper PR 90 is not merged and TemperPaw remains pinned to the earlier Temper revision. This proof built and tested the proposed PR 90 implementation separately; it does not change the repository pin prematurely.
- This is a draft PR. Genesis publication, installed-ref verification, Railway deployment, and Datadog production verification must occur after dependency-order merge.

## What Still Doesn't Work

- Downstream ARC SDK/WASM regeneration remains blocked until Temper PR 90, this PawFS PR, and ARC's remaining requiredness migrations land in dependency order.
- No production deployment was changed or claimed by this proof.

## Artifacts

- ADR: `os-apps/paw-fs/adrs/004-required-action-parameter-contracts.md`
- Contract test: `crates/temperpaw/tests/paw_fs_action_requiredness.rs`
- Draft PR: https://github.com/nikstern/temperpaw/pull/23
- Temper implementation under test: https://github.com/nerdsane/temper/pull/90

## Architecture Diagram

```text
IOA action contract
        |
        v
exact CSDL action twin -- required by default --> PR 90 schema gate
        |                                           |
        v                                           v
generated typed SDK                         reject before effects
        |
        v
PawFS WASM integration
```
