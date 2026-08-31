# ADR-004: Required Action Parameter Contracts

- Status: Accepted
- Date: 2026-08-31
- Scope: `paw-fs`
- Related: Temper ADR-0193, TemperPaw issue #22

## Context

Temper's required-by-default IOA contract makes a plain action parameter
required and reserves `nullable = true` for intentional absence. OData CSDL
has the opposite default: omitting `Nullable` makes a parameter nullable.

The PawFS IOA and CSDL contracts predate that distinction. Every bound action
binding is implicitly nullable, several non-binding parameters are implicitly
nullable despite being used by state or trigger logic, and 16 callable IOA
actions have no exact entity/action CSDL twin. These gaps let generated clients
and runtime validation disagree about the PawFS action ABI.

## Decision

Every callable PawFS IOA action has an exact bound CSDL twin. Every binding is
non-nullable. Plain IOA parameters remain required and their CSDL parameters
declare `Nullable="false"`.

PawFS has exactly three intentionally nullable action inputs:

- `Directory.Create.parent_id`: a root directory has no parent.
- `WorkspaceUsageBucket.Create.artifact_batch_id`: a time-bounded usage bucket
  need not originate from an artifact batch.
- `WorkspaceUsageBucket.ApplyDelta.artifact_batch_id`: the same optional
  provenance applies when recording a delta.

These parameters use typed IOA declarations with `nullable = true` and matching
nullable CSDL parameters. None is consumed by a guard, state-mutating effect,
spawn identity, trigger mapping, or template substitution.

Inputs that affect state, lifecycle completion, provenance, trigger payloads,
or filesystem operation behavior remain required. PawFS does not retain
implicit defaults as an absence contract.

## Consequences

- Generated SDKs expose required inputs as values and the three intentional
  absences as options.
- Missing or explicit-null required inputs are rejected before a transition,
  event append, trigger, lifecycle change, state mutation, or external effect.
- Nullable-to-required schema upgrades are breaking and must regenerate their
  typed SDK closures and WASM artifacts.
- The legacy FUSE-bound action aliases remain for compatibility, but exact
  callable IOA twins are authoritative for verification.
