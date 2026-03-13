# Decision Log — MCP Server Logging Design

## Primary Decision: `tracing` to append-only file, human-readable

**Decision**: Use `tracing` + `tracing-subscriber` (env-filter) + append-only file writer.
Log path defaults to `~/.local/share/sam-mcp/sam-mcp.log`, overridable via `SAM_MCP_LOG_FILE`.
Level defaults to INFO, overridable via `SAM_MCP_LOG`.

**Rationale**:
- `tracing` is the standard for tokio async; rmcp uses it internally — zero impedance mismatch.
- File destination avoids stdout (protocol-breaking) and avoids stderr noise in terminal.
- Human-readable format serves the stated use case (developer `tail -f`).
- Three deps only (`tracing`, `tracing-subscriber`, `tracing-appender`) — minimal surface.

**Alternatives rejected**:
- `log` + `simplelog`: not designed for async; spans unavailable; less idiomatic in tokio ecosystem.
- stderr-only: Claude Desktop captures it, but writing to file keeps logs persistent across sessions without client-side config.
- MCP `notifications/message`: adds protocol surface area (declare capability, handle `setLevel`); user confirmed not needed now.
- JSON output: no tooling requirement stated; adds visual noise for `tail -f` use case.

---

## Supporting Decisions

### No `dirs` crate for default path

Use `$HOME/.local/share/sam-mcp/sam-mcp.log` constructed from `HOME` env var directly.
Avoids adding a new dep for a single string.

### Manual call-sites over `#[instrument]`

`#[instrument]` spans add enter/exit log lines and async overhead.
For this server, a handful of manual `tracing::info!` / `debug!` / `error!` calls at
tool boundaries are sufficient and easier to read in logs.

### Log level ERROR for alias/var not found

These are user-facing errors (bad input from agent). Level ERROR ensures they surface
at default INFO level and are easy to grep.
