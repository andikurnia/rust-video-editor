use crate::tools::{builtin_tools, ToolDefinition};

#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("IO error: {0}")]
    Io(String),
    #[error("Protocol error: {0}")]
    Protocol(String),
}

pub struct McpServer {
    pub tools: Vec<ToolDefinition>,
}

impl McpServer {
    pub fn new() -> Self {
        McpServer {
            tools: builtin_tools(),
        }
    }

    pub fn handle_request(&self, request: &str) -> Result<String, McpError> {
        let parsed: serde_json::Value =
            serde_json::from_str(request).map_err(|e| McpError::Protocol(e.to_string()))?;

        let method = parsed
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let response = match method {
            "ping" => serde_json::json!({
                "jsonrpc": "2.0",
                "result": "pong",
                "id": parsed.get("id")
            }),
            "tools/list" => serde_json::json!({
                "jsonrpc": "2.0",
                "result": { "tools": self.tools },
                "id": parsed.get("id")
            }),
            "tools/call" => {
                let params = parsed.get("params");
                let tool_name = params
                    .and_then(|p| p.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let args = params.and_then(|p| p.get("arguments")).cloned();
                self.execute_tool(tool_name, args)
            }
            _ => serde_json::json!({
                "jsonrpc": "2.0",
                "error": { "code": -32601, "message": format!("Method not found: {method}") },
                "id": parsed.get("id")
            }),
        };

        serde_json::to_string(&response).map_err(|e| McpError::Io(e.to_string()))
    }

    fn execute_tool(&self, name: &str, _args: Option<serde_json::Value>) -> serde_json::Value {
        match name {
            "ping" => serde_json::json!({
                "jsonrpc": "2.0",
                "result": "pong"
            }),
            _ => serde_json::json!({
                "jsonrpc": "2.0",
                "error": { "code": -32602, "message": format!("Unknown tool: {name}") }
            }),
        }
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}
