# Proof Report: 096 — Durable-reaction Temper fork

## Date

2026-08-05

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

## What Worked

- Cargo resolved all Temper crates from the exact fork commit.
- The Turso event journal preserved the atomic source intent across state
  reconstruction.
- Startup recovery dispatched the persisted rule using its original authority.

## What Didn't Work

- Cargo's default libgit transport followed the workstation URL rewrite to the
  SSH alias `github-commercial`, which libgit could not resolve. Verification
  succeeded with `CARGO_NET_GIT_FETCH_WITH_CLI=true`, using the system Git/SSH
  configuration.

## Limitations

- No production deployment or Genesis claim is made from this fork pin.
- The immutable development image and ARC application-level acceptance flow are
  recorded in later steps of backlog task 96.

## What Still Doesn't Work

- Production must wait for the Temper change to land upstream and for
  TemperPaw to repin to that upstream commit.

## Artifacts

- `scripts/pin-temper-dependencies.py`
- `crates/temperpaw/tests/durable_reaction_recovery.rs`

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
