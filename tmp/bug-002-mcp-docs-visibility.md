# Bug Report: MCP Documentation Not Accessible to Users

## Summary
The tiered documentation system (basic + advanced via detect_help) is not visible or accessible when using the MCP tool, leaving users without critical usage information.

## Environment
- detect version: Latest (with tiered documentation)
- File structure includes: src/docs/mcp_basic.md and src/docs/mcp_advanced.md
- MCP server binary: detect-mcp

## Description
Beta testing revealed that users cannot access the carefully crafted documentation:
1. The basic documentation (mcp_basic.md) may not be properly displayed in tool description
2. The detect_help tool is listed but may not be returning documentation when called
3. Users have no way to discover the tiered operator system or usage patterns

## Steps to Reproduce
1. Connect to detect MCP server
2. List available tools
3. Try to view tool descriptions
4. Attempt to use detect_help tool

## Expected Behavior
- Basic documentation should appear in the detect tool's description
- detect_help tool should return the advanced documentation content
- Users should see the 4 core operators and examples immediately

## Actual Behavior
- Tool descriptions may be truncated or missing
- detect_help tool may not be functioning
- Users cannot access documentation through the MCP interface

## Impact
- Users don't know about the 4 core operators design
- Performance patterns aren't discoverable
- Advanced features remain hidden
- Reduces the value of the tiered documentation system

## Suggested Investigation
1. Verify `include_str!` is properly including the markdown files
2. Check if MCP protocol has length limits on tool descriptions
3. Test detect_help tool's response format
4. Consider alternative documentation delivery methods

## Priority
High - Documentation is critical for tool adoption and proper usage