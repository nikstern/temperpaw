# ADR-039: Action Trigger Module Packaging

- Status: Accepted
- Date: 2026-08-25
- Scope: `paw-agent`
- Related: ADR-004, task 62

## Context

`Session.PauseForPlanApproval` and `Session.PauseForApproval` name
`request_plan_review` and `request_approval` in inline WASM triggers. Their
module sources and compiled artifacts existed, but `paw-agent` did not declare
them in `app.toml`. Canonical installation packages only declared modules, so
the transitions committed and their required reactions then failed because the
runtime could not resolve the modules.

The typed `plan_approval_handler` also invokes a Session action through the
host-data ABI as the bound module principal. The previous Cedar policy only
authorized role-bearing human and system principals for
`ResumeWithPlanApproval`, so an otherwise valid typed callback had no matching
permit.

An artifact's presence in the source tree is not deployment intent. The app
manifest is the canonical module inventory.

## Decision

Every WASM module referenced by a `paw-agent` action trigger must also appear in
the app's `[[wasm_modules]]` declarations. `request_plan_review` and
`request_approval` are declared as app-required, lazily loaded modules because
approval notification is part of the gated Session state machine but is not
needed during startup.

The Session Cedar policy grants `Agent::"plan_approval_handler"` exactly
`ResumeWithPlanApproval` on Session resources. The manifest's artifact-bound
data grant independently limits that module to the same action plus the one
Session read required to decide whether resumption is valid.

Contract tests parse the app manifest and policy and require these declarations
for both pause paths and the typed callback.

## Consequences

- Immutable bundles include the reaction modules needed by the declared state
  transitions.
- Missing packaging fails during contract verification instead of after a live
  Session enters `WaitingForApproval`.
- The modules remain lazy, avoiding startup compilation cost until an approval
  path is exercised.
- Typed plan approval is authorized without granting the module any unrelated
  Session action.

## Alternatives Considered

- Discover every artifact directory automatically: rejected because it makes
  undeclared source files part of the deployed capability surface.
- Make the notification optional: rejected because silent loss of the human
  approval request breaks the standalone Discord operating model.

## Rollback

Revert the declaration and contract together. Doing so restores the known
runtime failure on `PauseForPlanApproval` and is not safe while the trigger is
present.
