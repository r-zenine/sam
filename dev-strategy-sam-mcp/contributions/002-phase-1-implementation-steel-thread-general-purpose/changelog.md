# Changelog - Phase 1 Implementation (Steel Thread)

## 🔍 Essential (Agents scan first)
**Delivered**: `sam-mcp` crate scaffolded and `list_aliases` tool working end-to-end. MCP client can discover all aliases, filter by namespace, and filter by keyword. Validates rmcp integration, config loading, and the async/sync boundary.
**Status**: Ready for Phase 2 — `resolve_alias`

## ✅ Validation (Strategy compliance)
**Strategy Followed**: Steel Thread — full vertical slice from MCP transport → tool handler → SAM config → filtered alias list
**Phase Objectives**: Met — `cargo build -p sam-mcp` clean; `cargo build -p sam-cli` green; all 3 sam-cli tests pass

## ➡️ Next Steps (Agent handoff)
**Unlocks**: Phase 2 — `resolve_alias` state machine tool
**Priority**: Add `var_resolver.rs` (cache-first choice resolution), then `resolve_alias` tool, then register in `server.rs`

---
## 📋 Human Context (Supporting details)
**Files Changed**: `Cargo.toml` (sam-mcp added to workspace), `sam-mcp/Cargo.toml` (new), `sam-mcp/src/main.rs` (new), `sam-mcp/src/loader.rs` (new), `sam-mcp/src/server.rs` (new)
**Testing**: `cargo build -p sam-mcp` clean (1 expected dead_code warning for Phase 2 fields); `cargo build -p sam-cli` clean; `cargo test -p sam-cli` — 3/3 pass
**Deviations from roadmap**: (1) `schemars = "1"` not "0.8" — rmcp 1.2.0 requires schemars 1.x; (2) `transport-io` feature added to rmcp — stdio() is gated behind it; (3) `#[tool_router]` + `#[tool_handler]` macros used instead of roadmap's `#[tool(tool_box)]` — that attribute doesn't exist in rmcp 1.2.0; (4) `Parameters<T>` used instead of `#[tool(aggr)]`; (5) `ServerInfo` is `#[non_exhaustive]` so mutated after `Default::default()` instead of struct literal
