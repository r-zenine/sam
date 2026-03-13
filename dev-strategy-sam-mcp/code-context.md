# Code Context — sam-mcp

Key locations to read before implementing each phase.

## Phase 0 — Config extraction

| File | Lines | Why |
|---|---|---|
| `sam-cli/src/config.rs` | all | AppSettings struct to extract into sam-config |
| `sam-cli/Cargo.toml` | all | deps to carry over / split |
| `Cargo.toml` (workspace root) | 1-11 | add sam-config + sam-mcp members |

## Phase 1 — Scaffolding + list_aliases

| File | Lines | Why |
|---|---|---|
| `sam-cli/src/environment.rs` | 116-152 | `from_settings()`: the config→repo loading pattern to replicate in loader.rs |
| `sam-core/src/engines/sam_engine.rs` | 22-47 | `AliasCollection` trait: `.aliases()` + `.get()` |
| `sam-persistence/src/repositories/` | all | `AliasesRepository::new()` API |
| `sam-readers/src/lib.rs` | all | `read_aliases_from_path()` signature |
| `sam-core/src/entities/aliases.rs` | 26-108 | `Alias` struct: `.name()`, `.namespace()`, `.desc()`, `.alias()`, `.full_name()` |
| `sam-core/src/entities/identifiers.rs` | all | `Identifier`: name + optional namespace; the universal key |

## Phase 2 — get_alias + get_var_choices

| File | Lines | Why |
|---|---|---|
| `sam-core/src/entities/vars.rs` | 1-87 | `Var` struct: `.choices()`, `.is_command()`, `.is_input()`, `.prompt()`, `.command()` |
| `sam-core/src/entities/choices.rs` | all | `Choice`: `.value()`, `.desc()` |
| `sam-persistence/src/lib.rs` | all | `VarsCache` trait + `RustBreakCache` + `NoopVarsCache` |
| `sam-persistence/src/cache.rs` (or equivalent) | all | `VarsCache::get()` / `VarsCache::set()` signatures |
| `sam-readers/src/lib.rs` | all | `read_vars_repository()` signature |
| `sam-persistence/src/repositories/` | all | `VarsRepository`: `.get()` API |
| `sam-core/src/algorithms/mod.rs` | all | `VarsCollection` trait |

## Phase 3 — resolve_alias

| File | Lines | Why |
|---|---|---|
| `sam-core/src/entities/aliases.rs` | 61-73 | `Alias::with_choices()` → `ResolvedAlias` |
| `sam-core/src/entities/aliases.rs` | 143-157 | `ResolvedAlias` struct + `.commands()` |
| `sam-core/src/entities/choices.rs` | all | `Choice::new(value, desc)` constructor |
| `sam-core/src/entities/identifiers.rs` | all | `Identifier::new()` + `Identifier::with_namespace()` |
| `sam-core/src/entities/dependencies.rs` | all | `Dependencies` trait: understand how variable deps are extracted from alias templates |
