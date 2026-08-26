# ADR-040: Rebuild the Locked Kernel SDK Lineage

- Status: Accepted
- Date: 2026-08-26
- Related:
  - Temper ADR-0184: Grant-Scoped Module SDK Surface
  - Temper ADR-0185: Canonical Schema Default Materialization
  - Temper ADR-0186: Canonical Property Provenance
  - TemperPaw ADR-029: Pin Temper Bounded Query Probes

## Context

Temper commit `e8ff002bde3e9512385c2856d733210600e7c253` changes the
kernel lineage that generates and enforces module SDK bindings. The lineage
includes grant-scoped generated APIs, canonical schema defaults, canonical
property provenance, and restoration of serialized bindings without workspace
sources.

TemperPaw previously pinned its host crates and packaged guest WASM SDK
manifests to `0ee5cf195434bd35fa843f3ff4118095ef0a9b36`. Reusing WASM
artifacts built from that older guest SDK while moving only the host would leave
the deployed artifact set on a mixed lineage immediately before exercising the
source-free restart path.

## Decision

Pin every TemperPaw host dependency and every checked-in guest
`temper-wasm-sdk` manifest and lockfile to the latest `nikstern/temper` main
commit at the time of this rollout:
`e8ff002bde3e9512385c2856d733210600e7c253`.

Rebuild the complete deployment WASM set from those locked manifests before a
source-free restart is attempted. The repository-wide pin contract and helper
must reject any mixed owner, branch, tag, short revision, or resolved commit.
WASM binaries remain build outputs rather than checked-in source artifacts;
CI and the production image rebuild them from the committed locks.

## Consequences

Positive:

- The host kernel and guest modules share one immutable SDK lineage.
- Generated grant surfaces and canonical binding restoration are exercised
  against artifacts compiled from the kernel revision that will restore them.
- A partial pin cannot pass the repository-wide dependency contract.

Tradeoffs:

- The rollout rebuilds the full OS-app WASM surface, even where guest source did
  not otherwise change.
- The pin is a snapshot. A later Temper main commit requires another explicit
  pin, lock refresh, artifact rebuild, and deployment verification.

## Verification

- Run the pin helper in `--check` mode for the full 40-character commit.
- Run the focused Temper dependency contract and locked host checks.
- Build every WASM script used by CI and the production Docker image, then run
  the route-message artifact verifier.
- Build and start TemperPaw, exercise an installed app, and verify entity state
  transitions through OData before the source-free restart is accepted.
- After merge, publish through Genesis, deploy to Railway, and use Datadog to
  verify the exact live TemperPaw/Temper revisions and restart behavior.

## Rollback Policy

Roll back the host manifests, guest manifests, lockfiles, and rebuilt deployment
image together. Do not combine a host rollback with WASM artifacts from a
different SDK lineage.
