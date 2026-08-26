# ADR-002: Honest Optional File Metadata

## Status

Accepted

## Context

The PawFS CSDL declared `File.DirectoryId`, `File.CreatedAt`, and
`File.UpdatedAt` as non-nullable. The `File` automaton does not declare durable
state for any of them, and Temper collection creation persists caller fields
without deriving timestamps. PawFS creators are also not uniform:
`temper.write` supplies a directory ID, image generation can omit it, and older
callers used lowercase field names.

Temper's generated module-data client therefore correctly treated the three
properties as required while persisted File entities could truthfully omit
them. After a persistent restart, canonical typed metadata reads failed with
`MissingRequiredProperty`. Fabricating timestamps or a default GUID would make
the client decode but would give security-sensitive consumers false metadata.

`WorkspaceId` is different: it is required by the PawFS ownership model and is
used by consumers such as ARC to enforce workspace isolation.

## Decision

Declare `File.DirectoryId`, `File.CreatedAt`, and `File.UpdatedAt` nullable in
the PawFS CSDL. Keep `File.WorkspaceId` non-nullable.

Generated typed clients must expose the three genuinely optional properties as
`Option` values. Persisted entities that omit them decode as `None`; PawFS does
not synthesize timestamps or sentinel GUIDs.

## Consequences

- Existing Files remain readable through generated typed clients after
  persistent restart.
- Current creation paths can continue to omit metadata that PawFS does not yet
  durably own.
- Workspace ownership stays mandatory and available to authorization checks.
- Consumers must handle absent directory and timestamp metadata explicitly.

A future Temper-wide migration may derive timestamps from durable event
metadata and normalize or enforce `DirectoryId` for every creator. That work
requires a coordinated persistence and creator migration and is deliberately
not part of this contract repair.

## Alternatives Rejected

- **Default timestamps or GUIDs:** generated values would look authoritative
  while describing no real durable fact.
- **Add fields only to the File automaton:** timestamps require a runtime-owned
  event-time source, and DirectoryId still needs every creator normalized.
- **Drop typed File metadata reads:** security-sensitive consumers need
  `WorkspaceId` to validate ownership before reading content.
