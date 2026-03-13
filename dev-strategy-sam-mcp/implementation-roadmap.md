# Implementation Roadmap — sam-mcp

**Strategy:** Steel Thread — one tool at a time, full vertical slice, then expand.
**Transport:** stdio (rmcp v1.2.0)
**Execution model:** return resolved command string; no shell execution in MCP server.

---

## Design: two tools

### Tool 1 — `list_aliases` (discovery)

Dedicated exploration tool with filtering. Called once (or a few times) to find the right alias before resolution begins.

```
list_aliases(namespace=null, keyword=null)
→ all aliases across all namespaces

list_aliases(namespace="docker")
→ aliases in the docker namespace only

list_aliases(keyword="run")
→ aliases where name or desc contains "run" (case-insensitive)

list_aliases(namespace="docker", keyword="run")
→ intersection: docker namespace AND matches "run"
```

Response: `[{ name, namespace, desc, template }]`

Filtering is applied in the server (no index needed — alias count is small).

---

### Tool 2 — `resolve_alias` (state machine)

Called repeatedly until resolution is complete. Each call returns one of two states:

```
// State 1: vars incomplete → next variable + its choices
resolve_alias(alias="docker::run", vars={})
→ { status: "needs_var", var: { name, namespace, desc, kind: "choices", choices: ["nginx", "redis"] } }

resolve_alias(alias="docker::run", vars={"docker::image": "nginx:latest"})
→ { status: "needs_var", var: { name, namespace, desc, kind: "input", prompt: "Enter host port" } }

// State 2: all vars provided → resolved command
resolve_alias(alias="docker::run", vars={"docker::image": "nginx:latest", "docker::port": "8080"})
→ { status: "resolved", commands: ["docker run -p 8080:80 nginx:latest"] }
```

The agent drives the loop. Each call is self-contained — no server session state.
Variable ordering follows SAM's dependency resolution (`execution_sequence_for_dependencies`).

---

## Phase 0 — Extract `sam-config` crate

**Objective:** Make `AppSettings` available to `sam-mcp` without pulling in TUI/clap deps.

**Tasks:**
1. Create `sam-config/` crate with `Cargo.toml`
2. Move `sam-cli/src/config.rs` → `sam-config/src/lib.rs`; adjust module paths
3. Add `sam-config` to workspace `Cargo.toml` members
4. Update `sam-cli/Cargo.toml` to depend on `sam-config`; fix imports in `sam-cli/src/`
5. `cargo build -p sam-cli` passes

**Done when:** `AppSettings` is re-exported from `sam-config`; sam-cli build is green.

---

## Phase 1 — Crate scaffold + `list_aliases` (Steel Thread)

**Objective:** Working end-to-end: MCP client → `list_aliases` → config load → filtered alias list. Validates rmcp integration, config loading, and the async/sync boundary before adding complexity.

**Tasks:**

### 1.1 — Create `sam-mcp` crate
- `sam-mcp/Cargo.toml` with deps:
  ```toml
  rmcp = { version = "1.2.0", features = ["server", "macros"] }
  tokio = { version = "1", features = ["full"] }
  serde = { version = "1", features = ["derive"] }
  serde_json = "1"
  schemars = "0.8"
  thiserror = "1"
  sam-config = { path = "../sam-config" }
  sam-core = { path = "../sam-core" }
  sam-persistence = { path = "../sam-persistence" }
  sam-readers = { path = "../sam-readers" }
  sam-utils = { path = "../sam-utils" }
  ```
- Add `sam-mcp` to workspace `Cargo.toml`

### 1.2 — `loader.rs` — load SAM context
```rust
pub struct SamContext {
    pub aliases: AliasesRepository,
    pub vars: VarsRepository,
    pub cache: RustBreakCache,
    pub env_variables: HashMap<String, String>,
}
pub fn load() -> Result<SamContext, LoadError>
```
Mirror `environment::from_settings()` from `sam-cli/src/environment.rs:116-152`, minus TUI/executor.
Blocking; called once at startup before serving.

### 1.3 — `tools/list_aliases.rs`
Input struct:
```rust
#[derive(Debug, Deserialize, JsonSchema)]
struct ListAliasesRequest {
    /// Filter to a specific namespace (e.g. "docker"). Optional.
    namespace: Option<String>,
    /// Case-insensitive keyword matched against alias name and description. Optional.
    keyword: Option<String>,
}
```

Logic:
```
aliases = ctx.aliases.aliases()                          // Vec<&Alias>

if let Some(ns) = namespace:
    aliases.retain(|a| a.namespace() == Some(ns))

if let Some(kw) = keyword:
    let kw_lower = kw.to_lowercase()
    aliases.retain(|a|
        a.name().to_lowercase().contains(&kw_lower) ||
        a.desc().to_lowercase().contains(&kw_lower)
    )

→ return JSON array of { name, namespace, desc, template }
```

