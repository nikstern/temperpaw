use std::fs;
use std::process::Command;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use pawfs_typed_client_fixture::{
    DataResponseV1, DataResultV1, FileClient, GENERATED_SOURCE, MANIFEST_JSON, ModuleSdkManifest,
    install_native_data_host_for_test, take_native_data_requests_for_test,
};
use temper_platform::PlatformState;
use temper_platform::os_apps::{
    OsAppReconcileResult, reconcile_os_app, reload_os_apps, set_os_apps_dir,
};
use temper_runtime::ActorSystem;
use temper_runtime::tenant::TenantId;
use temper_server::registry::{
    EntityLevelSummary, EntityVerificationResult, SpecRegistry, VerificationStatus,
};
use temper_server::request_context::AgentContext;
use temper_server::state::DispatchExtOptions;
use temper_server::{ServerState, StorageStack, build_router};
use temper_spec::csdl::parse_csdl;
use temper_store_turso::TursoEventStore;
use temper_wasm_sdk::data::{DataOperationKind, FileOperationKind};
use temper_wasm_sdk::schema_deployment::{
    AdvanceStreamDescriptorMigrationRequestV1, StartStreamDescriptorMigrationRequestV1,
    StreamDescriptorMigrationBudgetsV1, StreamDescriptorMigrationTargetV1,
};
use tower::ServiceExt;

const PAWFS_CSDL: &str = include_str!("../../../os-apps/paw-fs/specs/model.csdl.xml");
const FILE_IOA: &str = include_str!("../../../os-apps/paw-fs/specs/file.ioa.toml");
const FILE_VERSION_IOA: &str = include_str!("../../../os-apps/paw-fs/specs/file_version.ioa.toml");
const WORKSPACE_IOA: &str = include_str!("../../../os-apps/paw-fs/specs/workspace.ioa.toml");
const TEMPER_REVISION: &str = "e3dfe852e7a7373cef8bddfe2e3b8bcad8f94a0a";

async fn authenticate(
    mut request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    request
        .extensions_mut()
        .insert(temper_authz::AuthenticatedRequestContext::new(
            TenantId::default(),
            temper_authz::SecurityContext::system(),
        ));
    next.run(request).await
}

fn legacy_pawfs_csdl() -> String {
    PAWFS_CSDL.replace(
        "        <Annotation Term=\"Temper.Vocab.Stream.DescriptorContractVersion\" Int=\"1\"/>\n",
        "",
    )
}

fn proof_file_ioa() -> String {
    format!(
        r#"{FILE_IOA}

[[action]]
name = "RunStreamReadProof"
kind = "input"
from = ["Ready"]
to = "Ready"
effect = "trigger pawfs_stream_read_proof"

[[action]]
name = "StreamReadProofPassed"
kind = "input"
from = ["Ready"]
to = "Locked"

[[action]]
name = "StreamReadProofFailed"
kind = "input"
from = ["Ready"]
to = "Archived"

[[integration]]
name = "pawfs_stream_read_proof"
trigger = "pawfs_stream_read_proof"
type = "wasm"
module = "pawfs_restart_regression"
on_success = "StreamReadProofPassed"
on_failure = "StreamReadProofFailed"
"#
    )
}

fn build_state(
    name: &str,
    store: TursoEventStore,
    data_dir: &std::path::Path,
    csdl: &str,
    file_ioa: &str,
) -> ServerState {
    let mut registry = SpecRegistry::new();
    registry.register_tenant(
        TenantId::default().as_str(),
        parse_csdl(csdl).expect("PawFS CSDL parses"),
        csdl.to_string(),
        &[
            ("File", file_ioa),
            ("FileVersion", FILE_VERSION_IOA),
            ("Workspace", WORKSPACE_IOA),
        ],
    );
    let mut state = ServerState::from_registry(ActorSystem::new(name), registry);
    state.set_storage_stack(StorageStack::from_turso(store));
    state.data_dir = data_dir.to_path_buf();
    state
        .authz
        .reload_tenant_policies(
            TenantId::default().as_str(),
            "permit(principal, action, resource);",
        )
        .expect("PawFS regression policy parses");
    state.registry.write().unwrap().set_verification_status(
        &TenantId::default(),
        "File",
        VerificationStatus::Completed(EntityVerificationResult {
            all_passed: true,
            levels: vec![EntityLevelSummary {
                level: "L0".into(),
                passed: true,
                summary: "PawFS restart fixture".into(),
                details: None,
            }],
            verified_at: "2026-08-26T00:00:00Z".into(),
        }),
    );
    state
}

