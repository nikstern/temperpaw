# ADR-038: Cron-scoped Session initializer

## Status

Accepted

## Context

`CronJob.TriggerComplete` spawned `Session` and invoked the general `Configure`
action while providing only the fields owned by a scheduled job. Once Temper
preserved nested action behavior in canonical IOA, closure validation correctly
rejected the unmapped general-session parameters.

Adding unrelated workspace, channel, compaction, and trace fields to `CronJob`
would couple the scheduler to the full Session creation surface.

## Decision

`Session` exposes `ConfigureScheduledRun` from `Created`. It accepts exactly the
scheduled-run fields owned by `CronJob` and schedules the same
`ProvisionWorkspace` transition as the general initializer.

`CronJob.TriggerComplete` uses this initializer for its declarative spawn.

## Consequences

- The spawn contract is complete and can be understood from entity transitions.
- New general Session configuration fields do not automatically become CronJob
  state.
- Changes to scheduled-run inputs must update both entities and their CSDL
  metadata deliberately.
