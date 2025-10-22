# Detect MCP Server

The detect tool includes built-in MCP (Model Context Protocol) support that allows AI assistants like Claude Desktop to use detect's powerful filesystem search capabilities.

Note: The detect MCP binary can be used to evaluate expressions as a command line tool e.g. `detect 'ext == rs'`. Run `detect --help` for details.

## Installation

1. Build detect with MCP support:
```bash
cargo build --release
```

Or install via cargo:
```bash
cargo install detect
```

2. Configure Claude Desktop by adding detect to your Claude Desktop configuration:
   - On macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
   - On Windows: `%APPDATA%\Claude\claude_desktop_config.json`

Add the following to your config (adjust the path to match your detect installation):

```json
{
  "mcpServers": {
    "detect": {
      "command": "/path/to/detect",
      "args": ["--mcp"],
      "env": {}
    }
  }
}
```

If installed via cargo install, the path would typically be:
- macOS/Linux: `~/.cargo/bin/detect`
- Windows: `%USERPROFILE%\.cargo\bin\detect.exe`

3. Restart Claude Desktop to load the new MCP server.

## Available Tools

### `detect`
Search filesystem using detect's expressive query language.

**Parameters:**
- `expression` (required): The detect expression to evaluate
- `directory` (optional): Directory to search in (defaults to current directory)
- `include_gitignored` (optional): Include gitignored files (default: false)
- `max_results` (optional): Maximum number of results (default: 20, 0 for unlimited)

**Examples:**
```
# Find all Rust files
expression: "*.rs"

# Find large JavaScript files with TODOs
expression: "*.js AND size > 10kb AND content contains TODO"

# Find recently modified Python files
expression: "*.py AND modified > -7days"
```

## Testing the MCP Server

You can test the MCP server manually using JSON-RPC:

```bash
# Test initialization
echo '{"jsonrpc":"2.0","method":"initialize","id":1,"params":{}}' | detect --mcp

# List available tools
echo '{"jsonrpc":"2.0","method":"tools/list","id":2,"params":{}}' | detect --mcp

# Run a search
echo '{"jsonrpc":"2.0","method":"tools/call","id":3,"params":{"name":"detect","arguments":{"expression":"*.rs","max_results":5}}}' | detect --mcp
```

## Query Language Quick Reference

The detect MCP server supports the full detect query language:

- **File patterns**: `*.rs`, `**/*.js`, `test_*.txt`
- **Path selectors**: `name`, `basename`, `ext`, `dir`, `path`
- **Content search**: `content contains TODO`, `content ~= regex`
- **Metadata**: `size > 1mb`, `modified > -7days`, `type == file`
- **Boolean logic**: `AND`/`OR`/`NOT` or `&&`/`||`/`!`
- **Sets**: `extension in [js, ts, jsx]`

For complete documentation, use `detect --help`, `detect --examples`, `detect --predicates`, and `detect --operators`.