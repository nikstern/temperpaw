use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Eq, PartialEq)]
struct ActionContract {
    binding_nullable: bool,
    params: Vec<(String, bool)>,
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn local_type_name(qualified: &str) -> &str {
    qualified.rsplit('.').next().unwrap_or(qualified)
}

fn csdl_action_contracts(source: &str) -> BTreeMap<(String, String), ActionContract> {
    let document = roxmltree::Document::parse(source).expect("paw-fs CSDL should parse");
    let mut contracts = BTreeMap::new();

    for action in document
        .descendants()
        .filter(|node| node.has_tag_name("Action") && node.attribute("IsBound") == Some("true"))
    {
        let parameters = action
            .children()
            .filter(|node| node.has_tag_name("Parameter"))
            .collect::<Vec<_>>();
        let binding = parameters.first().expect("bound action needs a binding");
        let entity = local_type_name(binding.attribute("Type").expect("binding type")).to_string();
        let action_name = action.attribute("Name").expect("action name").to_string();
        let contract = ActionContract {
            binding_nullable: binding.attribute("Nullable") != Some("false"),
            params: parameters[1..]
                .iter()
                .map(|parameter| {
                    (
                        parameter
                            .attribute("Name")
                            .expect("parameter name")
                            .to_string(),
                        parameter.attribute("Nullable") != Some("false"),
                    )
                })
                .collect(),
        };

        assert!(
            contracts.insert((entity, action_name), contract).is_none(),
            "paw-fs CSDL must not contain duplicate bound action overloads"
        );
    }

    contracts
}

fn ioa_callable_contracts(specs_dir: &Path) -> BTreeMap<(String, String), Vec<(String, bool)>> {
    let mut contracts = BTreeMap::new();

    for entry in fs::read_dir(specs_dir).expect("read paw-fs specs") {
        let path = entry.expect("spec directory entry").path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("toml")
            || !path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".ioa.toml"))
        {
            continue;
        }

        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        let spec = source
            .parse::<toml::Value>()
            .unwrap_or_else(|error| panic!("parse {}: {error}", path.display()));
        let entity = spec["automaton"]["name"]
            .as_str()
            .expect("automaton name")
            .to_string();

        for action in spec["action"].as_array().expect("action array") {
            if action["kind"].as_str() == Some("output") {
                continue;
            }
            let action_name = action["name"].as_str().expect("action name").to_string();
            let params = action
                .get("params")
                .and_then(toml::Value::as_array)
                .into_iter()
                .flatten()
                .map(|param| match param {
                    toml::Value::String(name) => (name.clone(), false),
                    toml::Value::Table(param) => (
                        param["name"]
                            .as_str()
                            .expect("typed parameter name")
                            .to_string(),
                        param
                            .get("nullable")
                            .and_then(toml::Value::as_bool)
                            .unwrap_or(false),
                    ),
                    other => panic!("unsupported action parameter {other:?}"),
                })
                .collect();

            contracts.insert((entity.clone(), action_name), params);
        }
    }

    contracts
}

#[test]
fn every_callable_paw_fs_action_has_an_exact_requiredness_twin() {
    let specs_dir = repo_root().join("os-apps/paw-fs/specs");
    let csdl = fs::read_to_string(specs_dir.join("model.csdl.xml")).expect("read paw-fs CSDL");
    let csdl_contracts = csdl_action_contracts(&csdl);
    let ioa_contracts = ioa_callable_contracts(&specs_dir);

    for ((entity, action), ioa_params) in &ioa_contracts {
        let csdl_contract = csdl_contracts
            .get(&(entity.clone(), action.clone()))
            .unwrap_or_else(|| panic!("{entity}.{action} needs an exact bound CSDL action twin"));
        assert!(
            !csdl_contract.binding_nullable,
            "{entity}.{action} bindingParameter must be non-nullable"
        );
        assert_eq!(
            &csdl_contract.params, ioa_params,
            "{entity}.{action} IOA/CSDL parameter names and nullability must match"
        );
    }
}

#[test]
fn paw_fs_optional_action_inputs_are_explicit_and_exhaustive() {
    let specs_dir = repo_root().join("os-apps/paw-fs/specs");
    let actual = ioa_callable_contracts(&specs_dir)
        .into_iter()
        .flat_map(|((entity, action), params)| {
            params
                .into_iter()
                .filter(|(_, nullable)| *nullable)
                .map(move |(parameter, _)| format!("{entity}.{action}.{parameter}"))
        })
        .collect::<BTreeSet<_>>();
    let expected = [
        "Directory.Create.parent_id",
        "WorkspaceUsageBucket.ApplyDelta.artifact_batch_id",
        "WorkspaceUsageBucket.Create.artifact_batch_id",
    ]
    .into_iter()
    .map(str::to_string)
    .collect();

    assert_eq!(actual, expected);
}
