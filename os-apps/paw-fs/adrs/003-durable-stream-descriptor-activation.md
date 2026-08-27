# ADR-003: Durable Stream Descriptor Activation

## Status

Accepted

## Context

PawFS currently exposes `File.SizeBytes` and `FileVersion.SizeBytes` as
application properties. Those values are useful to the PawFS state machines,
but they are not a trustworthy platform admission fact for typed stream reads:
they are supplied through application transitions and are not atomically bound
to the accepted object bytes, digest, storage receipt, or journal sequence.

Temper ADR-0188 and PR #68 add `StreamDescriptorV1`, whose host-attested
length, digest, storage identity, relationship, and publication sequence are
committed as reserved kernel event metadata. The contract remains inert unless
the installed application CSDL explicitly selects version 1. PawFS therefore
continues to use the legacy read path even on a descriptor-aware kernel.

## Decision

PawFS adopts Temper's closed stream vocabulary:

- `File` is a mutable direct stream whose immutable versions are reached via
  `Versions` and have entity type `Paw.FS.FileVersion`.
- `FileVersion` is an immutable stream authorized through its `File` parent.
- Both entities select `Temper.Vocab.Stream.DescriptorContractVersion=1` in
  the activated application schema.

All Temper host and guest dependencies move together to the exact reviewed
Temper revision containing the descriptor implementation. Generated module
SDK artifacts remain bound to the verified stream capability digest.

Activation is allowed only for a tenant whose bounded historical inventory has
completed with no unresolved File or FileVersion streams. Historical content
is backfilled through Temper's idempotent, durable migration pages. Application
fields, including `SizeBytes`, are never aliases or fallback authority for the
descriptor.

When installed-app reconciliation finds that migration evidence is incomplete,
TemperPaw enters an unready maintenance mode instead of activating PawFS or
exiting. Liveness and the authenticated governed stream-descriptor migration
routes remain available; normal application routes remain startup-gated. After
the migration completes, an operator restarts TemperPaw and reconciliation
rechecks the durable completion evidence before activation.

## Consequences

- New `$value` writes commit a host-attested descriptor with the corresponding
  File transition, and version fan-out carries typed descriptor provenance.
- Typed and OData reads enforce length budgets and descriptor integrity before
  object fetch or allocation.
- Current and versioned content remain readable after durable restart without
  inferring length from PawFS field names.
- A missing, stale, corrupt, conflicting, or over-budget descriptor fails
  closed.
- Deployments with historical streams must finish and retain migration evidence
  before installing the activated PawFS schema. Rollback binaries must preserve
  descriptor metadata even if the activation marker is removed.
- A migration-required deployment remains observable and operable without
  advertising readiness or exposing ordinary application traffic.

## Alternatives Rejected

- **Teach Temper the `SizeBytes` alias:** this would make an application naming
  convention part of the kernel ABI and would not bind length to stored bytes.
- **Leave PawFS on the legacy reader:** generated clients would remain unable to
  make an artifact-bound, pre-fetch admission decision after restart.
- **Synthesize descriptors from application state at read time:** this would
  manufacture authority without verifying the object, digest, ownership, or
  publication sequence.
