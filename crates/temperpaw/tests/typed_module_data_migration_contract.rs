use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("temperpaw crate should be nested beneath the workspace root")
        .to_path_buf()
}

#[test]
fn plan_approval_handler_uses_its_generated_typed_data_client() {
    let root = workspace_root();
    let module_dir = root.join("os-apps/paw-agent/wasm/plan_approval_handler");
    let source = fs::read_to_string(module_dir.join("src/lib.rs"))
        .expect("plan approval handler source should exist");
    let generated = module_dir.join("src/temper_module_sdk.rs");
    let manifest = fs::read_to_string(root.join("os-apps/paw-agent/app.toml"))
        .expect("paw-agent app manifest should exist");
    let plan_review = fs::read_to_string(root.join("os-apps/paw-agent/specs/plan_review.ioa.toml"))
        .expect("plan review spec should exist");
    let approve_trigger = plan_review
        .split("[[action.triggers]]\nname = \"plan_approved_switch\"")
        .nth(1)
        .expect("Approve should declare the plan approval trigger")
        .split("[[action]]")
        .next()
        .expect("Approve trigger section should be bounded by the next action");

    assert!(
        generated.is_file(),
        "the leaf module must check in the SDK generated from its locked app closure"
    );
    assert!(
        source.contains("mod temper_module_sdk;"),
        "the leaf module must compile against its generated typed client"
    );
    assert!(
        manifest.contains("name = \"plan_approval_handler\"")
            && manifest.contains("[wasm_modules.data]"),
        "the app manifest must declare the module and its least-privilege data grant"
    );

    for forbidden in [
        "resolve_temper_api_url",
        "runtime_headers",
        ".http_call(",
        "/tdata/Sessions",
    ] {
        assert!(
            !source.contains(forbidden),
            "internal Temper access must use the typed host ABI, not {forbidden}"
        );
    }
    assert!(
        !approve_trigger.contains("temper_api_url")
            && !approve_trigger.contains("[action.triggers.config]"),
        "the typed data client must not retain obsolete loopback HTTP trigger configuration"
    );
}

#[test]
fn paw_agent_csdl_includes_the_project_entity_from_adr_0018() {
    let csdl = fs::read_to_string(workspace_root().join("os-apps/paw-agent/specs/model.csdl.xml"))
        .expect("paw-agent CSDL should exist");

    for expected in [
        "<EntityType Name=\"Project\">",
        "<Action Name=\"Configure\" IsBound=\"true\">",
        "<EntitySet Name=\"Projects\" EntityType=\"TemperPaw.Project\"/>",
    ] {
        assert!(
            csdl.contains(expected),
            "ADR-0018 Project metadata is missing from paw-agent CSDL: {expected}"
        );
    }
}

#[test]
fn cron_spawn_uses_a_session_initializer_scoped_to_cron_owned_fields() {
    let root = workspace_root();
    let cron = fs::read_to_string(root.join("os-apps/paw-agent/specs/cron_job.ioa.toml"))
        .expect("cron job IOA should exist");
    let session = fs::read_to_string(root.join("os-apps/paw-agent/specs/session.ioa.toml"))
        .expect("session IOA should exist");

    assert!(
        cron.contains("initial_action = \"ConfigureScheduledRun\""),
        "CronJob must not invoke the general Session.Configure action with unmapped fields"
    );
    assert!(
        session.contains("name = \"ConfigureScheduledRun\"")
            && session.contains("params = [\"system_prompt\", \"user_message\", \"model\", \"provider\", \"provider_options_json\", \"temperature\", \"max_turns\", \"tools_enabled\", \"soul_id\", \"sandbox_url\"]"),
        "Session must expose a cron-scoped initializer with exactly the fields CronJob owns"
    );
}
