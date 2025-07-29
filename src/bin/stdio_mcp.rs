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
    #[serde(default)]
    include_gitignored: bool,
    #[serde(default = "default_max_results")]
    max_results: usize,
}

fn default_max_results() -> usize {
    20
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
            "description": "Search filesystem entities using a powerful expression language with boolean logic, regex matching, and temporal queries.\n\nCORE CONCEPTS:\n• Selectors start with @ and target different file attributes\n• Operators compare/match values with support for regex, globs, sets\n• Boolean logic (&&, ||, !) combines conditions\n• Parentheses control evaluation order\n\nESSENTIAL SELECTORS:\n@name, @path    - filename/path matching\n@ext            - file extension\n@size           - file size in bytes\n@contents       - file content search\n@modified       - modification time\n@type           - entity type (file, dir, etc.)\n\nCORE OPERATORS:\n==, !=          - exact match/not equal\n~=              - regex match\ncontains        - substring search\nglob            - shell-style wildcards\nin [a,b,c]      - set membership\n>, <, >=, <=    - numeric/time comparison\n\nQUICK START EXAMPLES:\n@name ~= \"\\.rs$\"                   - Find Rust files\n@size > 1000000                    - Files over 1MB\n@contents ~= \"TODO|FIXME\"          - Files needing attention\n@modified > \"-1.week\"              - Recently modified\n@ext in [js,ts,jsx,tsx]           - JavaScript family\n\nCOMBINED CONTENT + METADATA PATTERNS:\n@ext == py && @contents ~= \"class.*Test\"     - Python test classes\n@name ~= \"config\" && @contents ~= \"password|secret\"  - Config files with secrets\n@size < 10000 && @contents ~= \"struct.*pub\" && @ext == rs  - Small public Rust structs\n@modified > \"-7.days\" && @contents ~= \"impl.*trait\" && @path ~= \"/src/\"  - Recent trait implementations\n@name ~= \".*test.*\" && @contents ~= \"panic|unwrap\" && @ext in [rs,go]  - Test files with potential issues\n\nPOWER USER PATTERNS:\n(@ext == rs && @size < 10000) || @name ~= \"test_.*\"     - Small Rust files or tests\n@type == file && @contents ~= \"struct.*pub\" && !@path ~= \"/target/\"  - Public structs, skip build\n@modified > \"-30.days\" && @size > 0 && @ext in [rs,py,js] && @contents ~= \"TODO\"  - Recent TODOs in code\n\nTEMPORAL SYNTAX:\n\"-7.days\", \"-2.weeks\", \"-1.month\", \"-6.hours\" - Relative time\n\"2024-01-15\" - Absolute dates\n\nPATTERN MATCHING GUIDE:\n• Simple substring: @name contains \"test\"\n• Shell wildcards: @name glob \"*test*.rs\" \n• Regex patterns: @name ~= \"test_.*\\.rs$\"\n\nESCAPING & QUOTING:\n• Quotes only needed for strings with spaces: @name == \"my file.txt\"\n• Simple strings work unquoted: @type == file, @path ~= /target/\n• Case sensitivity varies by selector (filesystem-dependent for @name/@path)",
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
    let mut total_count = 0;
    let root = Path::new(&detect_params.directory);
    let max_results = detect_params.max_results;
    
    // Use tokio runtime to run the async function
    let runtime = tokio::runtime::Runtime::new()?;
    let result = runtime.block_on(async {
        detect::parse_and_run_fs(
            logger,
            root,
            !detect_params.include_gitignored, // respect_gitignore (note the negation)
            detect_params.expression.clone(),
            |path| {
                total_count += 1;
                
                // Only collect up to max_results (unless limit is 0 which means unlimited)
                if max_results == 0 || matches.len() < max_results {
                    // Convert to relative path if possible
                    let display_path = match path.strip_prefix(root) {
                        Ok(relative) => relative.display().to_string(),
                        Err(_) => path.display().to_string(),
                    };
                    matches.push(display_path);
                }
            },
        ).await
    });

    // Handle any parsing or execution errors
    match result {
        Ok(()) => {},
        Err(e) => {
            match e {
                detect::error::DetectError::ParseError { message, hint, location } => {
                    // Build error message with location if available
                    let mut error_msg = message.clone();
                    if let Some((line, col)) = location {
                        error_msg.push_str(&format!(" at line {}, column {}", line, col));
                    }
                    
                    // Add hint if available
                    if let Some(hint) = hint {
                        error_msg.push_str(&format!("\n\n{}", hint));
                    } else {
                        // Fall back to generic hints
                        let hints = detect::error_hints::get_parse_error_hints();
                        error_msg.push_str(&format!("\n\n{}", hints));
                    }
                    
                    return Err(anyhow::anyhow!("{}", error_msg));
                }
                detect::error::DetectError::Other(err) => {
                    return Err(err);
                }
            }
        }
    }
    
    let was_limited = max_results > 0 && total_count > max_results;
    let files_text = if was_limited {
        format!("{}
\n[Showing {} of {} total matches]", matches.join("\n"), max_results, total_count)
    } else {
        format!("{}
\n[{} matches found]", matches.join("\n"), total_count)
    };

    Ok(json!({
        "content": [{
            "type": "text",
            "text": files_text
        }],
        "metadata": {
            "files_found": total_count,
            "directory": detect_params.directory,
            "expression": detect_params.expression,
            "was_limited": was_limited,
            "max_results": max_results
        }
    }))
}