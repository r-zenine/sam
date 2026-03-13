# Changelog — Polish + Release

## Delivered

- **`--config <path>` CLI flag**: `main.rs` parses `--config` from args; `loader::load_from(path)` delegates to new `AppSettings::load_from(path)` in `sam-config`. Existing `load()` path unchanged.
- **Improved tool descriptions**: `list_aliases` now includes usage examples in its description; `resolve_alias` description was already sufficient.
- **Better alias-not-found errors**: When an alias is not found, the server now suggests up to 5 similar aliases (substring match on full name) or directs the user to `list_aliases()`.
- **`sam-mcp/README.md`**: Documents both tools with examples, Claude Desktop `claude_desktop_config.json` snippet (default and `--config` variants), and build instructions.
- **`build-mcp` Makefile target**: `make build-mcp` runs `cargo build --release -p sam-mcp` for standalone MCP binary builds.

## Enables

The `sam-mcp` binary is now ready for distribution. Users can build with `make build-mcp`, drop the binary into PATH, and add the Claude Desktop config snippet to start using SAM aliases from any AI assistant.
