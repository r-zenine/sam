"""Tests for block-sam-direct-access.py hook."""
import json
import os
import subprocess
import sys
from pathlib import Path

HOOK = Path(__file__).parent / "block-sam-direct-access.py"
HOME = os.path.expanduser("~")


def run_hook(tool_name: str, tool_input: dict, pwd: str | None = None) -> subprocess.CompletedProcess:
    payload = json.dumps({"tool_name": tool_name, "tool_input": tool_input})
    env = {**os.environ, "PWD": pwd} if pwd else os.environ
    return subprocess.run(
        [sys.executable, str(HOOK)],
        input=payload,
        text=True,
        capture_output=True,
        env=env,
    )


# --- blocking cases ---

def test_blocks_read_on_sam_dir():
    result = run_hook("Read", {"file_path": f"{HOME}/.sam/default/aliases.yaml"})
    assert result.returncode == 2
    assert "mcp__sam-mcp__list_aliases" in result.stderr


def test_blocks_bash_touching_sam_dir():
    result = run_hook("Bash", {"command": "cat ~/.sam/default/aliases.yaml"})
    assert result.returncode == 2
    assert "mcp__sam-mcp__list_aliases" in result.stderr


def test_blocks_glob_on_sam_dir():
    result = run_hook("Glob", {"pattern": f"{HOME}/.sam/**/*.yaml"})
    assert result.returncode == 2
    assert "mcp__sam-mcp__list_aliases" in result.stderr


# --- allow cases ---

def test_allows_unrelated_path():
    result = run_hook("Read", {"file_path": f"{HOME}/workspace/project/foo.yaml"})
    assert result.returncode == 0


def test_allows_when_pwd_is_sam_dir():
    result = run_hook(
        "Read",
        {"file_path": f"{HOME}/.sam/default/aliases.yaml"},
        pwd=f"{HOME}/.sam",
    )
    assert result.returncode == 0


if __name__ == "__main__":
    tests = [v for k, v in list(globals().items()) if k.startswith("test_")]
    failed = 0
    for t in tests:
        try:
            t()
            print(f"  PASS  {t.__name__}")
        except AssertionError as e:
            print(f"  FAIL  {t.__name__}: {e}")
            failed += 1
    print(f"\n{len(tests) - failed}/{len(tests)} passed")
    sys.exit(failed)
