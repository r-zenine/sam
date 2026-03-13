# Changelog — Logging Implementation

## What Was Delivered

Added file-based logging to `sam-mcp` using `tracing` + `tracing-subscriber`.

- **`main.rs`**: `init_logging()` initializes a file-appender subscriber before serving.
  Log path defaults to `~/.local/share/sam-mcp/sam-mcp.log`; overridable via `SAM_MCP_LOG_FILE`.
  Log level defaults to INFO; overridable via `SAM_MCP_LOG=debug`.
- **`server.rs`**: 7 call-sites added — DEBUG on tool entry, INFO on success, ERROR on alias/var not found.
- **`Cargo.toml`**: 2 new deps (`tracing`, `tracing-subscriber` with `env-filter` feature).

All 4 existing tests pass; zero clippy warnings in `sam-mcp`.
