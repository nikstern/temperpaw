#![allow(dead_code)]
#![allow(clippy::new_without_default)]

include!(concat!(env!("OUT_DIR"), "/pawfs_file_client.rs"));

pub const MANIFEST_JSON: &str =
    include_str!(concat!(env!("OUT_DIR"), "/pawfs_file_client_manifest.json"));

pub use temper_wasm_sdk::data::{
    DataResponseV1, DataResultV1, ModuleSdkManifest, install_native_data_host_for_test,
    take_native_data_requests_for_test,
};
