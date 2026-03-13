# sam-mcp

MCP server for [SAM](https://github.com/ajmwagar/sam) — exposes SAM aliases as tools so AI assistants (Claude, etc.) can discover and resolve them interactively.

## Tools

### `list_aliases`

Returns all SAM aliases, optionally filtered by namespace or keyword.

```
list_aliases()                          → all aliases
list_aliases(namespace="docker")        → docker namespace only
list_aliases(keyword="run")             → aliases whose name or description contains "run"
list_aliases(namespace="docker", keyword="run")  → intersection
```

Response: `[{ name, namespace, desc, template }]`

### `resolve_alias`

Resolves a SAM alias to a runnable shell command via a stateless state machine. Call in a loop:
each call returns either the next variable that needs a value, or the final resolved command.

```
resolve_alias(alias="docker::run", vars={})
→ { status: "needs_var", var: { name: "image", kind: "choices", choices: [{ value: "nginx" }] } }

resolve_alias(alias="docker::run", vars={ "docker::image": "nginx" })
→ { status: "resolved", commands: ["docker run nginx"] }
```

## Usage with Claude Desktop

Add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "sam": {
      "command": "/path/to/sam-mcp"
    }
  }
}
```

With a custom config file:

```json
{
  "mcpServers": {
    "sam": {
      "command": "/path/to/sam-mcp",
      "args": ["--config", "/path/to/.sam_rc.toml"]
    }
  }
}
```

## Build

```bash
cargo build --release -p sam-mcp
# binary: target/release/sam-mcp
```

## Configuration

By default `sam-mcp` reads `~/.sam_rc.toml` (or `.sam_rc.toml` in the current directory).
Use `--config <path>` to override:

```bash
sam-mcp --config /path/to/.sam_rc.toml
```
