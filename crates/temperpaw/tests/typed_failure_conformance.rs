use temper_runtime::ActorSystem;
use temper_runtime::tenant::TenantId;
use temper_server::registry::SpecRegistry;
use temper_server::request_context::AgentContext;
use temper_server::state::DispatchExtOptions;
use temper_server::{EntityResponse, ServerState};
use temper_spec::csdl::parse_csdl;
use temper_verify::build_model_from_ioa;

const TYPED_ROUTE: &str = include_str!("fixtures/typed-failure-conformance/typed_route.ioa.toml");
const LEGACY_ON_FAILURE: &str =
    include_str!("fixtures/typed-failure-conformance/legacy_on_failure.ioa.toml");

const CSDL: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<edmx:Edmx Version="4.0" xmlns:edmx="http://docs.oasis-open.org/odata/ns/edmx">
  <edmx:DataServices>
    <Schema Namespace="TemperPaw.Conformance" xmlns="http://docs.oasis-open.org/odata/ns/edm">
      <EntityType Name="FailureRouteProbe">
        <Key><PropertyRef Name="Id"/></Key>
        <Property Name="Id" Type="Edm.String" Nullable="false"/>
        <Property Name="Status" Type="Edm.String"/>
      </EntityType>
      <EntityContainer Name="Container">
        <EntitySet Name="FailureRouteProbes" EntityType="TemperPaw.Conformance.FailureRouteProbe"/>
      </EntityContainer>
    </Schema>
  </edmx:DataServices>
</edmx:Edmx>"#;

fn state_from(ioa: &str) -> ServerState {
    let mut registry = SpecRegistry::new();
    registry
        .try_register_tenant(
            TenantId::default(),
            parse_csdl(CSDL).expect("conformance CSDL parses"),
            CSDL.to_string(),
            &[("FailureRouteProbe", ioa)],
        )
        .expect("conformance IOA parses and compiles into verified JIT metadata");
    ServerState::from_registry(ActorSystem::new("typed-failure-conformance"), registry)
}

async fn run_missing_module(ioa: &str, entity_id: &str) -> EntityResponse {
    let state = state_from(ioa);
    let agent_ctx = AgentContext::system();
    state
        .dispatch_tenant_action_ext(
            &TenantId::default(),
            "FailureRouteProbe",
            entity_id,
            "Run",
            serde_json::json!({}),
            DispatchExtOptions {
                agent_ctx: &agent_ctx,
                await_integration: true,
                await_reactions: true,
            },
        )
        .await
        .expect("source action and failure callback dispatch")
}

#[tokio::test(flavor = "multi_thread")]
async fn typed_failure_v1_route_parses_verifies_and_executes() {
    assert!(TYPED_ROUTE.contains("params = [{ name = \"failure\", type = \"failure_v1\" }]"));
    let model = build_model_from_ioa(TYPED_ROUTE, 2).expect("typed route verifies");
    assert!(
        model
            .transitions
            .iter()
            .any(|transition| transition.name == "Fail")
    );

    let response = run_missing_module(TYPED_ROUTE, "typed-route").await;
    assert!(response.success);
    assert_eq!(response.state.status, "Failed");
}

#[tokio::test(flavor = "multi_thread")]
async fn legacy_on_failure_callback_still_executes_unchanged() {
    let response = run_missing_module(LEGACY_ON_FAILURE, "legacy-on-failure").await;
    assert!(response.success);
    assert_eq!(response.state.status, "Failed");
}
