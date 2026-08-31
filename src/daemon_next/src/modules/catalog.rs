use crate::modules::manifest::{current_platform_key, ModuleManifest, PlatformEntry};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

/// A prebuilt official archive holding one executable. `checksum` is the SHA-256
/// of the archive, `file_sha256` the SHA-256 of the binary inside it.
fn cli_archive(
    url: &str,
    size: u64,
    checksum: &str,
    file_name: &str,
    file_sha256: &str,
) -> PlatformEntry {
    use crate::modules::manifest::{ArchiveFormat, FileEntry};
    PlatformEntry {
        url: url.to_string(),
        size,
        checksum: checksum.to_string(),
        archive_format: ArchiveFormat::Zip,
        files: vec![FileEntry {
            relative_path: file_name.to_string(),
            sha256: file_sha256.to_string(),
            executable: true,
            required: true,
        }],
    }
}

/// The DuckDB command-line binary the tabular feature runs on.
///
/// Published by the DuckDB project per platform; `size`/`checksum` match the
/// release assets of the pinned tag.
fn duckdb_entry(
    platforms: &mut HashMap<String, PlatformEntry>,
    key: &str,
    asset: &str,
    size: u64,
    checksum: &str,
    file_name: &str,
    file_sha256: &str,
) {
    platforms.insert(
        key.to_string(),
        cli_archive(
            &format!(
                "https://github.com/duckdb/duckdb/releases/download/{DUCKDB_RELEASE_TAG}/duckdb_cli-{asset}.zip"
            ),
            size,
            checksum,
            file_name,
            file_sha256,
        ),
    );
}

/// Version of the downloadable DuckDB engine, as it appears in the module path.
pub const DUCKDB_MODULE_VERSION: &str = "1.5.5";

/// Release tag the download URLs are pinned to.
pub const DUCKDB_RELEASE_TAG: &str = "v1.5.5";

/// Version of the downloadable ONNX Runtime module, as it appears in the module path.
pub const ONNX_RUNTIME_MODULE_VERSION: &str = "1.28.0";

/// Release tag the ONNX Runtime download URLs are pinned to.
pub const ONNX_RUNTIME_RELEASE_TAG: &str = "v1.28.0";

/// The downloadable ONNX Runtime shared library, per platform.
///
/// The daemon loads it at runtime through `ORT_DYLIB_PATH` rather than linking
/// it (`ort/load-dynamic`), so `size`/`checksum` are the archive Microsoft
/// publishes and `file_sha256` is the library file inside it. On Linux the
/// archive only contains the versioned `.so.1.28.0` as a real file — the
/// `libonnxruntime.so` symlinks are not extracted, which is why the manifest
/// names the versioned file.
fn onnx_runtime_entry(
    platforms: &mut HashMap<String, PlatformEntry>,
    key: &str,
    asset: &str,
    format: crate::modules::manifest::ArchiveFormat,
    size: u64,
    checksum: &str,
    file_path: &str,
    file_sha256: &str,
) {
    use crate::modules::manifest::FileEntry;
    platforms.insert(
        key.to_string(),
        PlatformEntry {
            url: format!(
                "https://github.com/microsoft/onnxruntime/releases/download/{ONNX_RUNTIME_RELEASE_TAG}/{asset}"
            ),
            size,
            checksum: checksum.to_string(),
            archive_format: format,
            files: vec![FileEntry {
                relative_path: file_path.to_string(),
                sha256: file_sha256.to_string(),
                executable: false,
                required: true,
            }],
        },
    );
}

