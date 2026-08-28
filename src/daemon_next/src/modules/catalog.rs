use crate::modules::manifest::{current_platform_key, ModuleManifest};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Catalog {
    pub schema_version: String,
    pub catalog_version: String,
    pub minimum_daemon_version: String,
    pub signature: SignatureMeta,
    pub modules: Vec<ModuleManifest>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SignatureMeta {
    pub algorithm: String,
    pub public_key_fingerprint: String,
    pub signature: String,
}

impl Catalog {
    /// Look up a module by id.
    pub fn find_module(&self, id: &str) -> Option<&ModuleManifest> {
        self.modules.iter().find(|m| m.id == id)
    }
}

/// Built-in catalog containing optional runtime modules that the daemon can ship with.
/// The URLs are placeholders and should be replaced with real distribution endpoints.
pub fn default_catalog() -> Catalog {
    use crate::modules::manifest::{ArchiveFormat, FileEntry, ModuleKind, PlatformEntry};
    use std::collections::HashMap;

    let mut platforms = HashMap::new();
    platforms.insert(
        current_platform_key(),
        PlatformEntry {
            url: String::from("https://example.com/mirage/onnx_runtime-1.28.0.tar.gz"),
            size: 0,
            checksum: String::from(
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            archive_format: ArchiveFormat::TarGz,
            files: vec![FileEntry {
                relative_path: String::from("libonnxruntime.so"),
                sha256: String::from(
                    "0000000000000000000000000000000000000000000000000000000000000000",
                ),
                executable: false,
                required: true,
            }],
        },
    );

    let onnx_runtime = ModuleManifest {
        id: String::from("onnx_runtime"),
        name: String::from("ONNX Runtime"),
        version: String::from("1.28.0"),
        description: String::from("ONNX Runtime inference engine for local embeddings and SLM."),
        kind: ModuleKind::Runtime,
        license: String::from("MIT"),
        is_optional: true,
        dependencies: vec![],
        platforms,
    };

    let mut duckdb_platforms = HashMap::new();
    duckdb_platforms.insert(
        current_platform_key(),
        PlatformEntry {
            url: String::from("https://example.com/mirage/duckdb-1.1.3.tar.gz"),
            size: 0,
            checksum: String::from(
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            archive_format: ArchiveFormat::TarGz,
            files: vec![FileEntry {
                relative_path: String::from("libduckdb.so"),
                sha256: String::from(
                    "0000000000000000000000000000000000000000000000000000000000000000",
                ),
                executable: false,
                required: true,
            }],
        },
    );

    let duckdb_module = ModuleManifest {
        id: String::from("duckdb"),
        name: String::from("DuckDB Analytics"),
        version: String::from("1.1.3"),
        description: String::from("Embedded analytics engine for SQL queries over metadata."),
        kind: ModuleKind::Runtime,
        license: String::from("MIT"),
        is_optional: true,
        dependencies: vec![],
        platforms: duckdb_platforms,
    };

    Catalog {
        schema_version: String::from("1.0.0"),
        catalog_version: String::from("builtin"),
        minimum_daemon_version: String::from("0.1.0"),
        signature: SignatureMeta {
            algorithm: String::from("ed25519"),
            public_key_fingerprint: String::from("builtin"),
            signature: String::from("builtin"),
        },
        modules: vec![onnx_runtime, duckdb_module],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_catalog() {
        let json = r#"{
            "schema_version": "1.0.0",
            "catalog_version": "2026.08.27-1",
            "minimum_daemon_version": "0.2.0",
            "signature": {
                "algorithm": "ed25519",
                "public_key_fingerprint": "a1b2c3",
                "signature": "base64sig"
            },
            "modules": []
        }"#;
        let catalog: Catalog = serde_json::from_str(json).unwrap();
        assert_eq!(catalog.schema_version, "1.0.0");
        assert!(catalog.find_module("duckdb").is_none());
    }
}
