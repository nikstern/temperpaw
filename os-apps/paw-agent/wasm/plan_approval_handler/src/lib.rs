//! Plan Approval Handler — WASM integration triggered by Plan.Approve.
//!
//! When a Plan entity is approved (UnderReview/Escalated → Active), this
//! module checks if the Plan has an associated session_id. If the session
//! is in WaitingForApproval state, it dispatches ResumeWithPlanApproval
//! to switch the session back to execute mode with restored tools.
//!
//! Follows the agent_reply cross-entity dispatch pattern.

use temper_wasm_sdk::prelude::*;
use wasm_helpers::entity_field_str;

mod temper_module_sdk;

use temper_module_sdk::{SessionClient, SessionResumeWithPlanApprovalInput};

/// Default tools — must match dispatch.rs and entity_ops.rs.
const DEFAULT_TOOLS_ENABLED: &str = "temper_create,temper_get,temper_list,temper_action,temper_patch,temper_submit_specs,temper_show_spec,temper_specs,temper_upload_wasm,temper_get_trajectories,temper_get_insights,temper_get_decisions,temper_poll_decision,temper_approve_decision,temper_deny_decision,temper_submit_policy,temper_list_policies,temper_get_policy,temper_update_policy,temper_delete_policy,temper_search_apps,temper_install_app,temper_publish_app,temper_update_app,temper_list_apps,temper_spawn_session,temper_list_sessions,temper_abort_session,temper_steer_session,temper_save_memory,temper_recall_memory,temper_write,temper_write_many,temper_read,temper_run_coding_agent,temper_get_secret,temper_datadog_query,temper_railway,temper_vercel,temper_web_search,temper_web_fetch,read,write,edit,bash";

#[unsafe(no_mangle)]
pub extern "C" fn run(_ctx_ptr: i32, _ctx_len: i32) -> i32 {
    let result = (|| -> Result<(), String> {
        let ctx = Context::from_host()?;
        let fields = ctx
            .entity_state
            .get("fields")
            .cloned()
            .unwrap_or_else(|| json!({}));
        // Read session_id from the Plan entity
        let session_id =
            entity_field_str(&fields, &["session_id", "SessionId"]).unwrap_or("");
        if session_id.is_empty() {
            ctx.log(
                "info",
                "plan_approval_handler: no session_id on Plan; skipping",
            );
            set_success_result(
                "",
                &json!({
                    "status": "skipped",
                    "reason": "no_session_id",
                }),
            );
            return Ok(());
        }

        let mut sessions = SessionClient::new();
        let session = sessions
            .get(session_id)
            .map_err(|error| format!("plan_approval_handler: get session {session_id}: {error}"))?;
        let session_status = session.value.status.as_deref().unwrap_or("");
        if session_status != "WaitingForApproval" {
            ctx.log(
                "info",
                &format!(
                    "plan_approval_handler: session {session_id} is {session_status}, not WaitingForApproval; skipping"
                ),
            );
            set_success_result(
                "",
                &json!({
                    "status": "skipped",
                    "reason": "session_not_waiting",
                    "session_status": session_status,
                }),
            );
            return Ok(());
        }

        // Read pre_plan_tools_enabled from the session to restore
        let stashed_tools = session
            .value
            .pre_plan_tools_enabled
            .as_deref()
            .filter(|tools| !tools.is_empty())
            .unwrap_or(DEFAULT_TOOLS_ENABLED);

        sessions
            .resume_with_plan_approval(
                session_id,
                Some(session.sequence),
                SessionResumeWithPlanApprovalInput {
                    session_mode: Some("execute".into()),
                    tools_enabled: Some(stashed_tools.into()),
                    pre_plan_tools_enabled: Some(String::new()),
                },
            )
            .map_err(|error| {
                format!(
                    "plan_approval_handler: ResumeWithPlanApproval for {session_id}: {error}"
                )
            })?;

        ctx.log(
            "info",
            &format!(
                "plan_approval_handler: resumed session {session_id} in execute mode"
            ),
        );
        set_success_result(
            "",
            &json!({
                "status": "resumed",
                "session_id": session_id,
            }),
        );
        Ok(())
    })();

    if let Err(error) = result {
        set_error_result(&error);
    }
    0
}