/// Built-in catalog of the on-device models that make search work.
///
/// Every entry is a real, publicly hosted artifact pinned by SHA-256, so a
/// download either verifies or fails loudly. The DuckDB and ONNX Runtime engines
/// are *not* linked into the daemon any more: they are downloaded as modules and
/// used as a subprocess / runtime-loaded library, so the `ort` crate is built
/// with `load-dynamic` and points at this module's library through
/// `ORT_DYLIB_PATH`.
pub fn default_catalog() -> Catalog {
    use crate::modules::manifest::ModuleKind;

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

    // The tabular SQL engine: the official DuckDB command-line binary, which the
    // daemon runs as a subprocess instead of linking into itself.
    let mut duckdb_platforms: HashMap<String, PlatformEntry> = HashMap::new();
    duckdb_entry(
        &mut duckdb_platforms,
        "macos_aarch64",
        "osx-arm64",
        17_005_846,
        "da5177b8869c4ed8c65d514fb47a8ed0f6fa7427f103304932d5e83851e46abd",
        "duckdb",
        "d0610710dd30667aa6c76709299b6822e55dc9199803350aa2e1b06e3346943b",
    );
    duckdb_entry(
        &mut duckdb_platforms,
        "macos_x86_64",
        "osx-amd64",
        18_961_648,
        "47cbda17c5d4643a58833617dfae649a6a8722d7e54435a08161b98ac1c4e832",
        "duckdb",
        "b5f6e002e2bf316534a3b79f81f07b0faf3f7b88c444d613b515d7346c772735",
    );
    duckdb_entry(
        &mut duckdb_platforms,
        "linux_x86_64",
        "linux-amd64",
        21_263_499,
        "08c0ca117111fcede14239d0093792352befdc174218c344d232c13279643d05",
        "duckdb",
        "3d33b1df037cb049155c393778df7853fafb23e9d49d7c9cacdde4dd67155788",
    );
    duckdb_entry(
        &mut duckdb_platforms,
        "linux_aarch64",
        "linux-arm64",
        19_269_812,
        "02163197027a42149147364d31fa67cac82108517a4be43304a1cc226eaef07a",
        "duckdb",
        "9882c99a9804407de82c0edb1816d7667733d37d771a98eb23ad5f6a8d37acb1",
    );
    duckdb_entry(
        &mut duckdb_platforms,
        "windows_x86_64",
        "windows-amd64",
        12_921_759,
        "e1428b7114a841626b5054723731cbf45c6df91b42ae1a6c355f88fad1f6dc4c",
        "duckdb.exe",
        "fde737c7749075f6b54e14772a4e6b33a5fa0201075d03640aca358074ea4554",
    );
    let duckdb = ModuleManifest {
        id: String::from(crate::analytics::DUCKDB_MODULE_ID),
        name: String::from("DuckDB (tabular)"),
        version: String::from(DUCKDB_MODULE_VERSION),
        description: String::from(
            "SQL engine for questions about tables: counts, sums and groups over indexed files.",
        ),
        kind: ModuleKind::Runtime,
        license: String::from("MIT"),
        is_optional: true,
        dependencies: vec![],
        platforms: duckdb_platforms,
    };

    // The shared library the CLIP encoders load at runtime: the official
    // ONNX Runtime, which the daemon no longer links (`ort/load-dynamic`).
    // macOS x64 is not published by ONNX Runtime 1.28, so the catalog offers
    // the four platforms that have a real asset.
    let mut onnx_platforms: HashMap<String, PlatformEntry> = HashMap::new();
    onnx_runtime_entry(
        &mut onnx_platforms,
        "macos_aarch64",
        "onnxruntime-osx-arm64-1.28.0.tgz",
        crate::modules::manifest::ArchiveFormat::TarGz,
        32_396_562,
        "1268b359718099bde2cedb55787f182a130067bc4f31e8c88478c445b850d3d8",
        "onnxruntime-osx-arm64-1.28.0/lib/libonnxruntime.dylib",
        "dc19bbcb2f5c9fb3c68b4f9248aa0a35065ff702c5dbeae75eac54a74da97b6d",
    );
    onnx_runtime_entry(
        &mut onnx_platforms,
        "linux_x86_64",
        "onnxruntime-linux-x64-1.28.0.tgz",
        crate::modules::manifest::ArchiveFormat::TarGz,
        9_125_960,
        "a3e1b79d7bb1bf09696ce675f49e4064e6c81f6202b8225624fff0e93f8d6407",
        "onnxruntime-linux-x64-1.28.0/lib/libonnxruntime.so.1.28.0",
        "1461ef7cc3d9e49982591721683cc3e3a55580aeca9a5254e7aac47b75ee4bab",
    );
    onnx_runtime_entry(
        &mut onnx_platforms,
        "linux_aarch64",
        "onnxruntime-linux-aarch64-1.28.0.tgz",
        crate::modules::manifest::ArchiveFormat::TarGz,
        8_116_278,
        "e15ff8b5d85afe6c144d97c6fd432254bf76a219daaf17658087d6ecb3e8f0bb",
        "onnxruntime-linux-aarch64-1.28.0/lib/libonnxruntime.so.1.28.0",
        "f1ec1a08eb99bd6e5401340f0a2b101381bf4694415480291dc13bcaa30f9ec7",
    );
    onnx_runtime_entry(
        &mut onnx_platforms,
        "windows_x86_64",
        "onnxruntime-win-x64-1.28.0.zip",
        crate::modules::manifest::ArchiveFormat::Zip,
        78_796_801,
        "abef733dacbe2f571547a7150b479b5cb9cc0df22f96c24983a42cadb1b4f8bc",
        "onnxruntime-win-x64-1.28.0/lib/onnxruntime.dll",
        "18370c375f07357fa5874344a9d9ac17e6b6fe1eb18b1dd209d79483b4470257",
    );
    let onnx_runtime = ModuleManifest {
        id: String::from("onnx_runtime"),
        name: String::from("ONNX Runtime (CLIP)"),
        version: String::from(ONNX_RUNTIME_MODULE_VERSION),
        description: String::from(
            "Shared library the CLIP text and vision encoders load at runtime for semantic search.",
        ),
        kind: ModuleKind::Runtime,
        license: String::from("MIT"),
        is_optional: true,
        dependencies: vec![],
        platforms: onnx_platforms,
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
        modules: vec![clip_text, clip_vision, clip_tokenizer, duckdb, onnx_runtime],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::manifest::{ArchiveFormat, ModuleKind};

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
                if platform.archive_format == ArchiveFormat::Raw {
                    // The archive is the file, so both hashes are the same one.
                    assert_eq!(
                        &file.sha256, &platform.checksum,
                        "{}: file checksum differs from the archive checksum",
                        module.id
                    );
                } else {
                    assert_eq!(
                        file.sha256.len(),
                        64,
                        "{}: {} is not pinned by a sha256",
                        module.id,
                        file.relative_path
                    );
                }
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

    #[test]
    fn duckdb_is_a_downloadable_runtime_engine() {
        let catalog = default_catalog();
        let duckdb = catalog
            .find_module(crate::analytics::DUCKDB_MODULE_ID)
            .expect("the catalog must offer the DuckDB engine");
        assert_eq!(duckdb.kind, ModuleKind::Runtime);
        assert!(duckdb.is_optional, "search works without the engine");
        assert_eq!(duckdb.version, DUCKDB_MODULE_VERSION);

        let host = duckdb
            .platform_for_current_target()
            .expect("the host platform must have a DuckDB build");
        assert_eq!(host.archive_format, ArchiveFormat::Zip);
        assert_eq!(host.files.len(), 1);
        let binary = &host.files[0];
        assert!(binary.executable, "the extracted engine has to be runnable");
        assert_ne!(
            binary.sha256, host.checksum,
            "the archive and the binary inside it cannot share a checksum"
        );
        // Every published platform is offered, so the same catalog serves the
        // installer for each supported OS.
        assert_eq!(duckdb.platforms.len(), 5);
        for platform in duckdb.platforms.values() {
            assert!(
                platform.url.contains(DUCKDB_RELEASE_TAG),
                "url {} is not pinned to a release",
                platform.url
            );
        }
    }

    #[test]
    fn onnx_runtime_is_a_downloadable_runtime_library() {
        let catalog = default_catalog();
        let onnx = catalog
            .find_module("onnx_runtime")
            .expect("the catalog must offer the ONNX Runtime");
        assert_eq!(onnx.kind, ModuleKind::Runtime);
        assert!(onnx.is_optional, "search falls back without the runtime");
        assert_eq!(onnx.version, ONNX_RUNTIME_MODULE_VERSION);

        let host = onnx
            .platform_for_current_target()
            .expect("the host platform must have an ONNX Runtime build");
        assert_eq!(host.files.len(), 1);
        let lib = &host.files[0];
        assert!(!lib.executable, "a shared library is not run directly");
        assert_ne!(
            lib.sha256, host.checksum,
            "the archive and the library inside cannot share a checksum"
        );
        for platform in onnx.platforms.values() {
            assert!(
                platform.url.contains(ONNX_RUNTIME_RELEASE_TAG),
                "url {} is not pinned to a release",
                platform.url
            );
        }
        // ONNX Runtime 1.28 publishes no macOS x64 asset, so the catalog cannot
        // offer it and the daemon cannot build for that target with `load-dynamic`.
        assert!(!onnx.platforms.contains_key("macos_x86_64"));
    }
}
