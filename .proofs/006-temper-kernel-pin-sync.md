# Proof Report: 006 — Temper Kernel Pin Synchronization

## Date

2026-08-26

## Branch / Commit

- Repository: `nikstern/temperpaw` on GitHub
- Worktree: `/private/tmp/temperpaw-issue-6`
- Branch: `codex/temper-kernel-sync`
- Red contract commit: `3b667369`
- Draft PR: <https://github.com/nikstern/temperpaw/pull/10>

## What Was Done

- Added `.temper-kernel.toml` as the canonical allowed Temper repository and
  full immutable revision.
- Added `scripts/sync-temper-kernel <commit>` and `--check`.
- Scoped discovery to configured root, `crates/`, and `os-apps/` manifest and
  lockfile patterns while excluding worktree, cache, generated target, and
  dependency directories.
- Replaced short manifest pins with the canonical 40-character revision.
- Refreshed existing lockfiles with targeted `cargo update --precise` commands.
  The synchronizer never edits Cargo lock source URLs itself.
- Updated existing Rust pin contracts to read the canonical file.
- Added the pin contract to CI and removed the older direct lockfile rewrite
  helper.

This is repository dependency-governance tooling. It does not change a Temper
app, entity state machine, WASM behavior, Cedar policy, trigger, runtime,
deployment behavior, or agent capability surface, so an ADR is not warranted.
That judgment is recorded here as required by the project guide.

## Verification Flow

| Step | Expected | Actual | Status |
|------|----------|--------|--------|
| Red contract | Tests fail before implementation | 3 tests errored because `scripts/sync-temper-kernel` did not exist | Pass |
| Focused synchronizer tests | Update/check, exact failures, multiline TOML, exclusions, malformed revisions, and stale locks work | 6 tests passed | Pass |
| Real repository synchronization | One command updates manifests and delegates lock refresh to Cargo | 93 manifest dependencies synchronized; 49 entries refreshed across 28 lockfiles | Pass |
| Repository check | No mixed, moving, malformed, or stale in-scope pins | `verified 93 manifest dependencies and 49 lockfile entries` | Pass |
| Rust format and lint | Formatting and warnings gates pass | `cargo fmt --all -- --check` and locked clippy with `-D warnings` passed | Pass |
| Full native build | Runtime and worker build against the canonical pin | Locked build of `temperpaw` and `paw-codex-worker` passed | Pass |
| Full native tests | Existing runtime and worker contracts remain green | TemperPaw test suites passed; worker 89/89 passed | Pass |
| Standalone WASM build | Independent locked WASM packaging still works | `context_compactor` release build passed for `wasm32-unknown-unknown` with the repository linker configuration | Pass |
| Binary exercise | Compiled CLI is runnable | `temperpaw-server --help` and `run --help` completed successfully | Pass |

## Verification Results

Commands exercised:

```text
python3 -m unittest scripts.tests.test_sync_temper_kernel -v
scripts/sync-temper-kernel e8ff002bde3e9512385c2856d733210600e7c253
scripts/sync-temper-kernel --check
cargo fmt --all -- --check
cargo clippy --locked -p temperpaw -p paw-codex-worker --all-targets -- -D warnings
cargo build --locked -p temperpaw -p paw-codex-worker
cargo test --locked -p temperpaw --quiet
cargo test --locked -p paw-codex-worker --quiet
RUSTFLAGS='-C link-arg=--allow-undefined' cargo build --locked --manifest-path os-apps/paw-agent/wasm/context_compactor/Cargo.toml --target wasm32-unknown-unknown --release
./target/debug/temperpaw-server --help
./target/debug/temperpaw-server run --help
```

The real synchronization changed only manifest `rev` values and Cargo-generated
lock source selectors from the existing short form to the same resolved full
commit. It did not update dependency versions or the resolved commit.

## What Worked

- A newly added stale standalone WASM manifest fails with its exact path.
- Mixed repositories and moving branches produce separate actionable errors.
- Full-SHA validation catches malformed CLI, canonical, and manifest revisions.
- Multiline inline tables and expanded dependency tables synchronize correctly.
- Existing checked-in lockfiles are refreshed without creating new lockfiles.
- Running `--check` is offline and does not mutate files.

## What Didn't Work

- The first real synchronization was stopped by sandboxed crates.io access. The
  same repository command succeeded with approved network access.
- A direct standalone WASM Cargo invocation initially omitted TemperPaw's
  required unresolved-host-import linker flag and failed at link time. Repeating
  it with the same `RUSTFLAGS` configured by `os-apps/wasm-build-env.sh` passed.
- Python bytecode compilation initially targeted the macOS user cache, which is
  outside the worktree sandbox. Repeating it with a task-specific cache under
  `/private/tmp` passed.

## Limitations

- This pre-merge developer-tooling change has no Genesis app publication,
  Railway deployment, Datadog production signal, or OData state transition to
  verify. No runtime or installable app artifact changed.
- The canonical repository remains `nikstern/temper` because that is the kernel
  lineage currently pinned on `main`. Moving back to `nerdsane/temper` requires
  deliberately changing the canonical repository and then running the same
  synchronizer with the upstream full commit.

## What Still Doesn't Work

- Post-merge CI and the downstream automation that opens kernel-bump PRs cannot
  be verified before this PR merges. The latter remains intentionally deferred,
  as described in issue #6.

## Artifacts

- `.temper-kernel.toml`
- `scripts/sync-temper-kernel`
- `scripts/tests/test_sync_temper_kernel.py`
- `.github/workflows/ci.yml`
- Durable memory decision note `189`

## Architecture Diagram

```text
.temper-kernel.toml
        |
        v
sync-temper-kernel ----> scoped Cargo.toml files
        |
        +--------------> cargo update --precise ----> existing Cargo.lock files
        |
        v
      --check ---------> CI pin contract
```
