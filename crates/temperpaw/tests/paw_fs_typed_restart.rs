use axum::body::Body;
use axum::http::{Request, StatusCode};
use pawfs_typed_client_fixture::{
    DataResponseV1, DataResultV1, FileClient, MANIFEST_JSON, ModuleSdkManifest,
    install_native_data_host_for_test, take_native_data_requests_for_test,
};
use temper_runtime::ActorSystem;
use temper_runtime::tenant::TenantId;
use temper_server::registry::{
    EntityLevelSummary, EntityVerificationResult, SpecRegistry, VerificationStatus,
};
use temper_server::{ServerState, StorageStack, build_router};
use temper_spec::csdl::parse_csdl;
use temper_store_turso::TursoEventStore;
use tower::ServiceExt;

const PAWFS_CSDL: &str = include_str!("../../../os-apps/paw-fs/specs/model.csdl.xml");
const FILE_IOA: &str = include_str!("../../../os-apps/paw-fs/specs/file.ioa.toml");
const WORKSPACE_IOA: &str = include_str!("../../../os-apps/paw-fs/specs/workspace.ioa.toml");

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

fn build_state(name: &str, store: TursoEventStore, data_dir: &std::path::Path) -> ServerState {
    let mut registry = SpecRegistry::new();
    registry.register_tenant(
        TenantId::default().as_str(),
        parse_csdl(PAWFS_CSDL).expect("PawFS CSDL parses"),
        PAWFS_CSDL.to_string(),
        &[("File", FILE_IOA), ("Workspace", WORKSPACE_IOA)],
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
async fn typed_file_client_reads_honest_optional_metadata_after_persistent_restart() {
    let temp = tempfile::tempdir().expect("PawFS restart data directory");
    let database_url = format!("file:{}", temp.path().join("pawfs.db").display());
    let store = TursoEventStore::new(&database_url, None)
        .await
        .expect("persistent Turso store initializes");
    let file_id = "018f1f80-7b2d-7000-8000-000000000008";
    let workspace_id = "018f1f80-7b2d-7000-8000-000000000009";

    let state = build_state("pawfs-before-restart", store.clone(), temp.path());
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
    drop(state);

    let restarted = build_state("pawfs-after-restart", store, temp.path());
    let persisted = restarted
        .get_tenant_entity_state(&TenantId::default(), "File", file_id)
        .await
        .expect("File rehydrates from persistent state");
    let manifest: ModuleSdkManifest =
        serde_json::from_str(MANIFEST_JSON).expect("generated manifest decodes");
    let file_schema = manifest
        .entities
        .iter()
        .find(|entity| entity.entity_type == "Paw.FS.File")
        .expect("generated File schema exists");
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