### 1.4 — `server.rs` + `main.rs`
```rust
// server.rs
#[tool(tool_box)]
impl SamMcpServer {
    #[tool(description = "List SAM aliases. Filter by namespace and/or keyword (matched against name and description).")]
    async fn list_aliases(&self, #[tool(aggr)] req: ListAliasesRequest)
        -> Result<CallToolResult, McpError> { ... }
}

// main.rs
#[tokio::main]
async fn main() {
    let ctx = Arc::new(loader::load().expect("failed to load sam config"));
    SamMcpServer { ctx }.serve(stdio()).await.unwrap();
}
```

**Done when:** Agent calls `list_aliases()` → all aliases; `list_aliases(namespace="docker")` → docker aliases only; `list_aliases(keyword="run")` → filtered by keyword.

---

## Phase 2 — `resolve_alias` (state machine)

**Objective:** Agent can pick an alias from the list and interactively resolve all its variables one at a time.

**Tasks:**

### 2.1 — `var_resolver.rs` — cache-first choice resolution
```rust
pub fn choices_for_var(
    var: &Var,
    cache: &dyn VarsCache,
    env: &HashMap<String, String>,
) -> Result<VarState, ResolveError>
```
Returns one of:
- `VarState::Static(Vec<Choice>)` — from `var.choices()`
- `VarState::Dynamic(Vec<Choice>)` — cache hit, or live `sh -c from_command` + cache write
- `VarState::Input(String)` — `var.prompt()`, agent supplies free text

Blocking shell execution wrapped in `tokio::task::spawn_blocking`.

### 2.2 — `tools/resolve_alias.rs`
Input struct:
```rust
#[derive(Debug, Deserialize, JsonSchema)]
struct ResolveAliasRequest {
    /// Alias identifier: "namespace::name" or "name"
    alias: String,
    /// Variable values collected so far: "namespace::var" or "var" → chosen value
    vars: HashMap<String, String>,
}
```

Logic:
```
parse alias → Identifier → aliases.get(&id) or McpError::not_found

exec_seq = execution_sequence_for_dependencies(&ctx.vars, alias)  // ordered list

for var_id in exec_seq:
    if vars.contains_key(&var_id.to_string()): continue
    var = ctx.vars.get(&var_id)
    state = choices_for_var(var, &ctx.cache, &ctx.env)
    → return { status: "needs_var", var: { name, namespace, desc, kind, choices/prompt } }

// all vars satisfied:
choices_map: HashMap<Identifier, Vec<Choice>> = vars.iter()
    .map(|(k, v)| (parse_id(k), vec![Choice::new(v, None)]))
    .collect()
resolved = alias.with_choices(&choices_map)?
→ return { status: "resolved", commands: resolved.commands() }
```

### 2.3 — Register in `server.rs`
Add `resolve_alias` to the `#[tool(tool_box)]` impl block.

**Done when:** Agent discovers an alias via `list_aliases`, then calls `resolve_alias` repeatedly — one variable prompted per call — until `status: resolved` returns the final command string.

---

## Phase 2 — Polish + release

**Tasks:**
1. Rich tool description string with usage example (helps LLM understand the state machine pattern)
2. Error messages: alias not found → suggest similar aliases; var parse failure → show expected format
3. `--config <path>` CLI flag to override default SAM config location
4. `sam-mcp/README.md` with Claude Desktop `claude_desktop_config.json` snippet:
   ```json
   {
     "mcpServers": {
       "sam": {
         "command": "/path/to/sam-mcp"
       }
     }
   }
   ```
5. `cargo build --release -p sam-mcp` in CI / Makefile

**Done when:** Binary ships; user adds it to Claude Desktop and agent can resolve any SAM alias interactively.

---

## Dependency graph

```
sam-config  ←── sam-cli (existing)
     ↑
sam-mcp ──→ sam-core
        ──→ sam-persistence
        ──→ sam-readers
        ──→ sam-utils
        ──→ rmcp
        ──→ tokio
```

## Risk notes

| Risk | Mitigation |
|---|---|
| rmcp docs ~38% coverage | Use `examples/` in modelcontextprotocol/rust-sdk as primary reference |
| Blocking code (rustbreak, process::Command) in async context | Wrap in `tokio::task::spawn_blocking` |
| `execution_sequence_for_dependencies` requires sync call in async handler | `spawn_blocking` or accept that it's CPU-bound and fast |
| AppSettings extraction breaks sam-cli | Phase 0 is small; gate Phase 1 on green sam-cli build |
| `from_command` vars with shell pipelines | Use `sh -c` as executor (same as sam-terminals does) |
