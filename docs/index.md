---
layout: default
title: SAM — Shell Alias Manager
---

# SAM — Shell Alias Manager

SAM turns multi-step shell workflows into interactive, parameterized commands. You define command templates in YAML, SAM resolves variable dependencies automatically, prompts you to select values through a terminal UI, then executes the final command — with full caching and session memory.

[![asciicast](https://asciinema.org/a/487681.svg)](https://asciinema.org/a/487681)

---

## The problem

Complex workflows often require two or three preparatory commands just to gather the values you need for the real command. Consider tailing logs from a Kubernetes pod: you first need to find the namespace, then the pod, then the container name — before you can run `kubectl logs`. Writing a shell script for this is tedious. Remembering the commands is worse.

SAM lets you define the lookup logic once as named variables, wire them together declaratively, and invoke the whole thing with a single command.

---

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

---

## How it works

1. You define **variables** (`vars.yaml`) and **aliases** (`aliases.yaml`) in a directory tree.
2. Variables can be static choice lists, dynamic choices populated from a shell command's output, or free-text prompts.
3. Alias templates reference variables with `{% raw %}{{ var }}{% endraw %}`. SAM parses the dependency graph and prompts you in the right order.
4. Command outputs are cached by default (TTL configurable) so repeated invocations stay fast.
5. You can pin variable values for the duration of a terminal session to skip prompts entirely.

---

## Quick start

```bash
# Run an alias interactively — pick from everything available
sam run

# Run a specific alias directly
sam alias docker::tail_logs

# Preview the resolved command without executing it
sam alias docker::tail_logs --dry

# Re-run the last command
sam run-last
```

---

## Configuration

Create `~/.sam_rc.toml` and point it at your alias directories:

```toml
root_dir = ["~/.sam"]

# Cache TTL for from_command outputs (in seconds)
ttl = 1800

# Arbitrary key/value pairs become environment variables in your aliases
REGISTRY = "registry.example.com"
```

SAM also looks for `.sam_rc.toml` in the current directory, which takes precedence over `~/.sam_rc.toml`.

---

## Directory structure and namespaces

SAM derives namespaces from the directory structure under `root_dir`. The parent directory name of a YAML file becomes the namespace for every alias and variable defined in it.

```
~/.sam/
├── aliases.yaml        # no namespace  →  my_alias
├── vars.yaml
├── docker/
│   ├── aliases.yaml    # namespace: docker  →  docker::my_alias
│   └── vars.yaml       # namespace: docker  →  docker::container
└── kubernetes/
    ├── aliases.yaml    # namespace: kubernetes
    └── vars.yaml       # namespace: kubernetes  →  kubernetes::namespace
```

Cross-namespace references work everywhere — in variable templates and alias templates alike. An alias in the `docker` namespace can reference a variable from the `kubernetes` namespace:

```yaml
- name: push
  alias: docker push {% raw %}{{ kubernetes::registry }}{% endraw %}/{% raw %}{{ docker::image }}{% endraw %}:{% raw %}{{ docker::tag }}{% endraw %}
```

---

## Variable types

### Static choices

Presented as a fixed menu at runtime.

```yaml
- name: environment
  desc: target deployment environment
  choices:
    - value: staging
      desc: Staging environment
    - value: production
      desc: Production environment
```

### Dynamic choices from a command

The command runs at resolution time. Each line of output becomes one menu item. If a line contains a tab (`\t`), everything before the tab is the value and everything after is the description shown in the UI.

```yaml
- name: container
  desc: a running Docker container
  from_command: 'docker ps --format="{% raw %}{{.ID}}{% endraw %}\t{% raw %}{{.Names}}{% endraw %}"'
```

If a variable's command references another variable, SAM detects the dependency and prompts for the upstream variable first — automatically.

```yaml
- name: container_ip
  desc: IP address of the selected container
  from_command: "docker inspect {% raw %}{{ container }}{% endraw %} | jq -r '.[0].NetworkSettings.IPAddress'"
```

### Free-text input

When a fixed list isn't appropriate, prompt the user to type a value directly.

```yaml
- name: tag
  desc: Docker image tag
  from_input: "enter a tag (e.g. v1.2.3)"
```

---

## Alias templates

Aliases live in `aliases.yaml` files. Templates reference variables with `{% raw %}{{ var_name }}{% endraw %}` and can embed other aliases with `{% raw %}[[ namespace::alias_name ]]{% endraw %}`.

```yaml
- name: get_metrics
  desc: Query metrics from a running container
  alias: curl http://{% raw %}{{ container_ip }}{% endraw %}:{% raw %}{{ container_port }}{% endraw %}/metrics

- name: deploy
  desc: Tag and push an image, then trigger a rollout
  alias: "{% raw %}[[ docker::tag_image ]]{% endraw %} && {% raw %}[[ kubernetes::rollout ]]{% endraw %}"
```

When SAM encounters an embedded alias (`{% raw %}[[ ]]{% endraw %}`), it expands it inline at load time and resolves all of its variables as part of the same dependency graph.

---

## Real-world examples

The following examples demonstrate SAM's expressive power against tools you use every day. Each example shows a complete, working configuration you can drop into your `~/.sam` directory.

---

### Example 1 — Docker: From container selection to live metrics

**The workflow without SAM:**
```bash
# Step 1 — pick a container from the list
docker ps

# Step 2 — get its IP address
docker inspect <container_id> | jq -r '.[0].NetworkSettings.IPAddress'

# Step 3 — get its exposed port
docker inspect <container_id> | jq -r '.[0].NetworkSettings.Ports | to_entries[0].value[0].HostPort'

# Step 4 — finally, query the metrics
curl http://<ip>:<port>/metrics
```

**With SAM — define once, run forever:**

`~/.sam/docker/vars.yaml`:
```yaml
- name: container
  desc: a running Docker container
  from_command: 'docker ps --format="{% raw %}{{.ID}}{% endraw %}\t{% raw %}{{.Names}}{% endraw %}"'

- name: container_ip
  desc: IP address of the selected container
  from_command: "docker inspect {% raw %}{{ container }}{% endraw %} | jq -r '.[0].NetworkSettings.IPAddress'"

- name: container_port
  desc: first exposed port of the selected container
  from_command: "docker inspect {% raw %}{{ container }}{% endraw %} | jq -r '.[0].NetworkSettings.Ports | to_entries[0].value[0].HostPort'"
```

`~/.sam/docker/aliases.yaml`:
```yaml
- name: get_metrics
  desc: Query Prometheus metrics from a running container
  alias: curl -s http://{% raw %}{{ container_ip }}{% endraw %}:{% raw %}{{ container_port }}{% endraw %}/metrics | grep -v "^#"

- name: open_shell
  desc: Open a bash shell inside a running container
  alias: docker exec -it {% raw %}{{ container }}{% endraw %} /bin/bash

- name: tail_logs
  desc: Follow the logs of a running container
  alias: docker logs -f --tail 100 {% raw %}{{ container }}{% endraw %}
```

**Running it:**
```bash
sam alias docker::get_metrics
```

SAM detects that `container_ip` and `container_port` both depend on `container`, prompts you to select a container once, then resolves both derived variables in parallel using the cached container ID. No repeated typing.

**Scripting with pre-supplied values:**
```bash
# Non-interactively, when you already know the container
sam alias docker::tail_logs -c docker::container=my-api-container
```

---

### Example 2 — Kubernetes: Multi-level pod log streaming

Tailing logs in Kubernetes requires three pieces of information — namespace, pod, and container — each dependent on the previous. This is where SAM's dependency resolution shines.

`~/.sam/kubernetes/vars.yaml`:
```yaml
- name: namespace
  desc: Kubernetes namespace
  from_command: "kubectl get namespaces -o jsonpath='{.items[*].metadata.name}' | tr ' ' '\n'"

- name: pod
  desc: pod in the selected namespace
  from_command: "kubectl get pods -n {% raw %}{{ namespace }}{% endraw %} --no-headers -o custom-columns='NAME:.metadata.name,STATUS:.status.phase' | awk '{print $1 \"\t\" $2}'"

- name: container
  desc: container inside the selected pod
  from_command: "kubectl get pod {% raw %}{{ pod }}{% endraw %} -n {% raw %}{{ namespace }}{% endraw %} -o jsonpath='{.spec.containers[*].name}' | tr ' ' '\n'"

- name: since
  desc: how far back to stream logs
  choices:
    - value: 1m
      desc: last 1 minute
    - value: 10m
      desc: last 10 minutes
    - value: 1h
      desc: last hour
    - value: 24h
      desc: last 24 hours
```

`~/.sam/kubernetes/aliases.yaml`:
```yaml
- name: tail_logs
  desc: Stream logs from a specific container in a pod
  alias: kubectl logs -f -n {% raw %}{{ namespace }}{% endraw %} {% raw %}{{ pod }}{% endraw %} -c {% raw %}{{ container }}{% endraw %} --since={% raw %}{{ since }}{% endraw %}

- name: exec_shell
  desc: Open an interactive shell in a running pod
  alias: kubectl exec -it {% raw %}{{ pod }}{% endraw %} -n {% raw %}{{ namespace }}{% endraw %} -c {% raw %}{{ container }}{% endraw %} -- /bin/sh

- name: describe_pod
  desc: Describe a pod for debugging
  alias: kubectl describe pod {% raw %}{{ pod }}{% endraw %} -n {% raw %}{{ namespace }}{% endraw %}

- name: port_forward
  desc: Forward a pod port to localhost
  alias: kubectl port-forward pod/{% raw %}{{ pod }}{% endraw %} {% raw %}{{ local_port }}{% endraw %}:{% raw %}{{ remote_port }}{% endraw %} -n {% raw %}{{ namespace }}{% endraw %}
```

**Running it:**
```bash
sam alias kubernetes::tail_logs
```

SAM will prompt: namespace → pod (filtered to that namespace) → container (filtered to that pod) → since. Each menu is dynamically populated from your live cluster.

**Pin the namespace for the session** so you're not prompted every time:
```bash
sam session-set namespace=production
sam alias kubernetes::tail_logs   # skips namespace prompt
sam alias kubernetes::exec_shell  # skips namespace prompt here too
```

**Preview before running:**
```bash
sam alias kubernetes::tail_logs --dry
# Prints: kubectl logs -f -n production my-api-pod-7d4f9 -c api --since=10m
```

---

### Example 3 — Git: Interactive branch and commit workflows

`~/.sam/git/vars.yaml`:
```yaml
- name: branch
  desc: a local or remote git branch
  from_command: "git branch -a --format='%(refname:short)' | sed 's|origin/||' | sort -u"

- name: commit
  desc: a recent commit on the selected branch
  from_command: "git log {% raw %}{{ branch }}{% endraw %} --oneline -30 | awk '{id=$1; $1=\"\"; print id \"\t\" $0}'"

- name: remote
  desc: a configured git remote
  from_command: "git remote -v | awk '{print $1}' | sort -u"

- name: stash
  desc: an entry in the stash
  from_command: "git stash list | awk -F: '{print $1 \"\t\" $3}'"
```

`~/.sam/git/aliases.yaml`:
```yaml
- name: cherry_pick
  desc: Cherry-pick a commit from another branch onto the current branch
  alias: git cherry-pick {% raw %}{{ commit }}{% endraw %}

- name: diff_branch
  desc: Show a diff between the current branch and another branch
  alias: git diff {% raw %}{{ branch }}{% endraw %}...HEAD

- name: rebase_onto
  desc: Rebase the current branch onto a selected branch
  alias: git rebase {% raw %}{{ branch }}{% endraw %}

- name: pop_stash
  desc: Apply and drop a selected stash entry
  alias: git stash pop {% raw %}{{ stash }}{% endraw %}

- name: push_branch
  desc: Push the current branch to a selected remote
  alias: git push {% raw %}{{ remote }}{% endraw %} HEAD
```

**Running it:**
```bash
# Pick a branch, then pick one of its last 30 commits to cherry-pick
sam alias git::cherry_pick

# Or preview what branch you'd be diffing against
sam alias git::diff_branch --dry
```

---

### Example 4 — Composing aliases across namespaces

SAM's most powerful feature is alias composition. You can build higher-level workflows by embedding existing aliases inside new ones using `{% raw %}[[ namespace::alias ]]{% endraw %}`. The embedded alias's variables are resolved as part of the same dependency graph — no duplication.

`~/.sam/aliases.yaml`:
```yaml
- name: promote
  desc: Build, push, and deploy to a selected environment
  alias: "{% raw %}[[ docker::build_and_push ]]{% endraw %} && {% raw %}[[ kubernetes::rollout ]]{% endraw %}"

- name: debug_session
  desc: Tail logs and then open a shell in the same pod
  alias: "{% raw %}[[ kubernetes::tail_logs ]]{% endraw %} ; {% raw %}[[ kubernetes::exec_shell ]]{% endraw %}"
```

When you run `sam alias promote`, SAM walks the full dependency graph across both namespaces — image tag, registry, namespace, deployment name — and prompts for each value exactly once, even if multiple aliases in the chain share the same variable.

---

## Session defaults

When you're focused on a specific environment or cluster for a work session, pin values so SAM skips those prompts automatically:

```bash
# Pin for the current terminal session
sam session-set namespace=production
sam session-set docker::tag=v2.4.1

# See what's pinned
sam session-list

# Clear everything at the end of the session
sam session-clear
```

Session identity is automatically detected from `TERM_SESSION_ID`, `TMUX_PANE`, `SSH_CLIENT`, or `PPID` — so different terminal tabs or tmux panes maintain independent session state.

---

## Caching

Every `from_command` variable result is cached by the verbatim command string. The TTL is set globally in `~/.sam_rc.toml`.

```bash
# Clear all cached outputs (useful after infra changes)
sam cache-clear

# Inspect what's cached
sam cache-keys

# Selectively delete cache entries
sam cache-keys-delete

# Skip reading or writing the cache for one invocation
sam alias kubernetes::tail_logs --no-cache
```

---

## MCP server

`sam-mcp` is a Model Context Protocol server that exposes your SAM aliases to AI assistants like Claude. It provides two tools:

- **`list_aliases`** — returns all aliases, optionally filtered by namespace or keyword
- **`resolve_alias`** — stateless resolution loop: each call returns either the next variable needing a value (with its choices), or the final resolved command

This lets an AI assistant discover your aliases, guide you through variable selection conversationally, and produce runnable commands — without needing to know the underlying shell mechanics.

**Setup with Claude Desktop** — add to `claude_desktop_config.json`:

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

---

## CLI reference

| Command | Description |
|---|---|
| `sam run` | Select and run an alias interactively |
| `sam alias <name>` | Run a specific alias by name (e.g. `docker::tail_logs`) |
| `sam history` | Browse and re-run previous commands |
| `sam run-last` / `sam %` | Re-run the last command |
| `sam show-last` / `sam s` | Print the last command without running it |
| `sam check-config` | Validate your configuration and alias files |
| `sam cache-clear` | Clear all cached `from_command` outputs |
| `sam cache-keys` | List all cache keys |
| `sam cache-keys-delete` | Remove specific cache entries interactively |
| `sam session-set <var=value>` | Pin a variable value for the current session |
| `sam session-list` | Show active session defaults |
| `sam session-clear` | Clear all session defaults |

### Flags

| Flag | Description |
|---|---|
| `--dry` / `-d` | Print the resolved command without executing it |
| `-c <ns::var=value>` | Pre-supply a variable value, skipping its prompt (repeatable) |
| `--silent` / `-s` | Run without writing `from_command` outputs to cache |
| `--no-cache` / `-n` | Disable cache reads and writes for this invocation |

`-c` can be specified multiple times:
```bash
sam alias kubernetes::tail_logs \
  -c kubernetes::namespace=production \
  -c kubernetes::since=1h
```

### Keybindings (TUI)

| Key | Action |
|---|---|
| `↑` / `↓` | Move selection |
| `Enter` | Confirm selection |
| `Ctrl-s` | Toggle multi-select on the current item |
| `Ctrl-a` | Select all items |

---

## A real-world configuration

For a full example of a production SAM configuration with Docker, Kubernetes, and general-purpose aliases, see [r-zenine/oneliners](https://github.com/r-zenine/oneliners).

---

## Source

SAM is open source. [View on GitHub](https://github.com/r-zenine/sam).
