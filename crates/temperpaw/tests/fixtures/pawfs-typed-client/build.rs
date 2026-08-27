use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::PathBuf;

use temper_codegen::generate_module_sdk;
use temper_spec::bundle::IoaSourceInput;
use temper_spec::csdl::parse_csdl;
use temper_wasm_sdk::data::{
    DataOperationKind, EntityDataGrant, FileOperationKind, ModuleDataGrant,
};

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let repository = manifest_dir.ancestors().nth(5).unwrap();
    let csdl_path = repository.join("os-apps/paw-fs/specs/model.csdl.xml");
    let ioa_path = repository.join("os-apps/paw-fs/specs/file.ioa.toml");
    let version_ioa_path = repository.join("os-apps/paw-fs/specs/file_version.ioa.toml");
    println!("cargo:rerun-if-changed={}", csdl_path.display());
    println!("cargo:rerun-if-changed={}", ioa_path.display());
    println!("cargo:rerun-if-changed={}", version_ioa_path.display());

    let csdl_source = fs::read_to_string(&csdl_path).unwrap();
    let ioa_source = fs::read_to_string(&ioa_path).unwrap();
    let version_ioa_source = fs::read_to_string(&version_ioa_path).unwrap();
    let csdl = parse_csdl(&csdl_source).unwrap();
    let generated = generate_module_sdk(
        &csdl,
        &[
            IoaSourceInput {
                entity_type: "Paw.FS.File".into(),
                source: ioa_source,
            },
            IoaSourceInput {
                entity_type: "Paw.FS.FileVersion".into(),
                source: version_ioa_source,
            },
        ],
        "pawfs_restart_regression",
        "test-closure",
        "test-closure",
        "unpackaged",
        ModuleDataGrant {
            operations: BTreeSet::from([DataOperationKind::EntityGet, DataOperationKind::FileRead]),
            entities: vec![EntityDataGrant {
                entity_type: "Paw.FS.File".into(),
                file_operations: BTreeSet::from([
                    FileOperationKind::MetadataRead,
                    FileOperationKind::ContentRead,
                    FileOperationKind::VersionRead,
                ]),
                ..EntityDataGrant::default()
            }],
            ..ModuleDataGrant::default()
        },
    )
    .unwrap();

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    fs::write(out_dir.join("pawfs_file_client.rs"), &generated.source).unwrap();
    fs::write(
        out_dir.join("pawfs_file_client_source.rs"),
        &generated.source,
    )
    .unwrap();
    fs::write(
        out_dir.join("pawfs_file_client_manifest.json"),
        serde_json::to_vec_pretty(&generated.manifest).unwrap(),
    )
    .unwrap();
}
