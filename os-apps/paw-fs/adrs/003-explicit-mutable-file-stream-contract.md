# ADR-003: Explicit Mutable File Stream Contract

- Status: Accepted
- Date: 2026-08-27
- Deciders: TemperPaw maintainers
- Related: TemperPaw issue #11, Temper ADR-0196

## Context

`Paw.FS.File` has always exposed a directly writable media stream through
`HasStream="true"`. The upgraded Temper verifier requires every streamed
entity to declare whether that stream is mutable or immutable. Without the
declaration, the app closure fails closed during code generation and install.

## Decision

Declare `Paw.FS.File` as `Temper.Vocab.Stream.Mutability = "Mutable"` in its
CSDL metadata. This describes the existing `$value` upload behavior; it does
not introduce a new state transition, storage model, or authorization path.

The existing `FileVersion` entity remains the application's historical record.
It is not declared as the immutable backing stream for `File` because it does
not itself own a media stream.

## Consequences

- Temper can verify the Paw FS stream contract during local and bundled app
  installation.
- Generated typed clients and module SDK closures agree on stream semantics.
- Future changes from mutable uploads to immutable replacement objects require
  a separate migration and ADR.

## Verification

- A contract test requires the exact mutability annotation.
- The canonical Temper module SDK generator validates the complete local app
  closure.
- The TemperPaw server installs Paw FS and reaches readiness.
