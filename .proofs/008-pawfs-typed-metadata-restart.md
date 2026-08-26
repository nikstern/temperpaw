# Proof Report: 008 — PawFS Typed Metadata Restart

## Date

2026-08-26

## Branch / Commit

- Repository: `nikstern/temperpaw` on GitHub via `github-commercial`
- Worktree: `/private/tmp/temperpaw-issue-8`
- Branch: `codex/pawfs-nullable-metadata`
- Draft PR: https://github.com/nikstern/temperpaw/pull/9

## What Was Done

PawFS `File.DirectoryId`, `File.CreatedAt`, and `File.UpdatedAt` now reflect the
durable contract as nullable. `File.WorkspaceId` remains required. A generated
Temper `FileClient` regression covers supported File creation, `$value`
content upload, Turso persistence, restart rehydration, canonical module-data
metadata, and typed decoding.

## Verification Flow

1. Generate and compile `FileClient` from the checked-in PawFS CSDL and File
   IOA using the pinned Temper code generator.
2. Create the workspace relation fixture and POST a File through
   `/tdata/Files`, intentionally omitting directory and timestamp metadata.
3. Upload content through `/tdata/Files('<id>')/$value`.
4. Drop the original server state and rebuild it against the same Turso store.
5. Read the uploaded bytes back through `$value` after restart.
6. Rehydrate the File, canonicalize it through Temper's production
   module-data schema path, and read it with the generated `FileClient`.
7. Assert that `WorkspaceId` is present and non-nullable while `DirectoryId`,
   `CreatedAt`, and `UpdatedAt` decode as `None`.

## Verification Results

| Step | Expected | Actual | Status |
|------|----------|--------|--------|
| Red regression | Typed restart read exposes the inconsistent contract | `SchemaMismatch / MissingRequiredProperty` | Pass |
| Supported write path | POST File and PUT `$value` succeed | HTTP 201 and 204 | Pass |
| Persistent content | Uploaded bytes survive state rebuild | Restarted GET `$value` returned `typed restart proof` | Pass |
| Focused green regression | Generated `FileClient` decodes restarted metadata | 1 passed | Pass |
| PawFS contracts | Existing PawFS hot-path coverage remains green | 16 passed | Pass |
| Formatting | Repository formatting is clean | `cargo fmt --all --check` passed | Pass |
| Workspace build/check | All targets compile | `cargo check --locked --workspace --all-targets` passed | Pass |
| Strict lint | Workspace has no warnings under strict clippy | `cargo clippy --locked --workspace --all-targets -- -D warnings` passed | Pass |
| Full tests | Workspace suite remains green | 471 passed, 0 failed | Pass |
| Runtime server boot | Startup TCP server accepts requests before transport boot | `spawn_runtime_server_accepts_requests_before_transport_boot` passed | Pass |

## What Worked

- The pre-fix test reproduced the exact typed-client error reported in issue
  #8 after a real persistent restart.
- The schema repair alone made the same generated-client flow pass.
- No timestamp or directory identifier was fabricated.

## What Didn't Work

- GitHub rejected the first draft-PR attempt before the red commit because the
  branch had no commits relative to `main`; the draft opened immediately after
  the red commit was pushed.

## Limitations

This change does not derive timestamps from event metadata or normalize
`DirectoryId` across all PawFS creators. ADR-002 records those as a broader
future Temper migration. Genesis publishing, Railway deployment, and Datadog
production confirmation are post-merge gates and were not performed on this
draft PR.

## What Still Doesn't Work

Files created without directory or timestamp metadata continue to lack those
facts. Typed clients now represent that absence honestly instead of rejecting
the whole entity.

## Artifacts

- Issue: https://github.com/nikstern/temperpaw/issues/8
- Draft PR: https://github.com/nikstern/temperpaw/pull/9
- ADR: `os-apps/paw-fs/adrs/002-honest-optional-file-metadata.md`
- Regression: `crates/temperpaw/tests/paw_fs_typed_restart.rs`

## Architecture Diagram

```text
POST File -> PUT $value -> Turso journal -> rebuilt ServerState
                                             |
                                             v
                                  canonical module-data response
                                             |
                                             v
                              generated FileClient -> Option fields
```
