use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModuleManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(rename = "kind")]
    pub kind: ModuleKind,
    pub license: String,
    pub is_optional: bool,
    #[serde(default)]
    pub dependencies: Vec<String>,
    pub platforms: HashMap<String, PlatformEntry>,
}

impl ModuleManifest {
    /// Select the platform entry that best matches the current host.
    /// Falls back to a `universal` entry if no host-specific platform is present.
    pub fn platform_for_current_target(&self) -> Option<&PlatformEntry> {
        let host = current_platform_key();
        self.platforms.get(&host).or_else(|| self.platforms.get("universal"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ModuleKind {
    Runtime,
    Library,
    Model,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlatformEntry {
    pub url: String,
    pub size: u64,
    pub checksum: String,
    pub archive_format: ArchiveFormat,
    pub files: Vec<FileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveFormat {
    #[serde(rename = "tar.gz")]
    TarGz,
    Zip,
    Raw,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileEntry {
    pub relative_path: String,
    pub sha256: String,
    pub executable: bool,
    pub required: bool,
}

/// Returns the platform key used inside module manifests for the host target.
pub fn current_platform_key() -> String {
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        String::from("macos_aarch64")
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        String::from("macos_x86_64")
    }
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        String::from("windows_x86_64")
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        String::from("linux_x86_64")
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        String::from("linux_aarch64")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_manifest() {
        let json = r#"{
            "id": "duckdb",
            "name": "DuckDB Analytics Engine",
            "version": "1.1.3",
            "description": "OLAP SQL engine.",
            "kind": "runtime",
            "license": "MIT",
            "is_optional": true,
            "dependencies": [],
            "platforms": {
                "universal": {
                    "url": "https://example.com/duckdb.tar.gz",
                    "size": 1024,
                    "checksum": "0000000000000000000000000000000000000000000000000000000000000000",
                    "archive_format": "tar.gz",
                    "files": [
                        {
                            "relative_path": "lib/libduckdb.dylib",
                            "sha256": "0000000000000000000000000000000000000000000000000000000000000000",
                            "executable": true,
                            "required": true
                        }
                    ]
                }
            }
        }"#;

        let manifest: ModuleManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.id, "duckdb");
        assert_eq!(manifest.kind, ModuleKind::Runtime);
        assert!(manifest.platform_for_current_target().is_some());
    }
}
