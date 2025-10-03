//! MCP server implementation for detect using manual JSON-RPC

use crate::parse_and_run_fs;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use slog::{o, Drain, Logger};
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
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
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .target(env_logger::Target::Stderr)
        .init();

    log::info!("Detect MCP server starting...");

    // Create a logger for detect operations
    let plain = slog_term::PlainSyncDecorator::new(std::io::stderr());
    let detect_logger = Logger::root(slog_term::FullFormat::new(plain).build().fuse(), o!());

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    // Read messages line by line
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        log::debug!("Received: {}", line);

        // Parse the JSON-RPC message
        match serde_json::from_str::<JsonRpcRequest>(&line) {
            Ok(req) => {
                log::info!("Request: {} (id: {:?})", req.method, req.id);

                let response = match req.method.as_str() {
                    "initialize" => {
                        log::debug!("Handling initialization");
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
                        log::debug!("Handling tools/list");

                        JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id: req.id,
                            result: Some(serde_json::json!({
                                "tools": [
                                    {
                                        "name": "detect",
                                        "description": "Search filesystem using expressive queries. Combines file metadata and content search in a single command.\n\nExamples:\n• 'ext == rs' - All Rust files\n• 'size > 1mb AND modified > -7d' - Large recent files\n• 'contents contains TODO' - Files with TODOs\n• 'name ~= test AND NOT contents contains skip' - Test files without skip\n• '*.{js,ts} AND contents ~= import.*React' - JS/TS files importing React\n\nSupports: path patterns, content search, size/date filters, regex, boolean logic (AND/OR/NOT)",
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
                                    },
                                    {
                                        "name": "detect_help",
                                        "description": "Get help for detect's query language, including all operators, selectors, and examples",
                                        "inputSchema": {
                                            "type": "object",
                                            "properties": {}
                                        }
                                    }
                                ]
                            })),
                            error: None,
                        }
                    }
                    "resources/list" => {
                        log::debug!("Handling resources/list");
                        JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id: req.id,
                            result: Some(serde_json::json!({"resources": []})),
                            error: None,
                        }
                    }
                    "prompts/list" => {
                        log::debug!("Handling prompts/list");
                        JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id: req.id,
                            result: Some(serde_json::json!({"prompts": []})),
                            error: None,
                        }
                    }
                    "tools/call" => {
                        log::debug!("Tool call params: {:?}", req.params);

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

                                    let max_results = tool_args
                                        .get("max_results")
                                        .and_then(|m| m.as_u64())
                                        .unwrap_or(20)
                                        as usize;

                                    log::info!(
                                        "Running detect with expression: {} in directory: {:?}",
                                        expression,
                                        directory
                                    );

                                    // Run detect directly with await since we're in an async function
                                    let logger = detect_logger.clone();
                                    let mut results = Vec::new();
                                    let detect_result = parse_and_run_fs(
                                        logger,
                                        &directory,
                                        !include_gitignored,
                                        expression,
                                        |path| {
                                            if max_results == 0 || results.len() < max_results {
                                                results.push(path.to_string_lossy().to_string());
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
                                        Err(e) => {
                                            ToolResult::Error(format!("Detect failed: {}", e))
                                        }
                                    }
                                } else {
                                    ToolResult::Error("Missing 'expression' parameter".to_string())
                                }
                            }
                            "detect_help" => ToolResult::Success(serde_json::json!({
                                "help": format!(
                                    "# Detect Query Language Reference\n\n\
                                    ## Basic Syntax\n\
                                    `selector operator value` OR glob patterns like `*.rs`\n\n\
                                    ## Common Selectors\n\
                                    • **Path**: name (filename), ext (extension), stem, parent, path (full)\n\
                                    • **Content**: contents, content, text (all equivalent)\n\
                                    • **Metadata**: size, type, modified (mtime/mdate), created (ctime/cdate), accessed (atime/adate)\n\n\
                                    ## Operators\n\
                                    • **Comparison**: == != > < >= <=\n\
                                    • **Pattern**: ~= (regex), contains (substring)\n\
                                    • **Membership**: in [value1, value2, ...]\n\
                                    • **Boolean**: AND OR NOT ( )\n\n\
                                    ## Size Units\n\
                                    b, kb (k), mb (m), gb (g), tb (t)\n\n\
                                    ## Time Formats\n\
                                    • Relative: -7d, -1h, -30m (negative = past)\n\
                                    • Keywords: now, today, yesterday\n\
                                    • Absolute: 2024-01-15\n\n\
                                    ## Examples\n\
                                    ```\n\
                                    # Find Rust files with async code\n\
                                    ext == rs AND contents ~= async\n\n\
                                    # Large files modified recently\n\
                                    size > 5mb AND modified > -7days\n\n\
                                    # Test files without 'skip' markers\n\
                                    name contains test AND NOT contents contains skip\n\n\
                                    # Config files with potential secrets\n\
                                    ext in [json, yml, env] AND contents ~= (password|secret|api_key)\n\n\
                                    # Using glob patterns\n\
                                    *.{{js,ts}} AND size > 10kb\n\
                                    **/*.md AND modified > yesterday\n\
                                    ```\n\n\
                                    ## Tips\n\
                                    • Glob patterns can be mixed with boolean expressions\n\
                                    • Path filters are evaluated first for performance\n\
                                    • Regex patterns use Rust regex, with automatic PCRE2 fallback if Rust regex parsing fails\n\
                                    • Use NOT instead of ! to avoid shell issues"
                                )
                            })),
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
                        log::debug!("Unknown method: {}", req.method);
                        continue;
                    }
                };

                // Send response
                let response_str = serde_json::to_string(&response)?;
                stdout.write_all(response_str.as_bytes())?;
                stdout.write_all(b"\n")?;
                stdout.flush()?;
                log::debug!("Sent response for {}", req.method);
            }
            Err(e) => {
                log::error!("Parse error: {}", e);
            }
        }
    }

    log::info!("Server exiting");
    Ok(())
}
