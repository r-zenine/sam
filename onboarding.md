# sam - Orientation Guide

## What This Project Does

`sam` is a CLI tool that lets users define parameterized shell command templates (called **aliases**) with dynamic variables, then interactively select values for those variables through a terminal UI before executing the final command. It solves the problem of remembering and typing complex multi-step shell commands with dynamic parts (e.g. selecting a running Docker container before running `docker inspect`).

---

## Before You Code Here

**Existing Patterns:**
- All persistent state (cache, sessions, history) goes through `AssociativeStateWithTTL` or `SequentialState` in `sam-persistence`, both backed by `rustbreak` with RON serialization. Do not introduce a new persistence mechanism.
- Namespaces are derived automatically from the directory name of the YAML file that defines the alias or var. The `NamespaceUpdater::update_from_path` trait method handles this — do not assign namespaces manually anywhere else.
- Variable resolution is always driven by a topologically-sorted execution sequence computed by `execution_sequence_for_dependencies`. Never resolve vars in ad-hoc order; always go through this function.

**Reusable DTOs/Types:**
- `Identifier` — the canonical name+namespace key for any alias or variable. Use `Identifier::from_str`, `Identifier::new`, or `Identifier::with_namespace`. Do not use raw strings as keys.
- `Choice` — a selected value with an optional description. The value is a plain string; the description is optional. Always use `Choice::new` or `Choice::from_value`.
- `ResolvedAlias` — the output of a fully resolved alias execution: original template + chosen values + final commands. Used in history.
- `VarsRepository` — the merged, default-aware collection of all vars loaded from disk. The single source of truth for var lookups at runtime.
- `AliasesRepository` — the fully expanded (alias-in-alias substitution already applied) collection of all aliases.

**Integration Points:**
- `sam-cli` is the only binary crate. It wires all other crates together inside `environment::from_settings`.
- `sam-core` defines all domain types and traits, with zero I/O. It must not depend on any other `sam-*` crate.
- `sam-tui` implements the `Resolver` trait from `sam-core`, bridging the domain layer to the terminal UI.
- `sam-readers` owns all YAML parsing; it is the only place that calls `serde_yaml`.
- `sam-persistence` owns all file-backed state; it is the only place that calls `rustbreak`.

---

## Key Abstractions to Reuse

**`Identifier` (`sam-core/src/entities/identifiers.rs`)**
Name + optional namespace pair. Implements `Hash`/`Eq` so it can be used as a `HashMap` key. Parses `{{ var }}` and `{{ ns::var }}` template syntax automatically. The `::` separator is the namespace delimiter everywhere.

**`Var` (`sam-core/src/entities/vars.rs`)**
A variable definition. Exactly one of three modes is active:
- `choices: Vec<Choice>` — static list presented to the user.
- `from_command: Option<String>` — shell command whose stdout provides choices (one per line, tab-separated value/description).
- `from_input: Option<String>` — free-text prompt string; user types the value directly.

Implements the `Command` trait (exposes its command string) and the `Dependencies` trait (parses `{{ }}` references out of `from_command`).

**`Alias` (`sam-core/src/entities/aliases.rs`)**
A command template. The `alias` field is a shell string with `{{ var }}` placeholders and `[[ other_alias ]]` embed references. At load time `AliasesRepository` expands `[[ ]]` references inline. Implements `Dependencies` to expose its variable requirements.

**`AssociativeStateWithTTL<V>` (`sam-persistence/src/associative_state.rs`)**
Generic TTL-aware key-value store backed by a single RON file via `rustbreak`. Used by both the vars cache (`RustBreakCache`) and session storage (`SessionStorage`). All entries carry a `when: u64` unix timestamp; expired entries are evicted on write.

**`SamEngine` (`sam-core/src/engines/sam_engine.rs`)**
The orchestration core. Accepts a `Resolver` (TUI), an `AliasCollection`, a `VarsCollection`, and a `VarsDefaultValues` source, then drives the full select-resolve-execute flow. Parameterized by generic traits, so it is testable with mocks.

---

## Architectural Constraints

