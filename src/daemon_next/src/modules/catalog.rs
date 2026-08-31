use crate::modules::manifest::{current_platform_key, ModuleManifest, PlatformEntry};
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

/// A single-file model download: the archive *is* the file, and the manifest
/// decides the name it lands under inside the module directory.
fn raw_model(url: &str, size: u64, sha256: &str, file_name: &str) -> PlatformEntry {
    use crate::modules::manifest::{ArchiveFormat, FileEntry};
    PlatformEntry {
        url: url.to_string(),
        size,
        checksum: sha256.to_string(),
        archive_format: ArchiveFormat::Raw,
        files: vec![FileEntry {
            relative_path: file_name.to_string(),
            sha256: sha256.to_string(),
            executable: false,
            required: true,
        }],
    }
}

/// Built-in catalog of the on-device models that make search work.
///
/// Every entry is a real, publicly hosted artifact pinned by SHA-256, so a
/// download either verifies or fails loudly. The ONNX Runtime and DuckDB entries
/// that used to live here are gone: this build links both (`ort/download-binaries`
/// and the `duckdb` feature), and their URLs were placeholders that never worked.
pub fn default_catalog() -> Catalog {
    use crate::modules::manifest::{ModuleKind, PlatformEntry};
    use std::collections::HashMap;

    let clip = "https://huggingface.co/Xenova/clip-vit-base-patch32/resolve/main";

    // CLIP text encoder: int8 quantized, emits a 512-d projected embedding.
    let mut text_platforms: HashMap<String, PlatformEntry> = HashMap::new();
    text_platforms.insert(
        current_platform_key(),
        raw_model(
            &format!("{clip}/onnx/text_model_int8.onnx"),
            64_070_791,
            "18845f2ccc35223bb7fec403383a131154b11ac0918df25cf51986df5efd3a21",
            "clip_text_encoder.onnx",
        ),
    );
    let clip_text = ModuleManifest {
        id: String::from("clip_text_encoder"),
        name: String::from("Text encoder (CLIP)"),
        version: String::from("1.0.0"),
        description: String::from(
            "Encodes words and sentences into the same 512-dimensional space as images.",
        ),
        kind: ModuleKind::Model,
        license: String::from("MIT"),
        is_optional: true,
        dependencies: vec![String::from("clip_tokenizer")],
        platforms: text_platforms,
    };

    // CLIP vision encoder: int8 quantized, same 512-d space.
    let mut vision_platforms: HashMap<String, PlatformEntry> = HashMap::new();
    vision_platforms.insert(
        current_platform_key(),
        raw_model(
            &format!("{clip}/onnx/vision_model_int8.onnx"),
            88_648_877,
            "0ab0c1b3ace708e539633af1744d5a95247fe4e14d3e08ff197ef82a6cb9bd93",
            "clip_vision_encoder.onnx",
        ),
    );
    let clip_vision = ModuleManifest {
        id: String::from("clip_vision_encoder"),
        name: String::from("Vision encoder (CLIP)"),
        version: String::from("1.0.0"),
        description: String::from(
            "Encodes photographs into the same 512-dimensional space as text, so a word can find a picture.",
        ),
        kind: ModuleKind::Model,
        license: String::from("MIT"),
        is_optional: true,
        dependencies: vec![],
        platforms: vision_platforms,
    };

    let mut tokenizer_platforms: HashMap<String, PlatformEntry> = HashMap::new();
    tokenizer_platforms.insert(
        current_platform_key(),
        raw_model(
            &format!("{clip}/tokenizer.json"),
            2_224_119,
            "f7f3b7af117d467b58374797691a6438d3e6b9e9cef800dfd5dced7f697a90cd",
            "clip_tokenizer.json",
        ),
    );
    let clip_tokenizer = ModuleManifest {
        id: String::from("clip_tokenizer"),
        name: String::from("CLIP tokenizer"),
        version: String::from("1.0.0"),
        description: String::from("Byte-pair encoding vocabulary shared by both CLIP encoders."),
        kind: ModuleKind::Model,
        license: String::from("MIT"),
        is_optional: false,
        dependencies: vec![],
        platforms: tokenizer_platforms,
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
        modules: vec![clip_text, clip_vision, clip_tokenizer],
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

    #[test]
    fn built_in_catalog_entries_are_real_downloads() {
        let catalog = default_catalog();
        assert!(!catalog.modules.is_empty());
        for module in &catalog.modules {
            let platform = module
                .platform_for_current_target()
                .unwrap_or_else(|| panic!("{} has no entry for the host platform", module.id));
            assert!(
                platform.url.starts_with("https://") && !platform.url.contains("example.com"),
                "{} points at {}",
                module.id,
                platform.url
            );
            assert!(platform.size > 0, "{} has no size", module.id);
            assert_eq!(
                platform.checksum.len(),
                64,
                "{} is not pinned by a sha256",
                module.id
            );
            assert!(
                !platform.files.is_empty(),
                "{} declares no files",
                module.id
            );
            for file in &platform.files {
                assert_eq!(
                    &file.sha256, &platform.checksum,
                    "{}: file checksum differs from the archive checksum",
                    module.id
                );
            }
            for dependency in &module.dependencies {
                assert!(
                    catalog.find_module(dependency).is_some(),
                    "{} depends on the unknown module {}",
                    module.id,
                    dependency
                );
            }
        }
    }

    #[test]
    fn clip_pair_is_the_shared_text_image_space() {
        let catalog = default_catalog();
        let text = catalog.find_module("clip_text_encoder").unwrap();
        let vision = catalog.find_module("clip_vision_encoder").unwrap();
        // Both encoders come from the same published model repository, which is what
        // makes their vectors comparable.
        let repo_of = |m: &ModuleManifest| {
            m.platform_for_current_target()
                .unwrap()
                .url
                .split("/resolve/")
                .next()
                .unwrap()
                .to_string()
        };
        assert_eq!(repo_of(text), repo_of(vision));
        assert_eq!(
            text.dependencies,
            vec![String::from("clip_tokenizer")],
            "the text encoder needs the tokenizer to produce real token ids"
        );
    }
}
