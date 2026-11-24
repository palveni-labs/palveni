//! Model Context Protocol (MCP) Streamable HTTP (`2025-03-26`) domain types
//!.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// MCP protocol version implemented by `/api/v1/mcp`.
pub const MCP_PROTOCOL_VERSION: &str = "2025-03-26";

/// Public endpoint path for Streamable HTTP MCP clients.
pub const MCP_ENDPOINT_PATH: &str = "/api/v1/mcp";

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpToolDescriptor {
    pub name: String,
    pub description: String,
    #[schema(value_type = Object)]
    pub input_schema: Value,
    pub admin_only: bool,
    pub requires_approval: bool,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpClientConfigs {
    pub claude_desktop: String,
    pub cursor: String,
    pub openclaw: String,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpInfoResponse {
    pub protocol_version: String,
    pub endpoint_path: String,
    pub server_name: String,
    pub server_version: String,
    pub tools: Vec<McpToolDescriptor>,
    pub client_configs: McpClientConfigs,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct McpJsonRpcRequest {
    pub jsonrpc: String,
    #[serde(default)]
    #[schema(value_type = Option<Object>)]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    #[schema(value_type = Option<Object>)]
    pub params: Option<Value>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct McpJsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub data: Option<Value>,
}

#[derive(utoipa::ToSchema, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct McpJsonRpcResponse {
    pub jsonrpc: String,
    #[schema(value_type = Object, nullable = true)]
    pub id: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<Object>)]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<McpJsonRpcError>,
}

impl McpJsonRpcResponse {
    pub fn ok(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_owned(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn err(id: Value, code: i32, message: impl Into<String>, data: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_owned(),
            id,
            result: None,
            error: Some(McpJsonRpcError {
                code,
                message: message.into(),
                data,
            }),
        }
    }
}
