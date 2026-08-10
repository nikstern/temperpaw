# Proof Report: 096 — Durable-reaction Temper fork

## Date

2026-08-10

## Branch / Commit

- TemperPaw branch: `codex/durable-reaction-temper-pin`
- Temper review commit: `nikstern/temper@ce34b48d5f6b10585bf2168329eaaabf51d577aa`
- Temper draft PR: `https://github.com/nikstern/temper/pull/1`

## What Was Done

- Pinned every Temper dependency manifest and checked-in lockfile to the exact
  fork commit above.
- Added `scripts/pin-temper-dependencies.py` so future pin changes and lockfile
  synchronization are one deterministic command.
- Added a Turso-backed acceptance test that commits a source reaction intent,
  reconstructs `ServerState`, starts recovery, and observes the target entity
  reach its expected state without redispatching the source action.

This is a development-only validation pin. It must be replaced with an exact
upstream Temper commit before production merge or deployment.

## Verification Flow

| Step | Expected | Actual | Status |
|------|----------|--------|--------|
| Pin check | All manifests and lock entries use one fork SHA | 86 manifest pins and 44 lock entries verified | Pass |
| Locked dependency build | Core TemperPaw crates resolve the fork SHA | `cargo check --locked -p temperpaw -p paw-codex-worker` passed | Pass |
| Restart recovery | Pending source intent survives reconstruction and reaches target | `Payment:order-1` reached `Authorized` from startup recovery | Pass |
| Immutable arm64 image | Image embeds the validation commit and has a stable local ID | `temperpaw-local:758b4fc9-durable-reactions` = `sha256:d8a90921b2a0e16dbefd43a4ab0290146c12d3d460ed5309aedf78b199c025b1`; `BUILD_SHA=758b4fc9` | Pass |
| Platform enforcement | ARC acceptance refuses a runtime image for the wrong architecture | The derived linux/amd64 release build rejected the arm64-only image with `no match for platform in manifest` | Pass (fail closed) |
| Intermediate amd64 image | Runtime is amd64 while retaining assets from the same validated commit | `sha256:16fa36bd4661bdd5fd4e2e0db4451d00544284c4d6035d62aacd4924158c3fff` failed startup because the production Dockerfile omitted required `artifact_batch_apply.wasm` | Fail closed |
| Corrected amd64 image | Runtime is amd64, embeds `BUILD_SHA=758b4fc9`, and includes all required paw-fs WASM | `temperpaw-local:758b4fc9-durable-reactions-amd64-complete` = `sha256:e25dc05d9070ad8358ca5a6444cb273132bd2217379afd12912b2977521c06d1`; bounded `/readyz` smoke test passed | Pass |

## What Worked

- Cargo resolved all Temper crates from the exact fork commit.
- The Turso event journal preserved the atomic source intent across state
  reconstruction.
- Startup recovery dispatched the persisted rule using its original authority.
- `Dockerfile.validation-amd64` reused the exact cached amd64 server layer and
  copied architecture-neutral dashboard/WASM assets from the arm64 image of the
  same commit; the helper validates both input and output build identities.
- The production Dockerfile now builds every app-required paw-fs module. A
  contract test prevents `artifact_batch_apply` from being omitted again.

## What Didn't Work

- Cargo's default libgit transport followed the workstation URL rewrite to the
  SSH alias `github-commercial`, which libgit could not resolve. Verification
  succeeded with `CARGO_NET_GIT_FETCH_WITH_CLI=true`, using the system Git/SSH
  configuration.
- The first immutable image was arm64-only. The ARC acceptance harness requires
  linux/amd64 and rejected it before running the provider-free trial; a separate
  amd64 image is required for that acceptance flow.
- A full QEMU rebuild later failed when GCC segfaulted while linking a proc-macro
  build script for `validate_webhook`. The validation-image helper avoids running
  architecture-neutral WASM builds under QEMU while preserving an amd64 server
  and final runtime.
- The first amd64 composition exposed a pre-existing production packaging gap:
  `artifact_batch_apply` was declared app-required but absent from the image.
  Startup rejected the image before readiness. The Dockerfile fix and corrected
  image passed a bounded readiness smoke test.

## Limitations

- No production deployment or Genesis claim is made from this fork pin.
- The ARC application-level provider-free acceptance flow is recorded in a
  later step of backlog task 96.

## What Still Doesn't Work

- Production must wait for the Temper change to land upstream and for
  TemperPaw to repin to that upstream commit.

## Artifacts

- `scripts/pin-temper-dependencies.py`
- `scripts/build-validation-amd64-image.sh`
- `scripts/rebind-validation-assets.sh`
- `Dockerfile.validation-amd64`
- `Dockerfile.validation-assets`
- `crates/temperpaw/tests/durable_reaction_recovery.rs`
- `temperpaw-local:758b4fc9-durable-reactions`
- `temperpaw-local:758b4fc9-durable-reactions-amd64`
- `temperpaw-local:758b4fc9-durable-reactions-amd64-complete`

## Architecture Diagram

```text
committed Order event + reaction intent
                 |
            process exit
                 |
        reconstructed ServerState
                 |
       durable recovery worker
                 |
     Payment.Authorize + receipt
                 |
        Payment = Authorized
```
