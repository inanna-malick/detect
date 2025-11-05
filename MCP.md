# MCP Server

detect can run as an MCP (Model Context Protocol) server, letting AI assistants like Claude Desktop search your filesystem using detect's query language.

Note: The `detect` binary works both as an MCP server (`detect --mcp`) and as a regular CLI tool (`detect 'ext == rs'`).

## Setup

Install detect:
```bash
cargo install detect
```

Add to Claude Desktop config:
- macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`
- Windows: `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "detect": {
      "command": "~/.cargo/bin/detect",
      "args": ["--mcp"]
    }
  }
}
```

Restart Claude Desktop.

## Usage

The MCP server exposes one tool:

**`detect`** - Search files with a query expression

Parameters:
- `expression` (required) - detect query
- `directory` (optional) - search path (default: current dir)
- `include_gitignored` (optional) - include .gitignore'd files (default: false)
- `max_results` (optional) - result limit (default: 20, 0 = unlimited)

Examples:
- `ext == rs` - all Rust files
- `ext == js AND size > 10kb AND content contains TODO`
- `ext == py AND modified > -7days`

See the main README for full query language documentation.