# sam

[![asciicast](https://asciinema.org/a/487681.svg)](https://asciinema.org/a/487681)

SAM turns multi-step shell workflows into interactive, parameterized commands. You define command templates in YAML, SAM resolves dependencies between variables automatically, prompts you to select values through a terminal UI, then executes the final command.

## Why

Complex workflows often require running 2–3 preparatory commands just to collect the values you need for the real command. With shell aliases or scripts, you either hardcode values (brittle) or rewrite the same lookup logic everywhere (tedious). SAM lets you define the lookup logic once as named variables, compose them into command templates, and reuse them across all your aliases.

Concrete example: querying metrics from a running Docker container requires selecting a container, running `docker inspect` to get its IP and port, then assembling the `curl` command. With SAM, you define those steps once and run them with a single command.

## Features

- **Dependency resolution** — variables can depend on other variables; SAM computes the execution order automatically
- **Dynamic choices** — populate selection menus from any shell command's output (`from_command`)
- **Free-text input** — prompt for typed values when a fixed list isn't appropriate (`from_input`)
- **TTL caching** — expensive `from_command` lookups are cached so repeated invocations stay fast
- **Session defaults** — set variable values for the duration of a terminal session to skip repeated prompts
- **History** — view and re-run previous commands
- **Namespaces** — directory structure maps to namespaces; aliases and variables are scoped automatically
- **Alias composition** — embed one alias inside another with `[[ namespace::alias ]]`
- **MCP server** — exposes SAM aliases to AI assistants (Claude, etc.) via the Model Context Protocol

## Install

**macOS (Homebrew):**
```bash
brew tap r-zenine/sam
brew install sam
```

**Linux / macOS (binary):**

Download the latest binary for your platform from the [releases page](https://github.com/r-zenine/sam/releases).

**Build from source:**
```bash
cargo build --release -p sam-cli
# binary: target/release/sam
```

## Quick start

```bash
# run an alias interactively (select from all available)
sam run

# run a specific alias directly
sam alias docker::get_metrics

# preview the resolved command without executing
sam alias docker::get_metrics --dry

# re-run the last command
sam run-last
```

To try the bundled examples:
```bash
cargo run --bin sam run
```

See a real-world configuration: [r-zenine/oneliners](https://github.com/r-zenine/oneliners)

## Configuration

Create `~/.sam_rc.toml`:

```toml
# one or more directories containing your aliases and vars files
root_dir = ["~/.sam", "~/work/.sam"]

# how long (in seconds) to cache from_command outputs
ttl = 1800

# arbitrary key/value pairs available as environment variables in your aliases
REGISTRY = "registry.example.com"
```

## Defining aliases

Aliases live in `aliases.yaml` files. A template can reference variables with `{{ var_name }}` and embed other aliases with `[[ namespace::alias_name ]]`.

```yaml
- name: get_metrics
  desc: Query metrics from a running container
  alias: curl http://{{ container_ip }}:{{ container_port }}/metrics

- name: deploy
  desc: Tag and push an image, then restart the service
  alias: "[[ docker::tag_image ]] && [[ kubernetes::rollout ]]"
```

SAM resolves all variable references before executing. If an alias embeds another alias, that alias's variables are resolved first.

## Defining variables

Variables live in `vars.yaml` files alongside your `aliases.yaml`. There are three kinds:

### Static choices

```yaml
- name: environment
  desc: target environment
  choices:
    - value: staging
      desc: staging environment
    - value: production
      desc: production environment
```

### Dynamic choices from a command

The command runs at resolution time. Each line of output becomes one choice. If a line contains a tab (`\t`), the part before the tab is the value and the part after is the description.

```yaml
- name: container
  desc: a running Docker container
  from_command: 'docker ps --format="{{.ID}}\t{{.Names}}"'

- name: container_ip
  desc: IP address of the selected container
  from_command: "docker inspect {{ container }} | jq -r '.[0].NetworkSettings.IPAddress'"
```

SAM detects that `container_ip` depends on `container` and prompts for `container` first. The output is cached for `ttl` seconds.

### Free-text input

```yaml
- name: tag
  desc: Docker image tag
  from_input: "enter a tag (e.g. v1.2.3)"
```

## Directory structure and namespaces

SAM derives namespaces from the directory tree under your `root_dir`. Files at the top level have no namespace; files in subdirectories inherit the directory name as their namespace.

```
~/.sam/
├── aliases.yaml        # no namespace  →  my_alias
├── vars.yaml
├── docker/
│   ├── aliases.yaml    # namespace: docker  →  docker::my_alias
│   └── vars.yaml       # namespace: docker  →  docker::my_var
└── kubernetes/
    ├── aliases.yaml    # namespace: kubernetes
    └── vars.yaml
```

Cross-namespace references work in both variable templates and alias templates:

```yaml
# in docker/aliases.yaml
- name: push
  alias: docker push {{ kubernetes::registry }}/{{ image }}:{{ tag }}
```

## CLI reference

| Command | Description |
|---|---|
| `sam run` | Select and run an alias interactively |
| `sam alias <name>` | Run a specific alias by name (e.g. `docker::push`) |
| `sam history` | Show previously executed commands |
| `sam run-last` / `sam %` | Re-run the last command |
| `sam show-last` / `sam s` | Print the last command without running it |
| `sam check-config` | Validate your configuration and alias files |
| `sam cache-clear` | Clear all cached `from_command` outputs |
| `sam cache-keys` | List all cache keys |
| `sam cache-keys-delete` | Remove specific cache entries |
| `sam session-set <var=value>` | Set a session default for a variable |
| `sam session-list` | Show active session defaults |
| `sam session-clear` | Clear all session defaults |

### Flags

| Flag | Description |
|---|---|
| `--dry` / `-d` | Print the resolved command without executing |
| `--choices <var=value>` | Pre-supply a variable value, skipping its prompt |
| `--silent` / `-s` | Run without caching `from_command` outputs |
| `--no-cache` / `-n` | Disable caching entirely for this invocation |

`--choices` can be specified multiple times:
```bash
sam alias docker::push --choices docker::image=nginx --choices docker::tag=latest
```

## Keybindings

When selecting variable values in the TUI:

| Key | Action |
|---|---|
| `↑` / `↓` | Move selection |
| `Enter` | Confirm selection |
| `Ctrl-s` | Toggle multi-select on the current item |
| `Ctrl-a` | Select all items |

## MCP server

`sam-mcp` is an MCP server that exposes your SAM aliases to AI assistants. It provides two tools:

- **`list_aliases`** — returns all aliases, optionally filtered by namespace or keyword
- **`resolve_alias`** — resolves an alias to a runnable command via a stateless loop: each call either returns the next variable that needs a value (with its choices), or returns the final resolved command

This lets an AI assistant like Claude discover your aliases and guide you through variable selection without needing to know the underlying shell commands.

### Setup with Claude Desktop

Add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "sam": {
      "command": "/usr/local/bin/sam-mcp"
    }
  }
}
```

With a custom config file:

```json
{
  "mcpServers": {
    "sam": {
      "command": "/usr/local/bin/sam-mcp",
      "args": ["--config", "/path/to/.sam_rc.toml"]
    }
  }
}
```

Build the MCP server:
```bash
cargo build --release -p sam-mcp
# binary: target/release/sam-mcp
```
