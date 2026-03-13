# Context Handoff - Testing Design

## What Problem Are We Solving

`sam-mcp` has zero tests. The two core behaviors — alias filtering and the
variable state machine — could silently break. This design session determined
the minimal test strategy: 4 tests that catch regressions on the happy paths of
both tools without aiming for exhaustive coverage.

## Design Overview

**Approach**: 4 `#[tokio::test]` functions in `#[cfg(test)] mod tests` at the
bottom of `server.rs`. Tests call `list_aliases` and `resolve_alias` methods
directly on `SamMcpServer` — no transport, no file I/O except a cache tempdir.

**Tests**: no-filter list (smoke), namespace+keyword filter, `needs_var` response,
`resolved` response. Input var path and error messages are explicitly out of scope.

**Key constraint**: Private request types force tests into `server.rs` itself.
Fixtures built from entity constructors inline; `tempfile` crate for cache.

## Reading Guide

Start with `design-doc.md` — it has everything needed to implement.
Check the "The 4 Tests" table for the exact scenario spec.
Check `decision-log.md` D2/D3 for fixture construction approach.
