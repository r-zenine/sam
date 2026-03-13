# Design Decisions - Testing Design

## Primary Design Decision

**Test level**: Unit tests on `SamMcpServer` methods directly, not via MCP transport.
**Why**: Private request types (`ListAliasesRequest`, `ResolveAliasRequest`) force tests
into `server.rs` anyway. Transport layer is rmcp's concern. User explicitly chose
speed and simplicity over end-to-end confidence.
**Rejected**: YAML fixture integration (more realistic but same coverage for our scope);
stdio end-to-end (catches more but requires compiled binary, far heavier).
**Impact**: Tests can call async tool methods directly — no JSON-RPC serialization needed.

## Supporting Decisions

**D1 — 4 tests, not more**: User priority is "just enough to detect breakage on core features".
Input var path and error cases are explicitly out of scope. → **Impact**: Implementer writes
exactly 4 test functions plus 1 fixture helper.

**D2 — Fixture construction via existing constructors**: Use `AliasesRepository::new()` /
`VarsRepository::new()` + SAM entity constructors (`Alias::new`, `Var::new`, `Choice::new`).
Avoids file I/O except for the required cache tempdir. → **Impact**: No external test fixtures
directory needed; fixtures are inline in the test module.

**D3 — Cache via tempdir**: `RustBreakCache` requires a real path. Use `tempfile::TempDir` in
the fixture helper. → **Impact**: Add `tempfile` as a `[dev-dependency]` in `sam-mcp/Cargo.toml`.
