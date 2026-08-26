# Proof Report: 040 — Kernel SDK Lineage Rebuild

## Date

2026-08-26

## Branch / Commit

- Repository: `nikstern/temperpaw` on GitHub
- Worktree: `/private/tmp/temperpaw-kernel-lineage-rebuild`
- Branch: `codex/kernel-lineage-rebuild`
- Base: `origin/main` at `ae3016d32992f51a92e377c5cb2e89a70a3b9d71`
- Temper lineage: `nikstern/temper` `main` at `e8ff002bde3e9512385c2856d733210600e7c253`
- Initial dependency-pin commit: `1e84ee48ca0e52007b9b5d1cff8f20e4bdbb84c8`

## What Was Done

- Pinned all host and guest Temper dependencies and their Cargo locks to the
  latest `nikstern/temper` main commit.
- Rebuilt every production WASM module using the shared locked WASM build
  environment.
- Made the Session lifecycle provenance explicit by matching the CSDL `Status`
  default to the IOA initial state, `Created`.
- Regenerated the `plan_approval_handler` typed SDK and closure lock with the
  exact new Temper CLI, rebuilt the leaf artifact, and rebound its manifest.
- Updated repository contract tests that intentionally assert the active
  Temper revision.

## Verification Flow

1. Confirmed the remote `nikstern/temper` `main` SHA with `git ls-remote`.
2. Ran the dependency contract before changing the pin and observed the old
   revision expectation (red); updated the pin and made the focused contract
   green.
3. Ran the full TemperPaw suite and observed that the old typed binding no
   longer parsed under the new kernel because binding properties lacked
   provenance. The first SDK generation then rejected Session because its
   lifecycle property had no canonical default (red).
4. Added `DefaultValue="Created"`, regenerated and rebound the typed module,
   then ran both generator and binder in `--check` mode (green).
5. Built all production WASM modules and verified the route-message artifact.
6. Built and tested both runtime crates with locked dependencies.
7. Installed the locally locked `paw-agent` closure into an isolated latest
   Temper server and captured health, metadata, and module inventory.
8. Stopped that server, changed the working directory to `/private/tmp`,
   removed `TEMPER_OS_APPS_DIR` from the launch environment, and restarted only
   from the persisted data directory. Queried health, OData metadata, Sessions,
   and the module inventory after the source-free restore.

## Verification Results

| Step | Expected | Actual | Status |
|------|----------|--------|--------|
| Pin contract | Every Temper manifest/lock uses one immutable revision | 88 manifest pins and 49 lock source entries resolve to `e8ff002b` | Pass |
| Typed SDK generate check | Checked-in SDK matches current schema/grants | Closure `sha256:124bb3273c5580f498480f4fd854760fd2777fd0f10946c2c7274f32c53807c2` | Pass |
| Typed SDK bind check | Manifest binding matches rebuilt artifact | Artifact `0c77706566e15ee28d37bda7c1eec5aebe05f8783f4487fe3282ab1d551aa3db` | Pass |
| Production WASM builds | All release modules compile under the new guest SDK | All 12 repository build scripts completed | Pass |
| Route-message verifier | Rebuilt route artifact matches source | SHA-256 `e3c87a950008fad4eda1985896880e6d40e44afe6cdf849437b7a3dd189f6204`, 545098 bytes | Pass |
| TemperPaw tests | Locked full suite is green | `cargo test --locked -p temperpaw -- --test-threads=1` passed | Pass |
| Worker tests | Locked full suite is green | 89 tests passed | Pass |
| Full build | Both shipping binaries compile | `cargo build --locked -p temperpaw -p paw-codex-worker` passed | Pass |
| Locked install | Latest kernel accepts the staged closure | Installed bundle `sha256:a0634b1b6698ea4188c7b1e58223864a143f77ceaf53ed814e029216f2b12bbb` | Pass |
| Source-free restart | Persisted app restores without workspace source | `Restored app cache roots: local=1`; server listened on port 31286 | Pass |
| Restored API | Schema and governed read model remain available | health 200, metadata 200, `TemperPaw.Session` present, Sessions read 200 | Pass |
| Restored artifact identity | Same bound artifact is registered after restart | 18 modules; `plan_approval_handler` SHA unchanged | Pass |

## What Worked

- The latest kernel's provenance validation caught the stale generated binding
  before the restart attempt.
- Grant-scoped generation reduced the checked-in client to only operations the
  module is allowed to use.
- The installed bundle restored from the data directory without a TemperPaw
  source/catalog path.

## What Didn't Work

- A direct Session create using the isolated operator token returned 403. This
  is expected: the Session Cedar policy permits creation by human, supervisor,
  system, and agent principals, not the operator principal. The policy was not
  weakened for verification.

## Limitations

- This proof covers the local source-free restart boundary. The draft PR is not
  merged, so Genesis publication, pinned-ref verification, Railway deployment,
  and production Datadog confirmation remain post-merge work.
- Build outputs for modules other than the artifact-bound typed module remain
  ignored local artifacts and are rebuilt by CI/image construction.

## What Still Doesn't Work

- No regression remains in the verified local scope. Production verification
  cannot begin until this change is reviewed, merged, and published to Genesis.

## Artifacts

- ADR: `os-apps/paw-agent/adrs/040-rebuild-kernel-sdk-lineage.md`
- Typed closure lock: `os-apps/paw-agent/temper-module-sdk.lock`
- Generated client: `os-apps/paw-agent/wasm/plan_approval_handler/src/temper_module_sdk.rs`
- Bound manifest: `os-apps/paw-agent/app.toml`

## Architecture Diagram

```text
nikstern/temper@e8ff002b
        |
        +-- host Cargo pins + locks
        +-- guest Cargo pins + locks
        +-- typed SDK generator ----> closure lock + generated Rust
        +-- wasm32 compiler --------> rebuilt WASM artifact
                                      |
                                      v
                              locked app bundle
                                      |
                         persist, remove source path
                                      |
                                      v
                         source-free kernel restart
                         schema + WASM hash restored
```
