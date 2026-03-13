# Context Document — sam-mcp

## Behavioral Spec

Build a read-only MCP (Model Context Protocol) server for SAM, implemented as a new Rust crate `sam-mcp` inside the existing workspace.

The server allows an AI agent to:
1. **List aliases** — discover all SAM aliases across all namespaces (name, namespace, description, template string)
2. **Inspect an alias** — get the full alias detail including its required variables (their names, descriptions, and resolution type: static / from_command / from_input)
3. **Get variable choices** — retrieve the set of valid choices for a variable, using the SAM vars cache first (TTL-based, RON format via rustbreak), falling back to running the `from_command` live if cache is cold
4. **Resolve an alias** — given an alias identifier and a map of `var_name → chosen_value`, return the fully substituted command string(s), ready for the agent to run

The server runs over **stdio** transport (compatible with Claude Desktop, Cursor, and all standard MCP clients) and does **not** execute commands itself.

---

## Architecture Summary

### Workspace structure (current)
```
sam/
├── sam-cli/        # binary; clap CLI, config loading, engine dispatch
├── sam-core/       # zero-I/O domain; Alias, Var, Identifier, Choice, SamEngine
├── sam-persistence/# rustbreak file state: vars cache (RON+TTL), history, sessions
├── sam-readers/    # YAML parsing (serde_yaml) for aliases and vars
├── sam-tui/        # terminal fuzzy-search Resolver implementation
├── sam-terminals/  # shell/tmux process helpers
├── sam-utils/      # filesystem utilities
└── Cargo.toml      # workspace root
```

### New crate: `sam-mcp`
```
sam-mcp/
├── Cargo.toml
└── src/
    ├── main.rs       # tokio entry point; build server, serve(stdio())
    ├── server.rs     # SamMcpServer struct + ServerHandler impl
    ├── loader.rs     # load AliasesRepository + VarsRepository from AppSettings
    ├── cache.rs      # VarChoicesResolver: cache-first, live fallback
    └── tools/
        ├── mod.rs
        ├── list_aliases.rs
        ├── get_alias.rs
        ├── get_var_choices.rs
        └── resolve_alias.rs
```

### Key dependencies (new crate only)
| Crate | Version | Role |
|---|---|---|
| `rmcp` | 1.2.0 | MCP server SDK (official, 5M downloads) |
| `tokio` | 1 | async runtime (required by rmcp) |
| `serde` / `serde_json` | 1 | JSON serialisation for tool I/O |
| `schemars` | 0.8 | JSON Schema generation from Rust types (required by rmcp macros) |
| `thiserror` | 1 | error types |
| `sam-core` | workspace | Alias, Var, Identifier, Choice, AliasCollection, VarsCollection |
| `sam-persistence` | workspace | RustBreakCache, VarsCache trait |
| `sam-readers` | workspace | read_aliases_from_path, read_vars_repository |
| `sam-utils` | workspace | fsutils |

`sam-cli/src/config.rs` (AppSettings) will be moved or re-exposed so `sam-mcp` can load it without depending on the entire CLI crate.

### How SAM config loading works (reuse pattern)
```
AppSettings::load(None)
  → reads ~/.config/sam/config.toml (or $SAM_CONFIG)
  → yields aliases_files() + vars_files() + cache_dir() + ttl()

read_aliases_from_path(&f) → Vec<Alias>   [sam-readers]
AliasesRepository::new(iter)               [sam-persistence]

read_vars_repository(&f) → VarsRepository [sam-readers + sam-persistence]
VarsRepository::merge(other)               [sam-persistence]

RustBreakCache::with_ttl(cache_dir, ttl)  [sam-persistence]
```

### Variable resolution: cache-first strategy
```
get_var_choices(var_id):
  1. cache.get(var_id) → Some(choices) if TTL valid → return
  2. Var::from_command? → run shell command → parse stdout lines → cache.set → return
  3. Var::choices (static) → return directly (no cache needed)
  4. Var::from_input → return prompt string (agent must supply free text)
```

### Alias resolution pipeline
```
resolve_alias(alias_id, {var → value}):
  1. aliases.get(alias_id) → &Alias
  2. Build HashMap<Identifier, Vec<Choice>> from user-provided values
  3. alias.with_choices(&choices) → ResolvedAlias
  4. resolved_alias.commands() → Vec<String>  ← return this
```

---

## Research Findings

### MCP SDK: rmcp v1.2.0
- **Official SDK** maintained by the MCP org (modelcontextprotocol/rust-sdk)
- 5.1M total downloads, 3.2k GitHub stars, 41 releases, active maintenance
- Full MCP 2025-11-25 spec compliance
- Transport: stdio (for this project), also Streamable HTTP, SSE
- API: `#[tool(tool_box)]` proc macro on impl block + `#[tool(description = "...")]` on methods
- Tool args: `#[derive(Deserialize, JsonSchema)]` structs; auto-generates JSON Schema
- `ServerHandler` trait: implement `get_info()` to declare capabilities
- Startup: `MyServer.serve(stdio()).await?`

### Macro pattern (confirmed from rmcp examples)
```rust
#[derive(Debug, Deserialize, JsonSchema)]
struct ListAliasesRequest {}

#[tool(tool_box)]
impl SamMcpServer {
    #[tool(description = "List all available SAM aliases across namespaces")]
    async fn list_aliases(&self, #[tool(aggr)] _req: ListAliasesRequest) -> Result<CallToolResult, McpError> {
        // ...
    }
}

impl ServerHandler for SamMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
```

### Async constraint
`rmcp` requires `tokio`. The current SAM codebase is synchronous. The MCP server will be async at the boundary (tool methods) but delegate to sync `sam-core` / `sam-persistence` code via `tokio::task::spawn_blocking` where needed (e.g., cache reads, shell command execution for `from_command` vars).
