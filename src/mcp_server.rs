//! MCP server implementation for detect using manual JSON-RPC

use crate::parse_and_run_fs;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use slog::{debug, error, info, o, Drain, Logger};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    method: String,
    id: Option<Value>,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

pub async fn run_mcp_server() -> Result<()> {
    // Set up logging to stderr for MCP mode
    let plain = slog_term::PlainSyncDecorator::new(std::io::stderr());
    let logger = Logger::root(slog_term::FullFormat::new(plain).build().fuse(), o!());

    info!(logger, "Detect MCP server starting...");

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    // Read messages line by line
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        debug!(logger, "Received: {}", line);

        // Parse the JSON-RPC message
        match serde_json::from_str::<JsonRpcRequest>(&line) {
            Ok(req) => {
                info!(logger, "Request: {} (id: {:?})", req.method, req.id);

                let response = match req.method.as_str() {
                    "initialize" => {
                        debug!(logger, "Handling initialization");
                        JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id: req.id,
                            result: Some(serde_json::json!({
                                "protocolVersion": "2024-11-05",
                                "capabilities": {
                                    "tools": {}
                                },
                                "serverInfo": {
                                    "name": "detect",
                                    "version": env!("CARGO_PKG_VERSION")
                                }
                            })),
                            error: None,
                        }
                    }
                    "tools/list" => {
                        debug!(logger, "Handling tools/list");

                        JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id: req.id,
                            result: Some(serde_json::json!({
                                "tools": [
                                    {
                                        "name": "detect",
                                        "description": "Search filesystem using expressive queries. Combines file metadata and content search in a single command.\n\n## Selectors\n• name/filename - full filename with extension (e.g., \"main.rs\")\n• ext/extension - file extension without dot (e.g., \"rs\")\n• stem - filename without extension (e.g., \"main\")\n• parent/dir - parent directory name\n• path - full relative path from search root\n• contents/content/text - file contents\n• size - file size in bytes\n• type - file type (file, dir, symlink)\n• modified/mtime, created/ctime, accessed/atime - timestamps\n\n## Operators\n• Comparison: ==, !=, >, <, >=, <=\n• Pattern: ~= (regex), contains (substring)\n• Membership: in [value1, value2]\n• Boolean: AND, OR, NOT, ( )\n\n## Units & Formats\n• Size: b, kb, mb, gb (case-insensitive, e.g., 10MB == 10mb)\n• Time: -7d, -1h, yesterday, 2024-01-15\n\n## Quoting Rules\n• Required: patterns with spaces\n• Optional: most patterns (name ~= (foo|bar) works unquoted)\n• Backslash works unquoted: \\d+, \\w{3}, \\(\n\n## Examples\n• ext == rs - All Rust files\n• size > 5mb AND modified > -7d - Large recent files\n• contents ~= async.*fn - Files with async functions\n• name ~= test AND NOT contents contains skip - Test files without skip\n• ext in [js,ts] AND contents ~= import.*React - JS/TS importing React\n• (ext == rs OR ext == toml) AND size > 10kb - Precedence with parentheses\n• content contains \"TODO: fix this\" - Quoted strings with spaces\n• modified > -7d - Files modified in last 7 days\n• path ~= src/parser/.* - Path-based filtering",
                                        "inputSchema": {
                                            "type": "object",
                                            "properties": {
                                                "expression": {
                                                    "type": "string",
                                                    "description": "The detect expression to evaluate (e.g., 'ext == rs && content contains TODO')"
                                                },
                                                "directory": {
                                                    "type": "string",
                                                    "description": "The directory to search in (absolute path). Defaults to current directory if not specified."
                                                },
                                                "include_gitignored": {
                                                    "type": "boolean",
                                                    "description": "Include files that are gitignored (default: false)",
                                                    "default": false
                                                },
                                                "max_results": {
                                                    "type": "integer",
                                                    "description": "Maximum number of results to return (default: 20, use 0 for unlimited)",
                                                    "default": 20,
                                                    "minimum": 0
                                                }
                                            },
                                            "required": ["expression"]
                                        }
                                    }
                                ]
                            })),
                            error: None,
                        }
                    }
                    "resources/list" => {
                        debug!(logger, "Handling resources/list");
                        JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id: req.id,
                            result: Some(serde_json::json!({"resources": []})),
                            error: None,
                        }
                    }
                    "prompts/list" => {
                        debug!(logger, "Handling prompts/list");
                        JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id: req.id,
                            result: Some(serde_json::json!({"prompts": []})),
                            error: None,
                        }
                    }
                    "tools/call" => {
                        debug!(logger, "Tool call params: {:?}", req.params);

                        let params = req.params.unwrap_or(Value::Null);
                        let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                        let tool_args = params.get("arguments").cloned().unwrap_or(Value::Null);

                        // Distinguish between success and error results
                        enum ToolResult {
                            Success(Value),
                            Error(String),
                        }

                        let result = match tool_name {
                            "detect" => {
                                let expression =
                                    tool_args.get("expression").and_then(|e| e.as_str());

                                if let Some(expr) = expression {
                                    let expression = expr.to_string();

                                    let directory = tool_args
                                        .get("directory")
                                        .and_then(|d| d.as_str())
                                        .map(PathBuf::from)
                                        .unwrap_or_else(|| {
                                            std::env::current_dir()
                                                .unwrap_or_else(|_| PathBuf::from("."))
                                        });

                                    let include_gitignored = tool_args
                                        .get("include_gitignored")
                                        .and_then(|i| i.as_bool())
                                        .unwrap_or(false);

                                    // Validate max_results parameter
                                    let max_results_validation: Result<usize, String> =
                                        match tool_args.get("max_results") {
                                            Some(val) => {
                                                // Check if it's a number (could be negative)
                                                if let Some(num) = val.as_i64() {
                                                    if num < 0 {
                                                        Err(format!("max_results must be non-negative, got: {}", num))
                                                    } else {
                                                        Ok(num as usize)
                                                    }
                                                } else if let Some(num) = val.as_u64() {
                                                    Ok(num as usize)
                                                } else {
                                                    Err("max_results must be a number".to_string())
                                                }
                                            }
                                            None => Ok(20), // Default value
                                        };

                                    // Check if validation failed
                                    match max_results_validation {
                                        Err(err_msg) => ToolResult::Error(err_msg),
                                        Ok(max_results) => {
                                            info!(
                                        logger,
                                        "Running detect with expression: {} in directory: {:?}",
                                        expression,
                                        directory
                                    );

                                            // Canonicalize directory for relative path computation
                                            let canonical_dir = directory
                                                .canonicalize()
                                                .unwrap_or_else(|_| directory.clone());

                                            // Run detect directly with await since we're in an async function
                                            let detect_logger = logger.clone();
                                            let mut results = Vec::new();
                                            let detect_result = parse_and_run_fs(
                                                detect_logger,
                                                &directory,
                                                !include_gitignored,
                                                expression,
                                                |path| {
                                                    if max_results == 0
                                                        || results.len() < max_results
                                                    {
                                                        // Convert to relative path for cleaner output
                                                        let display_path = path
                                                            .strip_prefix(&canonical_dir)
                                                            .unwrap_or(path)
                                                            .to_string_lossy()
                                                            .to_string();
                                                        results.push(display_path);
                                                    }
                                                },
                                            )
                                            .await;

                                            match detect_result {
                                                Ok(_) => ToolResult::Success(serde_json::json!({
                                                    "matches": results,
                                                    "count": results.len(),
                                                    "truncated": false
                                                })),
                                                Err(e) => ToolResult::Error(format!(
                                                    "Detect failed: {}",
                                                    e
                                                )),
                                            }
                                        }
                                    }
                                } else {
                                    ToolResult::Error("Missing 'expression' parameter".to_string())
                                }
                            }
                            _ => ToolResult::Error(format!("Unknown tool: {}", tool_name)),
                        };

                        // Build response based on success/error
                        match result {
                            ToolResult::Success(value) => JsonRpcResponse {
                                jsonrpc: "2.0".to_string(),
                                id: req.id,
                                result: Some(serde_json::json!({
                                    "content": [{
                                        "type": "text",
                                        "text": serde_json::to_string(&value)?
                                    }]
                                })),
                                error: None,
                            },
                            ToolResult::Error(msg) => JsonRpcResponse {
                                jsonrpc: "2.0".to_string(),
                                id: req.id,
                                result: None,
                                error: Some(JsonRpcError {
                                    code: -32000, // Application error
                                    message: msg,
                                    data: None,
                                }),
                            },
                        }
                    }
                    _ => {
                        debug!(logger, "Unknown method: {}", req.method);
                        continue;
                    }
                };

                // Send response
                let response_str = serde_json::to_string(&response)?;
                stdout.write_all(response_str.as_bytes())?;
                stdout.write_all(b"\n")?;
                stdout.flush()?;
                debug!(logger, "Sent response for {}", req.method);
            }
            Err(e) => {
                error!(logger, "Parse error: {}", e);
            }
        }
    }

    info!(logger, "Server exiting");
    Ok(())
}
