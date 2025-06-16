use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::path::Path;
use slog::{Drain, Logger};

const GRAMMAR: &str = include_str!("../expr/expr.pest");

#[derive(Debug, Serialize, Deserialize)]
struct DetectParams {
    expression: String,
    directory: String,
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .target(env_logger::Target::Stderr)
        .init();

    log::info!("Starting detect MCP server");

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        log::debug!("Received: {}", line);

        let request: Value = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
                log::error!("Failed to parse request: {}", e);
                let response = json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": -32700,
                        "message": "Parse error",
                        "data": e.to_string()
                    }
                });
                writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
                stdout.flush()?;
                continue;
            }
        };

        let response = handle_request(request);
        
        // Don't send a response for notifications
        if !response.is_null() {
            let response_str = serde_json::to_string(&response)?;
            log::debug!("Sending: {}", response_str);
            writeln!(stdout, "{}", response_str)?;
            stdout.flush()?;
        }
    }

    Ok(())
}

fn handle_request(request: Value) -> Value {
    let id = request.get("id").cloned();
    let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let params = request.get("params").cloned();

    // Handle notifications (no id field, no response expected)
    if id.is_none() && method.starts_with("notifications/") {
        log::debug!("Received notification: {}", method);
        // For notifications, return null to indicate no response should be sent
        return Value::Null;
    }

    let result = match method {
        "initialize" => handle_initialize(),
        "tools/list" => handle_list_tools(),
        "resources/list" => handle_list_resources(),
        "prompts/list" => handle_list_prompts(),
        "tools/call" => {
            if let Some(params) = params {
                handle_call_tool(params)
            } else {
                Err(anyhow::anyhow!("Missing params for tools/call"))
            }
        }
        _ => Err(anyhow::anyhow!("Unsupported method: {}", method)),
    };

    match result {
        Ok(result) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        }),
        Err(e) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32603,
                "message": e.to_string(),
                "data": null
            }
        }),
    }
}

fn handle_initialize() -> Result<Value> {
    Ok(json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "detect-mcp",
            "version": "0.1.0"
        }
    }))
}

fn handle_list_tools() -> Result<Value> {
    Ok(json!({
        "tools": [{
            "name": "detect",
            "description": "Search for files using detect expression language. See the grammar property for the full PEST grammar syntax.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "expression": {
                        "type": "string",
                        "description": "The detect expression to evaluate (e.g., 'name ~= \"*.rs\"')"
                    },
                    "directory": {
                        "type": "string",
                        "description": "The directory to search in (absolute path)"
                    }
                },
                "required": ["expression", "directory"]
            },
            "grammar": GRAMMAR
        }]
    }))
}

fn handle_list_resources() -> Result<Value> {
    Ok(json!({
        "resources": []
    }))
}

fn handle_list_prompts() -> Result<Value> {
    Ok(json!({
        "prompts": []
    }))
}

fn handle_call_tool(params: Value) -> Result<Value> {
    let name = params.get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing tool name"))?;
    
    if name != "detect" {
        return Err(anyhow::anyhow!("Unknown tool: {}", name));
    }

    let args = params.get("arguments")
        .cloned()
        .unwrap_or(json!({}));
    
    let detect_params: DetectParams = serde_json::from_value(args)?;
    
    log::info!("Running detect with expression: {} in directory: {}", 
               detect_params.expression, detect_params.directory);

    // Create a logger for the detect library
    let decorator = slog_term::PlainSyncDecorator::new(std::io::stderr());
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let logger = Logger::root(drain, slog::o!());

    // Collect matching files
    let mut matches = Vec::new();
    let root = Path::new(&detect_params.directory);
    
    // Use tokio runtime to run the async function
    let runtime = tokio::runtime::Runtime::new()?;
    let result = runtime.block_on(async {
        detect::parse_and_run_fs(
            logger,
            root,
            true, // respect_gitignore
            detect_params.expression.clone(),
            |path| {
                matches.push(path.display().to_string());
            },
        ).await
    });

    // Handle any parsing or execution errors
    result?;

    let files_text = matches.join("\n");

    Ok(json!({
        "content": [{
            "type": "text",
            "text": files_text
        }],
        "metadata": {
            "files_found": matches.len(),
            "directory": detect_params.directory,
            "expression": detect_params.expression
        }
    }))
}