fn app(state: ServerState) -> axum::Router {
    build_router(state).layer(axum::middleware::from_fn(authenticate))
}

#[tokio::test]
async fn typed_file_client_reads_current_and_version_content_after_migration_and_restart() {
    let temp = tempfile::tempdir().expect("PawFS restart data directory");
    let database_url = format!("file:{}", temp.path().join("pawfs.db").display());
    let store = TursoEventStore::new(&database_url, None)
        .await
        .expect("persistent Turso store initializes");
    let file_id = "018f1f80-7b2d-7000-8000-000000000008";
    let workspace_id = "018f1f80-7b2d-7000-8000-000000000009";

    let legacy_csdl = legacy_pawfs_csdl();
    let state = build_state(
        "pawfs-before-migration",
        store.clone(),
        temp.path(),
        &legacy_csdl,
        FILE_IOA,
    );
    state
        .get_or_create_tenant_entity(
            &TenantId::default(),
            "Workspace",
            workspace_id,
            serde_json::json!({"Name": "typed-restart"}),
        )
        .await
        .expect("workspace relation fixture creates");
    let response = app(state.clone())
        .oneshot(
            Request::post("/tdata/Files")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "Id": file_id,
                        "Name": "typed-restart.txt",
                        "Path": "/typed-restart.txt",
                        "WorkspaceId": workspace_id,
                        "MimeType": "text/plain"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .expect("PawFS File create responds");
    let create_status = response.status();
    let create_body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("PawFS File create body reads");
    assert_eq!(
        create_status,
        StatusCode::CREATED,
        "PawFS File create failed: {}",
        String::from_utf8_lossy(&create_body)
    );

    let response = app(state.clone())
        .oneshot(
            Request::put(format!("/tdata/Files('{file_id}')/$value"))
                .header("content-type", "text/plain")
                .body(Body::from("typed restart proof"))
                .unwrap(),
        )
        .await
        .expect("PawFS $value write responds");
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let mut version_id = None;
    for _ in 0..256 {
        let persisted = state
            .get_tenant_entity_state(&TenantId::default(), "File", file_id)
            .await
            .expect("File remains readable while version fan-out settles");
        version_id = persisted.state.fields["last_version_id"]
            .as_str()
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        if version_id.is_some() {
            break;
        }
        tokio::task::yield_now().await;
    }
    let version_id = version_id.expect("FileVersion fan-out records the immutable version id");
    drop(state);

    set_os_apps_dir(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("os-apps"),
    );
    reload_os_apps();
    let mut platform = PlatformState::new(None);
    platform
        .server
        .set_storage_stack(StorageStack::from_turso(store.clone()));
    platform.server.data_dir = temp.path().to_path_buf();
    platform
        .server
        .authz
        .reload_tenant_policies(
            TenantId::default().as_str(),
            "permit(principal, action, resource);",
        )
        .expect("migration policy parses");

    let required = reconcile_os_app(&platform, TenantId::default().as_str(), "paw-fs")
        .await
        .expect("activated PawFS bundle stages its migration target");
    let OsAppReconcileResult::MigrationRequired {
        semantic_digest,
        capability_digest,
        descriptor_contract_version,
        ..
    } = required
    else {
        panic!("first activated PawFS reconcile must require migration: {required:?}");
    };
    assert_eq!(descriptor_contract_version, 1);
    let target = StreamDescriptorMigrationTargetV1::InstalledApplication {
        application_id: "paw-fs".into(),
        semantic_digest,
    };
    let started = platform
        .server
        .start_governed_stream_descriptor_migration_v1(
            &TenantId::default(),
            StartStreamDescriptorMigrationRequestV1 {
                request_id: "pawfs-restart-migration-start".into(),
                idempotency_key: "pawfs-restart-migration-start".into(),
                target: target.clone(),
                expected_capability_digest: capability_digest,
                descriptor_contract_version,
                budgets: StreamDescriptorMigrationBudgetsV1 {
                    max_subjects: 16,
                    max_events_per_subject: 64,
                    max_blob_bytes: 1_048_576,
                },
            },
        )
        .await
        .expect("governed PawFS migration starts");
    let first_page = platform
        .server
        .advance_governed_stream_descriptor_migration_v1(
            &TenantId::default(),
            AdvanceStreamDescriptorMigrationRequestV1 {
                request_id: "pawfs-restart-migration-advance".into(),
                idempotency_key: "pawfs-restart-migration-advance".into(),
                job_id: started.job_id,
            },
        )
        .await
        .expect("governed PawFS migration advances");
    assert_eq!(first_page.status, "migrating", "{first_page:?}");
    assert_eq!(first_page.migrated_subjects, 2, "{first_page:?}");
    let completed = platform
        .server
        .advance_governed_stream_descriptor_migration_v1(
            &TenantId::default(),
            AdvanceStreamDescriptorMigrationRequestV1 {
                request_id: "pawfs-restart-migration-finalize".into(),
                idempotency_key: "pawfs-restart-migration-finalize".into(),
                job_id: first_page.job_id,
            },
        )
        .await
        .expect("governed PawFS migration commits its final inventory page");
    assert_eq!(completed.status, "completed", "{completed:?}");
    assert_eq!(completed.migrated_subjects, 2, "{completed:?}");
    assert_eq!(completed.unresolved_subjects, 0, "{completed:?}");
    assert!(completed.completion_receipt_id.is_some(), "{completed:?}");
    platform
        .server
        .require_stream_descriptor_completion_v1(
            &TenantId::default(),
            &target,
            completed.completion_receipt_id.as_deref(),
        )
        .await
        .expect("completed PawFS migration evidence passes the activation gate");

    let post_install_migration =
        reconcile_os_app(&platform, TenantId::default().as_str(), "paw-fs")
            .await
            .expect("PawFS install completes before its publication fence activates");
    assert!(
        matches!(
            post_install_migration,
            OsAppReconcileResult::MigrationRequired { .. }
        ),
        "PawFS content bootstrap must invalidate the pre-install generation: {post_install_migration:?}"
    );

    let mut resumed = completed;
    for page in 0..16 {
        resumed = platform
            .server
            .advance_governed_stream_descriptor_migration_v1(
                &TenantId::default(),
                AdvanceStreamDescriptorMigrationRequestV1 {
                    request_id: format!("pawfs-post-install-migration-{page}"),
                    idempotency_key: format!("pawfs-post-install-migration-{page}"),
                    job_id: resumed.job_id.clone(),
                },
            )
            .await
            .expect("governed PawFS migration resumes after install-time stream writes");
        if resumed.status == "completed" {
            break;
        }
    }
    assert_eq!(resumed.status, "completed", "{resumed:?}");
    assert_eq!(resumed.unresolved_subjects, 0, "{resumed:?}");
    assert!(resumed.completion_receipt_id.is_some(), "{resumed:?}");

    let installed = reconcile_os_app(&platform, TenantId::default().as_str(), "paw-fs")
        .await
        .expect("completed migration permits PawFS activation");
    assert!(
        matches!(
            installed,
            OsAppReconcileResult::Installed { .. } | OsAppReconcileResult::Skipped { .. }
        ),
        "activated PawFS bundle should reach steady state after migration: {installed:?}"
    );
    drop(platform);

    let proof_ioa = proof_file_ioa();
    let restarted = build_state(
        "pawfs-after-migration-restart",
        store,
        temp.path(),
        PAWFS_CSDL,
        &proof_ioa,
    );
    let response = app(restarted.clone())
        .oneshot(
            Request::get(format!("/tdata/Files('{file_id}')/$value"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .expect("restarted PawFS $value read responds");
    assert_eq!(response.status(), StatusCode::OK);
    let restarted_content = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .expect("restarted PawFS content reads");
    assert_eq!(restarted_content.as_ref(), b"typed restart proof");
    let persisted = restarted
        .get_tenant_entity_state(&TenantId::default(), "File", file_id)
        .await
        .expect("File rehydrates from persistent state");
    assert_eq!(persisted.state.status, "Ready");
    assert_eq!(persisted.state.fields["has_content"], true);
    let manifest: ModuleSdkManifest =
        serde_json::from_str(MANIFEST_JSON).expect("generated manifest decodes");
    let file_schema = manifest
        .entities
        .iter()
        .find(|entity| entity.entity_type == "Paw.FS.File")
        .expect("generated File schema exists");
    assert!(
        manifest
            .grant
            .operations
            .contains(&DataOperationKind::FileRead),
        "generated PawFS fixture must grant typed stream reads"
    );
    let file_grant = manifest
        .grant
        .entities
        .iter()
        .find(|entity| entity.entity_type == "Paw.FS.File")
        .expect("generated File grant exists");
    for operation in [
        FileOperationKind::ContentRead,
        FileOperationKind::VersionRead,
    ] {
        assert!(
            file_grant.file_operations.contains(&operation),
            "generated PawFS fixture must grant {operation:?}"
        );
    }
    assert_eq!(
        manifest.stream_capabilities.len(),
        2,
        "generated PawFS fixture must bind both File and FileVersion descriptor contracts"
    );
    run_generated_stream_client(restarted.clone(), manifest.clone(), file_id, &version_id).await;
    for property in ["DirectoryId", "CreatedAt", "UpdatedAt"] {
        assert!(
            file_schema
                .properties
                .iter()
                .find(|candidate| candidate.canonical_name == property)
                .expect("generated optional property exists")
                .nullable,
            "{property} must generate as optional"
        );
    }
    assert!(
        !file_schema
            .properties
            .iter()
            .find(|property| property.canonical_name == "WorkspaceId")
            .expect("generated WorkspaceId property exists")
            .nullable,
        "WorkspaceId must remain required for workspace isolation"
    );
    let typed_response = match temper_server::application_data::canonicalize_entity_for_test(
        file_schema,
        &persisted.state,
    ) {
        Ok(value) => DataResponseV1::ok(DataResultV1::Entity {
            value,
            sequence: persisted.state.sequence_nr,
        }),
        Err(error) => DataResponseV1::error(error),
    };
    install_native_data_host_for_test(vec![typed_response]);

    let typed = FileClient::new()
        .get(file_id)
        .expect("generated FileClient decodes persisted PawFS metadata");
    let value = serde_json::to_value(&typed.value).expect("typed File serializes");
    assert_eq!(value["WorkspaceId"], workspace_id);
    assert!(value["DirectoryId"].is_null());
    assert!(value["CreatedAt"].is_null());
    assert!(value["UpdatedAt"].is_null());
    assert_eq!(typed.sequence, persisted.state.sequence_nr);

    let requests = take_native_data_requests_for_test();
    assert_eq!(requests.len(), 1);
}

async fn run_generated_stream_client(
    state: ServerState,
    manifest: ModuleSdkManifest,
    file_id: &str,
    version_id: &str,
) {
    let wasm = compile_generated_stream_guest(file_id, version_id);
    let module_hash = state
        .wasm_engine
        .compile_and_cache(&wasm)
        .expect("generated PawFS stream guest compiles");
    state
        .wasm_module_registry
        .write()
        .expect("PawFS WASM registry lock")
        .register(
            &TenantId::default(),
            "pawfs_restart_regression",
            &module_hash,
        );
    state
        .wasm_module_registry
        .write()
        .expect("PawFS WASM registry lock")
        .bind_data_manifest(
            &TenantId::default(),
            "pawfs_restart_regression",
            &module_hash,
            manifest,
        );
    let agent = AgentContext::system();
    let result = state
        .dispatch_tenant_action_ext(
            &TenantId::default(),
            "File",
            file_id,
            "RunStreamReadProof",
            serde_json::json!({}),
            DispatchExtOptions {
                agent_ctx: &agent,
                await_integration: true,
                await_reactions: true,
            },
        )
        .await
        .expect("generated PawFS stream proof dispatch completes");
    assert!(result.success, "generated PawFS client failed: {result:?}");
    assert_eq!(
        result.state.status, "Locked",
        "only the generated client's success callback may lock the File"
    );
}

fn compile_generated_stream_guest(file_id: &str, version_id: &str) -> Vec<u8> {
    let file_id = serde_json::to_string(file_id).expect("File id encodes as Rust string literal");
    let version_id =
        serde_json::to_string(version_id).expect("FileVersion id encodes as Rust string literal");
    let guest = format!(
        r#"
fn read_all(mut opened: OpenedFileRead) -> Result<Vec<u8>, ModuleDataError> {{
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 7];
    loop {{
        let read = opened.reader.read(&mut chunk)?;
        if read == 0 {{
            return Ok(bytes);
        }}
        bytes.extend_from_slice(&chunk[..read]);
    }}
}}

#[unsafe(no_mangle)]
pub extern "C" fn run(_ctx_ptr: i32, _ctx_len: i32) -> i32 {{
    let result = (|| -> Result<(), String> {{
        let mut client = FileClient::new();
        let current = client
            .open_file_read({file_id})
            .map_err(|error| format!("current File read: {{error}}"))?;
        if read_all(current).map_err(|error| error.to_string())? != b"typed restart proof" {{
            return Err("generated current File read returned different bytes".into());
        }}
        let version = client
            .open_file_version_read({file_id}, {version_id})
            .map_err(|error| format!("FileVersion read: {{error}}"))?;
        if read_all(version).map_err(|error| error.to_string())? != b"typed restart proof" {{
            return Err("generated FileVersion read returned different bytes".into());
        }}
        Ok(())
    }})();
    match result {{
        Ok(()) => temper_wasm_sdk::set_success_result(
            "callback",
            &temper_wasm_sdk::json!({{"verified": true}}),
        ),
        Err(error) => temper_wasm_sdk::set_error_result(&error),
    }}
    0
}}
"#
    );
    let temp = tempfile::tempdir().expect("generated PawFS guest directory creates");
    fs::create_dir(temp.path().join("src")).expect("generated PawFS guest src creates");
    fs::write(
        temp.path().join("Cargo.toml"),
        format!(
            "[package]\nname='pawfs-generated-stream-restart-guest'\nversion='0.0.0'\nedition='2024'\n\n[lib]\ncrate-type=['cdylib']\n\n[dependencies]\ntemper-wasm-sdk={{git='https://github.com/nikstern/temper.git',rev='{TEMPER_REVISION}'}}\nserde={{version='1',features=['derive']}}\nserde_json='1'\n"
        ),
    )
    .expect("generated PawFS guest manifest writes");
    fs::write(
        temp.path().join("src/lib.rs"),
        format!("{GENERATED_SOURCE}\n{guest}"),
    )
    .expect("generated PawFS guest source writes");
    let output = Command::new(env!("CARGO"))
        .args([
            "build",
            "--target",
            "wasm32-unknown-unknown",
            "--release",
            "--offline",
            "--quiet",
        ])
        .env("RUSTFLAGS", "-C link-arg=--allow-undefined")
        .current_dir(temp.path())
        .output()
        .expect("generated PawFS guest Cargo starts");
    assert!(
        output.status.success(),
        "generated PawFS guest failed to compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::read(
        temp.path().join(
            "target/wasm32-unknown-unknown/release/pawfs_generated_stream_restart_guest.wasm",
        ),
    )
    .expect("generated PawFS guest artifact reads")
}
