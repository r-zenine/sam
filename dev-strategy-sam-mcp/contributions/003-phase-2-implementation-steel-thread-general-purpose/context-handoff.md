# Context Handoff - Phase 2 Implementation (resolve_alias)

## 🎯 Core Result (What agents get from this work)
**Built**: `resolve_alias` MCP tool — a stateless state machine. Each call takes `{alias, vars}` and returns either `{status:"needs_var", var:{name,namespace,desc,kind,choices/prompt}}` or `{status:"resolved", commands:[...]}`.
**Key insight**: `McpResolver` implements the `Resolver` trait (same interface as `UserInterfaceV2`) using `ShellCommand` + `read_choices` + `VarsCache` from existing crates. The handler calls `choice_for_var` from `sam_core::algorithms` for per-var dispatch — no custom resolution logic needed.

## 🚦 Current State (Agent decision points)
**✅ Solid foundation**: Both tools working; binary compiles clean; all workspace tests pass. The full vertical slice is complete: discover aliases → resolve variables one by one → get final command.
**⚠️ Needs attention**: Input vars (from_input) are signalled via error pattern-match on `NoInputWasProvided` — functional but relies on an error variant that encodes the prompt in its second field. Works correctly; just unusual.
**⏸️ Deferred**: `--config` CLI flag; rich tool descriptions; README; CI/release build (Polish phase).

## 👥 Next Agent Guidance (Specific handoff)
**Polish implementor**: Phase 2 "Polish + release" tasks are in the roadmap. The binary is usable as-is. Key items: `--config <path>` flag in `main.rs` (override default SAM config), `sam-mcp/README.md` with `claude_desktop_config.json` snippet, and `cargo build --release -p sam-mcp` in CI/Makefile.

---
## 🔗 Integration Points (Technical context)
**resolver.rs**: `McpResolver<'a>` borrows `env_variables: &HashMap<String,String>` and `cache: &dyn VarsCache`. Created per-request in the async handler; no Send/Sync needed since there are no await points between creation and drop.
**State machine in server.rs**: `execution_sequence_for_dependencies` → iterate exec_seq → skip vars already in `choices` (built from req.vars) → call `choice_for_var` → return needs_var or fall through to `alias.with_choices` for resolved.
**Input var detection**: `Err(ErrorDependencyResolution::NoChoiceForVar { error: ErrorsResolver::NoInputWasProvided(_, ref prompt), .. })` — the prompt string comes from `var.from_input` (the YAML `from_input:` field on the var).

## 📋 Reference Links
- [decision-log.yaml](decision-log.yaml) - Key design decisions (D001: use existing infra, D002: choice_for_var, D003: input signalling)
- [changelog.md](changelog.md) - Phase completion summary
