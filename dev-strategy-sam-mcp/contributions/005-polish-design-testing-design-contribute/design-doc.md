# Design Document - Minimal Regression Tests for sam-mcp

## Decision: 4 unit tests in server.rs, in-memory fixtures

4 `#[tokio::test]` functions in `#[cfg(test)] mod tests` at the bottom of
`sam-mcp/src/server.rs`. Each calls a tool method directly on `SamMcpServer`,
bypassing rmcp transport entirely.

## Why This Design

**Constraints:**
- `ListAliasesRequest` / `ResolveAliasRequest` are private to `server.rs` — tests
  must live in the same file to access them without making types pub.
- The transport layer (rmcp stdio) is not what we want to test.
- The goal is "just enough to catch regressions", not coverage completeness.

**User priority:** Minimal — 4 tests, fast, no infra.

**Simplicity rationale:** Unit tests on the server struct hit all the custom logic
(filtering, state machine loop, JSON shape) without spawning processes or touching
the filesystem at test time except for the cache tempdir.

## How It Works

**Fixture helper `make_test_ctx()`** (in the test module):
- Build `Vec<Alias>` and `Vec<Var>` using `read_aliases_from_path` /
  `read_vars_repository` pointed at small YAML files written to a `tempdir`.
- Alternatively, construct `Alias` / `Var` objects directly via their `::new()`
  constructors and feed into `AliasesRepository::new()` / `VarsRepository::new()`.
- Cache: `RustBreakCache::with_ttl(tempdir.path, ttl)` — use `tempfile` crate.
- `env_variables`: empty `HashMap`.

Fixture alias: `docker::run` with one static var `docker::image` (choices: `["nginx", "redis"]`).
Second alias: `git::push` with no vars.

**Calling tool methods:**
```
server.list_aliases(Parameters(ListAliasesRequest { .. })).await
server.resolve_alias(Parameters(ResolveAliasRequest { .. })).await
```
Unwrap `CallToolResult`, extract the text content, parse as JSON, assert on fields.

## The 4 Tests

| # | Method | Input | Assert |
|---|--------|-------|--------|
| T1 | `list_aliases` | no filter | 2 entries returned |
| T2 | `list_aliases` | `namespace="docker"` | 1 entry; `keyword="xyz"` → 0 entries |
| T3 | `resolve_alias` | `alias="docker::run", vars={}` | `status=needs_var`, `kind=choices`, choices non-empty |
| T4 | `resolve_alias` | `alias="docker::run", vars={"docker::image":"nginx"}` | `status=resolved`, commands non-empty |

## What We're NOT Doing

- **Input var path** (`kind=input`) — unusual error-as-signal path, not in the 4 core tests.
- **Alias not found error** — polish behavior, not core path.
- **`McpResolver` shell execution** — exercised by sam-terminals tests upstream.
- **stdio transport / JSON-RPC framing** — rmcp's concern, not ours.
- **`--config` flag** — trivial arg scanner, not worth a test.

## Success Criteria

- `cargo test -p sam-mcp` passes in < 3s.
- Removing or inverting the namespace filter in `list_aliases` → T2 fails.
- Removing the `resolved` branch in `resolve_alias` → T4 fails.