- `sam-core` has **no I/O dependencies** — no file system, no `rustbreak`, no `serde_yaml`. All I/O is pushed to the outer crates. This boundary must not be broken.
- Namespace assignment is **path-derived only**. The parent directory name of a YAML file becomes the namespace of every alias and var defined in that file. There is no explicit namespace field in the YAML.
- The `Resolver` trait (in `sam-core/src/algorithms/resolver.rs`) is the **only** way the engine obtains user choices. The TUI (`UserInterfaceV2`) is the production implementation. Any new selection mechanism must implement this trait.
- Default values follow a **strict priority**: CLI `--choices` flags override session defaults, which override nothing (they are only used when no CLI default exists). This merge order is enforced in `AppSettings::merge_session_defaults`.
- All `from_command` var outputs are **cached by the command string** as the cache key. The TTL comes from `ttl` in `.sam_rc.toml`. Cache is bypassed with `--no-cache`.
- Session identity is determined at runtime from environment variables in this priority: `TERM_SESSION_ID` > `TMUX_PANE` > `SSH_CLIENT` > `PPID` > `"terminal_session"`.

---

## Core Data Models

### Var (YAML: `vars.yaml`)
```yaml
# Static choices
- name: pager
  desc: the pager tool to use
  choices:
    - value: less
      desc: use less

# Dynamic choices from a shell command (output: one choice per line, tab-separated value\tdesc)
- name: file
  desc: file selection
  from_command: "ls -1 {{ directory }}"

# Free-text user input
- name: value
  desc: some input
  from_input: "enter a value"
```

### Alias (YAML: `aliases.yaml`)
```yaml
- name: list_stuff
  desc: list current directory
  alias: "cd {{directory}} && {{pager}} {{file}}"

# Embed another alias by reference with [[ ]]
- name: echo_HOME
  desc: prints home dir
  alias: "[[docker::echo_input]] && echo $HOME"

# Reference vars in other namespaces with ns::var_name
- name: cross_ns
  desc: cross-namespace example
  alias: "cd {{docker::directory}} && {{kubernetes::pager}} {{docker::file}}"
```

### Session Entry (RON file: `~/.cache/sam/session_storage`)
Keyed by `"<session_id>:<namespace>::<var_name>"`. Each entry stores `{ var_name, choice, session_id }` with a timestamp for TTL.

### History Entry (RON file: `~/.local/share/sam/history`)
A `Vec` of `HistoryEntry { r: ResolvedAlias, pwd: String }`. Capped at 1000 entries (oldest dropped).

### Cache Entry (RON file: `~/.cache/sam/sam`)
Keyed by the verbatim command string. Stores `{ name, command, output }` with timestamp for TTL.

---

## How Commands Are Configured and Stored

1. User creates a directory tree with `aliases.yaml` and `vars.yaml` files.
2. `$HOME/.sam_rc.toml` (or `.sam_rc.toml` in the current directory, which takes precedence) declares `root_dir` pointing at those directories and a `ttl` in seconds.
3. `AppSettings` walks every `root_dir` recursively, collecting all files named `aliases.yaml`/`aliases.yml` and `vars.yaml`/`vars.yml`.
4. `sam-readers` parses them with `serde_yaml` and applies path-derived namespaces via `NamespaceUpdater`.
5. `AliasesRepository::new` expands all `[[ ]]` embed references at load time (not at execution time).
6. `VarsRepository::new` merges all var files into a single `HashSet<Var>`.

**`~/.sam_rc.toml` format:**
```toml
root_dir = ["./examples/oneliners/", "~/.sam"]
ttl = 1800          # seconds; controls from_command cache TTL

# Arbitrary key=value pairs become env variables available in aliases as $KEY
MY_TOKEN = "abc123"
```

---

## How the CLI Works

**Entry point:** `sam-cli/src/main.rs` -> `run()` -> `cli::read_cli_request()` then `run_command()`.

**Subcommands (defined in `sam-cli/src/cli.rs`):**

| Subcommand | Action |
|---|---|
| _(none)_ / `run` | Open TUI to choose and execute an alias |
| `alias <name>` | Execute a named alias directly |
| `run-last` / `%` | Re-execute the last alias from history |
| `show-last` / `s` | Display the last executed alias |
| `history` | Interactive TUI over history |
| `check-config` | Validate all config files |
| `cache-clear` | Clear the vars cache |
| `cache-keys` | List all cache keys |
| `cache-keys-delete` | Interactive delete of cache entries |
| `session-set <var=value>` | Pin a variable value for the current session |
| `session-clear` | Clear all session-pinned values |
| `session-list` | List all session-pinned values |

