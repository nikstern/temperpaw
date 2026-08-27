//! Acceptance coverage for Temper durable reaction recovery.

use std::sync::Arc;
use std::time::Duration;

use temper_runtime::ActorSystem;
use temper_runtime::persistence::{EventMetadata, PersistenceEnvelope};
use temper_runtime::scheduler::{sim_now, sim_uuid};
use temper_runtime::tenant::TenantId;
use temper_server::ServerState;
use temper_server::registry::SpecRegistry;
use temper_server::request_context::AgentContext;
use temper_server::storage::{BoxedEventStore, StorageStack};
use temper_server::trigger::delivery::{
    DeliveryKind, PersistedReactionIntent, attach_intents, stable_delivery_id,
};
use temper_server::trigger::registry::parse_reactions;
use temper_spec::csdl::parse_csdl;
use temper_store_turso::TursoEventStore;

const CSDL: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<edmx:Edmx Version="4.0" xmlns:edmx="http://docs.oasis-open.org/odata/ns/edmx">
  <edmx:DataServices>
    <Schema Namespace="Paw.DurableReaction" xmlns="http://docs.oasis-open.org/odata/ns/edm">
      <EntityType Name="Order"><Key><PropertyRef Name="Id"/></Key><Property Name="Id" Type="Edm.String" Nullable="false"/><Property Name="Status" Type="Edm.String"/></EntityType>
      <EntityType Name="Payment"><Key><PropertyRef Name="Id"/></Key><Property Name="Id" Type="Edm.String" Nullable="false"/><Property Name="Status" Type="Edm.String"/></EntityType>
      <EntityContainer Name="Container"><EntitySet Name="Orders" EntityType="Paw.DurableReaction.Order"/><EntitySet Name="Payments" EntityType="Paw.DurableReaction.Payment"/></EntityContainer>
    </Schema>
  </edmx:DataServices>
</edmx:Edmx>"#;

const ORDER_IOA: &str = r#"
[automaton]
name = "Order"
states = ["Draft", "Confirmed"]
initial = "Draft"

[[action]]
name = "Confirm"
kind = "internal"
from = ["Draft"]
to = "Confirmed"
"#;

const PAYMENT_IOA: &str = r#"
[automaton]
name = "Payment"
states = ["Pending", "Authorized"]
initial = "Pending"

[[action]]
name = "Authorize"
kind = "internal"
from = ["Pending"]
to = "Authorized"
"#;

const REACTIONS: &str = r#"
[[reaction]]
name = "confirmed_authorizes_payment"
[reaction.when]
entity_type = "Order"
action = "Confirm"
to_state = "Confirmed"
[reaction.then]
entity_type = "Payment"
action = "Authorize"
[reaction.resolve_target]
type = "same_id"
"#;

fn build_state(tenant: &str) -> ServerState {
    let mut registry = SpecRegistry::new();
    registry
        .try_register_tenant_with_reactions(
            tenant,
            parse_csdl(CSDL).expect("CSDL parses"),
            CSDL.to_string(),
            &[("Order", ORDER_IOA), ("Payment", PAYMENT_IOA)],
            parse_reactions(REACTIONS).expect("reaction parses"),
        )
        .expect("tenant registers");
    let state = ServerState::from_registry(ActorSystem::new("paw-reaction-recovery"), registry);
    state
        .authz
        .reload_tenant_policies(tenant, "permit(principal, action, resource);")
        .expect("reaction fixture policy parses");
    state.rebuild_reaction_dispatcher();
    state
}

#[tokio::test]
async fn startup_recovers_source_intent_committed_before_process_exit() {
    let tenant_name = "paw-reaction-recovery";
    let tenant = TenantId::new(tenant_name);
    let tempdir = tempfile::tempdir().expect("temporary Turso directory");
    let database_url = format!("file:{}", tempdir.path().join("reactions.db").display());
    let store = TursoEventStore::new(&database_url, None)
        .await
        .expect("Turso store initializes");
    let boxed = BoxedEventStore::new(store.clone());
    let rule = parse_reactions(REACTIONS)
        .expect("reaction parses")
        .pop()
        .expect("one reaction");
    let delivery_id =
        stable_delivery_id(tenant_name, "Order", "order-1", "Confirm", 1, &rule.name, 0);
    let authority = AgentContext::for_service("durable-reaction-test")
        .security_ctx
        .expect("service authority");
    let intent = PersistedReactionIntent {
        kind: DeliveryKind::Reaction,
        delivery_id: delivery_id.clone(),
        root_delivery_id: delivery_id,
        tenant: tenant_name.to_string(),
        source_entity_type: "Order".to_string(),
        source_entity_id: "order-1".to_string(),
        source_action: "Confirm".to_string(),
        source_sequence: 1,
        source_to_state: "Confirmed".to_string(),
        source_fields: serde_json::json!({}),
        source_stream_descriptor: None,
        guard_passed: true,
        target_entity_id: Some("order-1".to_string()),
        trigger_name: rule.name.clone(),
        trigger_index: 0,
        depth: 0,
        rule: serde_json::to_value(rule).expect("rule serializes"),
        authority: serde_json::to_value(authority).expect("authority serializes"),
        created_at: sim_now(),
        not_before: None,
        state_timeout: None,
        collection: None,
        schema_pin: None,
    };
    let mut payload = serde_json::json!({
        "action": "Confirm",
        "from_status": "Draft",
        "to_status": "Confirmed",
        "timestamp": sim_now(),
        "params": {},
    });
    attach_intents(&mut payload, &[intent]).expect("intent attaches atomically");
    boxed
        .append(
            &format!("{tenant_name}:Order:order-1"),
            0,
            &[PersistenceEnvelope {
                sequence_nr: 1,
                event_type: "Confirm".to_string(),
                payload,
                metadata: EventMetadata {
                    event_id: sim_uuid(),
                    causation_id: sim_uuid(),
                    correlation_id: sim_uuid(),
                    timestamp: sim_now(),
                    actor_id: format!("{tenant_name}:Order:order-1"),
                    kernel: None,
                },
            }],
        )
        .await
        .expect("source event and intent commit");

    let mut restarted = build_state(tenant_name);
    restarted.set_storage_stack(StorageStack::from_turso(store));
    restarted.rebuild_reaction_dispatcher();
    let restarted = Arc::new(restarted);

    for _ in 0..100 {
        if let Ok(state) = restarted
            .get_tenant_entity_state(&tenant, "Payment", "order-1")
            .await
            && state.state.status == "Authorized"
        {
            return;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    panic!("startup recovery did not authorize Payment:order-1");
}
