# Context Handoff — Logging Design

## Problem Solved

The MCP server had zero observability — no way to follow tool calls, errors, or cache activity.
MCP stdio transport makes this non-trivial: stdout is reserved for JSON-RPC messages,
so logs must go to a file. This design specifies the simplest viable logging setup for
developer debugging via `tail -f`.

## Design Overview

- **Crate stack**: `tracing` + `tracing-subscriber` (env-filter) + `tracing-appender`
- **Output**: append-only file at `~/.local/share/sam-mcp/sam-mcp.log`
- **Level**: default INFO, override via `SAM_MCP_LOG=debug`
- **Path override**: `SAM_MCP_LOG_FILE=/custom/path.log`
- **Format**: human-readable timestamped lines (not JSON)
- **Touch points**: `main.rs` (subscriber init) + `server.rs` (7 call-sites)
- **Not doing**: MCP protocol notifications, log rotation, JSON output, `#[instrument]`

## Reading Guide

See `design-doc.md` for:
- Exact `Cargo.toml` deps to add
- Full `main.rs` initialization snippet (copy-paste ready)
- Table of 7 call-sites with level and fields
- Success criteria
