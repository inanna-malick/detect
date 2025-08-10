//! MCP server implementation for detect using manual JSON-RPC

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use crate::parse_and_run_fs;
use slog::{o, Drain, Logger};

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
    let detect_logger = Logger::root(
        slog_term::FullFormat::new(plain).build().fuse(),
        o!(),
    );

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
                                        "description": "Search filesystem using detect's expressive query language. Examples:\n- Find Rust files: 'extension == rs'\n- Find TODOs: 'contents contains TODO'\n- Complex: 'size > 1mb AND modified > -7days AND NOT path contains node_modules'\n- Glob patterns: '*.rs' or '**/*.js'\n- Multiple conditions: '(*.rs OR *.toml) AND size > 1kb'",
                                        "inputSchema": {
                                            "type": "object",
                                            "properties": {
                                                "expression": {
                                                    "type": "string",
                                                    "description": "The detect expression to evaluate (e.g., 'ext == rs && contents contains TODO')"
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

                        let result = match tool_name {
                            "detect" => {
                                let expression = tool_args.get("expression")
                                    .and_then(|e| e.as_str());
                                
                                if expression.is_none() {
                                    serde_json::json!({
                                        "error": "Missing 'expression' parameter"
                                    })
                                } else {
                                let expression = expression.unwrap().to_string();
                                
                                let directory = tool_args.get("directory")
                                    .and_then(|d| d.as_str())
                                    .map(PathBuf::from)
                                    .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
                                
                                let include_gitignored = tool_args.get("include_gitignored")
                                    .and_then(|i| i.as_bool())
                                    .unwrap_or(false);
                                
                                let max_results = tool_args.get("max_results")
                                    .and_then(|m| m.as_u64())
                                    .unwrap_or(20) as usize;
                                
                                log::info!("Running detect with expression: {} in directory: {:?}", expression, directory);
                                
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
                                    }
                                ).await;
                                
                                match detect_result {
                                    Ok(_) => {
                                        serde_json::json!({
                                            "matches": results,
                                            "count": results.len(),
                                            "truncated": false
                                        })
                                    }
                                    Err(e) => serde_json::json!({
                                        "error": format!("Detect failed: {}", e)
                                    })
                                }
                                }
                            }
                            "detect_help" => {
                                serde_json::json!({
                                    "help": include_str!("docs/expr_guide.md")
                                })
                            }
                            _ => serde_json::json!({
                                "error": format!("Unknown tool: {}", tool_name)
                            })
                        };

                        JsonRpcResponse {
                            jsonrpc: "2.0".to_string(),
                            id: req.id,
                            result: Some(serde_json::json!({
                                "content": [{
                                    "type": "text",
                                    "text": serde_json::to_string(&result)?
                                }]
                            })),
                            error: None,
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