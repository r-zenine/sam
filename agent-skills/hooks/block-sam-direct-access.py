#!/usr/bin/env python3
"""
PreToolUse hook: block direct ~/.sam filesystem access and nudge toward the SAM MCP server.
Applies to Read, Glob, and Bash tools.

Exception: access is allowed when Claude's working directory is inside ~/.sam
(e.g. the user is explicitly authoring SAM files).
"""
import json
import os
import re
import sys

home = os.path.expanduser("~")
sam_dir = os.path.join(home, ".sam")

# Allow if Claude is working inside ~/.sam (e.g. authoring aliases)
cwd = os.environ.get("PWD", os.getcwd())
if os.path.abspath(cwd).startswith(os.path.abspath(sam_dir)):
    sys.exit(0)

data = json.load(sys.stdin)
tool_name = data.get("tool_name", "")
tool_input = data.get("tool_input", {})

sam_pattern = re.compile(r"(~/\.sam|" + re.escape(sam_dir) + r")")


def touches_sam(text: str) -> bool:
    return bool(text and sam_pattern.search(text))


blocked = False
if tool_name == "Read":
    blocked = touches_sam(tool_input.get("file_path", ""))
elif tool_name == "Glob":
    blocked = touches_sam(tool_input.get("pattern", "")) or touches_sam(
        tool_input.get("path", "")
    )
elif tool_name == "Bash":
    blocked = touches_sam(tool_input.get("command", ""))

if blocked:
    print(
        "Blocked: direct access to ~/.sam is not allowed.\n"
        "Use the SAM MCP tools instead:\n"
        "  • mcp__sam-mcp__list_aliases   — list all available aliases\n"
        "  • mcp__sam-mcp__resolve_alias  — get the full definition of an alias",
        file=sys.stderr,
    )
    sys.exit(2)

sys.exit(0)
