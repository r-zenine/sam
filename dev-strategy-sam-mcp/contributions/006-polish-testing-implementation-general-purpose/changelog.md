# Changelog - Testing Implementation

## What Was Built

Added 4 unit tests to `sam-mcp/src/server.rs` covering the two MCP tools' core paths.
All tests pass in < 1s. Full workspace test suite remains green.

## Tests Added

- **`list_aliases_no_filter_returns_all`** — Smoke test: both aliases returned when no filter applied.
- **`list_aliases_filtering`** — Namespace filter returns 1 of 2 aliases; keyword no-match returns 0.
- **`resolve_alias_needs_var`** — First call with no vars returns `{status: "needs_var", kind: "choices"}`.
- **`resolve_alias_resolved`** — All vars provided returns `{status: "resolved", commands: [...]}`.

## Key Implementation Note

`Alias` has both an inherent `update(String)` (updates the template) and the `NamespaceUpdater::update`
trait method (sets namespace). Rust's method resolution picks the inherent method, so UFCS is required:
`NamespaceUpdater::update(&mut alias, "namespace")`. This is the only non-obvious part of the fixture setup.
