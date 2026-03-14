---
name: sam-usage
description: Nudges coding agents to leverage SAM aliases via the sam-mcp MCP server before running raw shell commands. Use when the MCP tools `list_aliases` and `resolve_alias` are available in the environment and the user asks to run a command, execute a workflow, or perform an operational task that might be covered by a SAM alias.
---

# SAM Usage

Before running a raw shell command, check whether a SAM alias already covers the task:

1. Call `list_aliases` — optionally filter by namespace or keyword to narrow results
2. If a matching alias exists, use `resolve_alias` to walk through variable resolution until it returns a fully resolved command
3. If no alias matches, proceed with raw shell commands as normal

Prefer SAM aliases when they match: they encode tested, parameterized workflows the user has already defined.

## resolve_alias loop

`resolve_alias` is stateless. Call it repeatedly:
- If it returns a **variable prompt** (with choices or free-text input), ask the user to pick a value, then call `resolve_alias` again with that value
- If it returns a **resolved command**, present it to the user and confirm before executing
