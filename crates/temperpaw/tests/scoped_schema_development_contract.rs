use temper_spec::{IoaSourceInput, ScopedBundleBudgets, ScopedSpecBundle, ScopedSpecBundleInput};

const IOA: &str = r#"[automaton]
name = "Task"
states = ["Open"]
initial = "Open"
"#;

const CSDL: &str = r#"<?xml version="1.0"?>
<edmx:Edmx Version="4.0" xmlns:edmx="http://docs.oasis-open.org/odata/ns/edmx">
<edmx:DataServices><Schema Namespace="Example" xmlns="http://docs.oasis-open.org/odata/ns/edm">
<EntityType Name="Task"><Key><PropertyRef Name="Id"/></Key><Property Name="Id" Type="Edm.String" Nullable="false"/><Property Name="Title" Type="Edm.String"/></EntityType>
<EntityContainer Name="Default"><EntitySet Name="Tasks" EntityType="Example.Task"/></EntityContainer>
</Schema></edmx:DataServices></edmx:Edmx>"#;

#[test]
fn scoped_schema_smoke_fixture_has_stable_digest() {
    let bundle = ScopedSpecBundle::compile(ScopedSpecBundleInput {
        scope_id: "task-115-smoke".into(),
        predecessor_digest: None,
        csdl_xml: CSDL.into(),
        ioa_sources: vec![IoaSourceInput {
            entity_type: "Example.Task".into(),
            source: IOA.into(),
        }],
        cedar_policies: vec![],
        wasm_modules: vec![],
        migration: None,
        budgets: ScopedBundleBudgets::default(),
    })
    .expect("the development image smoke fixture must compile");

    assert_eq!(
        bundle.digest(),
        "sha256:6e48666e22a3a4c6a6579c76608c2d247b908c092ba779623e8d9eca7253713e"
    );
}
