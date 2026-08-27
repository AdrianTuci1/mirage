use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PersistedState {
    pub version: String,
    #[serde(default)]
    pub modules: BTreeMap<String, ModuleInstanceState>,
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            version: String::from("1"),
            modules: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModuleInstanceState {
    pub module_id: String,
    pub state: ModuleState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default)]
    pub bytes_downloaded: u64,
    #[serde(default)]
    pub bytes_total: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ModuleState {
    Missing,
    Queued,
    Downloading,
    Paused,
    Verifying,
    Ready,
    Error,
    Removing,
}

impl ModuleInstanceState {
    pub fn new(module_id: impl Into<String>) -> Self {
        let module_id = module_id.into();
        Self {
            module_id: module_id.clone(),
            state: ModuleState::Missing,
            version: None,
            bytes_downloaded: 0,
            bytes_total: 0,
            error: None,
        }
    }

    pub fn missing(module_id: impl Into<String>) -> Self {
        Self::new(module_id)
    }
}