**Global flags:**
- `-d` / `--dry` — resolve vars but print the final command instead of running it.
- `-s` / `--silent` — suppress the vars cache (don't write `from_command` outputs).
- `-n` / `--no-cache` — bypass the cache entirely (don't read or write).
- `-c ns::var=value` — supply a default choice for a variable inline (can repeat; requires namespace).

**Startup sequence:**
1. Parse CLI args into `CLIRequest { command, settings }`.
2. Load `AppSettings` from TOML.
3. Load session defaults and merge into `AppSettings` (session < CLI flags).
4. Construct `Environment`: read YAML files, build repositories, open cache and history.
5. Dispatch to the appropriate engine (`SamEngine`, `HistoryEngine`, `CacheEngine`, `ConfigEngine`, `SessionEngine`).

---

## Directory Map

```
sam/
├── sam-cli/          # Binary crate: entry point, CLI parsing, engines wiring, environment init
│   └── src/
│       ├── main.rs          # Entry point
│       ├── cli.rs           # Clap argument definitions and SubCommand enum
│       ├── config.rs        # AppSettings: loads .sam_rc.toml, resolves file paths
│       ├── environment.rs   # Builds Environment struct from AppSettings
│       ├── session_engine.rs # session-set / session-clear / session-list logic
│       ├── cache_engine.rs   # cache-* subcommands
│       ├── history_engine.rs # history / run-last / show-last
│       ├── config_engine.rs  # check-config
│       └── executors.rs      # SamExecutor impl: actually forks/runs shell commands
│
├── sam-core/         # Domain logic, zero I/O: entities, algorithms, engine traits
│   └── src/
│       ├── entities/        # Var, Alias, Choice, Identifier, ResolvedAlias, ...
│       ├── algorithms/      # Dependency resolution, execution sequencing, Resolver trait
│       └── engines/         # SamEngine, SamCommand, trait definitions (SamExecutor, SamLogger, ...)
│
├── sam-persistence/  # All file-backed state: cache, sessions, history, repositories
│   └── src/
│       ├── associative_state.rs  # Generic TTL key-value store (rustbreak + RON)
│       ├── sequential_state.rs   # Generic append-only list store (rustbreak + RON)
│       ├── vars_cache.rs         # RustBreakCache / NoopVarsCache implementing VarsCache
│       ├── session_storage.rs    # SessionStorage: per-session variable pins
│       ├── history_aliases.rs    # AliasHistory: last-N resolved alias log
│       └── repositories/        # VarsRepository, AliasesRepository
│
├── sam-readers/      # YAML parsing only: reads aliases.yaml and vars.yaml into domain types
│
├── sam-tui/          # Terminal UI: implements Resolver trait using a modal fuzzy-search view
│
├── sam-terminals/    # Shell/tmux process helpers (ShellCommand, etc.)
│
├── sam-utils/        # Filesystem utilities (walk_dir, TempFile, etc.)
│
└── examples/oneliners/  # Example config directory: shows expected aliases.yaml/vars.yaml layout
    ├── aliases.yaml
    ├── vars.yaml
    └── docker/            # Subdirectory = "docker" namespace
        ├── aliases.yaml
        └── vars.yaml
```

---

## Storage Formats Summary

| What | File path | Format |
|---|---|---|
| User config | `$HOME/.sam_rc.toml` or `./.sam_rc.toml` | TOML |
| Alias definitions | `<root_dir>/**/aliases.yaml` | YAML (array of `{name, desc, alias}`) |
| Var definitions | `<root_dir>/**/vars.yaml` | YAML (array of `{name, desc, choices|from_command|from_input}`) |
| Vars cache | `$HOME/.cache/sam/sam` | RON (rustbreak key-value with TTL timestamps) |
| Session storage | `$HOME/.cache/sam/session_storage` | RON (rustbreak key-value with TTL timestamps) |
| Command history | `$HOME/.local/share/sam/history` | RON (rustbreak append-only Vec) |
