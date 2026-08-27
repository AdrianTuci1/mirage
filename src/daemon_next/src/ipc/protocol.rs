use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
    pub id: Option<Value>,
}

impl JsonRpcRequest {
    pub fn new(method: impl Into<String>, params: Option<Value>, id: Option<Value>) -> Self {
        Self {
            jsonrpc: String::from("2.0"),
            method: method.into(),
            params,
            id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: Option<Value>,
}

impl JsonRpcResponse {
    pub fn new_result(result: Value, id: Option<Value>) -> Self {
        Self {
            jsonrpc: String::from("2.0"),
            result: Some(result),
            error: None,
            id,
        }
    }

    pub fn new_error(code: i32, message: impl Into<String>, id: Option<Value>) -> Self {
        Self {
            jsonrpc: String::from("2.0"),
            result: None,
            error: Some(JsonRpcError::new(code, message)),
            id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }
}

pub const ERROR_PARSE_ERROR: i32 = -32700;
pub const ERROR_INVALID_REQUEST: i32 = -32600;
pub const ERROR_METHOD_NOT_FOUND: i32 = -32601;
pub const ERROR_INVALID_PARAMS: i32 = -32602;
pub const ERROR_INTERNAL_ERROR: i32 = -32603;
pub const ERROR_MODULE_MISSING: i32 = -32001;
