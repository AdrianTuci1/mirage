use crate::modules::manifest::ModuleManifest;
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
