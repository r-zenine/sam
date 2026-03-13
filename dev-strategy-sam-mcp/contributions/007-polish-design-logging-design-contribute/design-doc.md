# Design Document — MCP Server Logging

## Decision: `tracing` to a file, human-readable, env-var-controlled level

Initialize a `tracing-subscriber` file appender in `main.rs` before serving.
Add `tracing::info!` / `debug!` / `error!` call-sites in `server.rs`.
No changes to any other crate.

---

## Why This Design

**Constraints from implementation:**
- MCP stdio transport: stdout is exclusively reserved for JSON-RPC messages.
  Any log output to stdout corrupts the protocol — logs MUST go elsewhere.
- The server is already fully working; this is an additive polish pass only.
- Existing code has zero logging infrastructure — clean slate.

**User priority:** Developer debugging — tail the log file to follow what's happening.
Not AI agent visibility (MCP `notifications/message` deferred — YAGNI).

**Simplicity rationale:** One subscriber, one output, no JSON, no rotation.
`tracing` is the standard for tokio async apps and is already in the ecosystem
used by `rmcp` internally. Human-readable format supports `tail -f` directly.

---

## How It Works

### New dependencies (sam-mcp/Cargo.toml)

```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-appender = "0.2"
```

### Initialization (main.rs)

```rust
let log_path = std::env::var("SAM_MCP_LOG_FILE")
    .unwrap_or_else(|_| {
        dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("sam-mcp")
            .join("sam-mcp.log")
            .to_string_lossy()
            .into_owned()
    });

let log_file = std::path::Path::new(&log_path);
std::fs::create_dir_all(log_file.parent().unwrap()).ok();
let file = std::fs::OpenOptions::new()
    .create(true).append(true).open(log_file)?;

let level = std::env::var("SAM_MCP_LOG").unwrap_or_else(|_| "info".into());
let filter = tracing_subscriber::EnvFilter::new(level);

tracing_subscriber::fmt()
    .with_env_filter(filter)
    .with_writer(std::sync::Mutex::new(file))
    .with_ansi(false)
    .init();

tracing::info!(version = env!("CARGO_PKG_VERSION"), "sam-mcp started");
```

Default log path: `~/.local/share/sam-mcp/sam-mcp.log`
Override: `SAM_MCP_LOG_FILE=/tmp/sam-mcp.log`
Level override: `SAM_MCP_LOG=debug`

### Call-sites (server.rs)

| Location | Level | Fields |
|---|---|---|
| `list_aliases` entry | DEBUG | `ns`, `keyword` |
| `list_aliases` exit | INFO | result count |
| `resolve_alias` entry | DEBUG | `alias`, `vars_count` |
| `resolve_alias` → `needs_var` | DEBUG | `var_name`, `kind` |
| `resolve_alias` → `resolved` | INFO | `alias`, `commands_count` |
| alias not found | ERROR | `alias`, suggested names |
| var not found | ERROR | `var_id` |

---

## What We're NOT Doing

- **MCP `notifications/message`** — AI agent visibility not needed yet (YAGNI)
- **JSON structured logs** — human-readable is fine for `tail -f` debugging
- **Log rotation** — single append file; rotation is an OS/logrotate concern
- **Per-request trace IDs** — overkill for a local CLI tool
- **`#[instrument]` macros** — manual call-sites give enough signal without span overhead
- **`dirs` crate** — use `$HOME/.local/share` via env expansion to avoid new dep

---

## Success Criteria

- `tail -f ~/.local/share/sam-mcp/sam-mcp.log` shows tool calls in real time
- `SAM_MCP_LOG=debug` exposes cache and filter detail
- Zero output to stdout at runtime (protocol intact)
- `cargo build -p sam-mcp` passes with the 3 new deps
