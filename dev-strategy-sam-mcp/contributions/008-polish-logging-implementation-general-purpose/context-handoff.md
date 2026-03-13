# Context Handoff — Logging Implementation

## What Was Built

`sam-mcp` now writes structured logs to a file. To follow the server in real time:

```
tail -f ~/.local/share/sam-mcp/sam-mcp.log
# or
SAM_MCP_LOG=debug SAM_MCP_LOG_FILE=/tmp/sam.log sam-mcp
```

## What Works / What's Fragile / What's Missing

**Works:**
- `init_logging()` in `main.rs` — called before serving, creates parent dirs, appends to file
- 7 call-sites in `server.rs` — DEBUG on entry, INFO on success, ERROR on not-found paths
- Level and path fully env-var-controlled; no config file needed

**Fragile:**
- Log init silently no-ops if the file can't be opened (e.g., bad `SAM_MCP_LOG_FILE` path)
  — no indication to the user that logging was skipped

**Missing (deferred):**
- Log rotation — single append file grows unbounded; OS logrotate handles this if needed
- MCP `notifications/message` — protocol-level logging for AI agent visibility (YAGNI)

## Notes for Next Contributors

- Zero stdout writes — MCP protocol is intact; logs go only to the file
- `tracing-appender` was NOT added (design doc listed it but it wasn't needed)
- All existing tests pass; no test changes were required
