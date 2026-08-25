# ADR-039: Action Trigger Module Packaging

- Status: Accepted
- Date: 2026-08-25
- Scope: `paw-agent`
- Related: ADR-004, task 62

## Context

`Session.PauseForPlanApproval` names `request_plan_review` in an inline WASM
trigger. The module source and compiled artifact existed, but `paw-agent` did
not declare the module in `app.toml`. Canonical installation packages only
declared modules, so the transition committed and its required reaction then
failed because the runtime could not resolve the module.

An artifact's presence in the source tree is not deployment intent. The app
manifest is the canonical module inventory.

## Decision

Every WASM module referenced by a `paw-agent` action trigger must also appear in
the app's `[[wasm_modules]]` declarations. `request_plan_review` is declared as
an app-required, lazily loaded module because plan-review notification is part
of the approval-gated Session state machine but is not needed during startup.

A contract test parses the app manifest and requires this declaration for the
`PauseForPlanApproval` path.

## Consequences

- Immutable bundles include the reaction module needed by the declared state
  transition.
- Missing packaging fails during contract verification instead of after a live
  Session enters `WaitingForApproval`.
- The module remains lazy, avoiding startup compilation cost until the approval
  path is exercised.

## Alternatives Considered

- Discover every artifact directory automatically: rejected because it makes
  undeclared source files part of the deployed capability surface.
- Make the notification optional: rejected because silent loss of the human
  approval request breaks the standalone Discord operating model.

## Rollback

Revert the declaration and contract together. Doing so restores the known
runtime failure on `PauseForPlanApproval` and is not safe while the trigger is
present.
