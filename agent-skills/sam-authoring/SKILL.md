---
name: sam-authoring
description: Guide for authoring SAM aliases and vars files. Use when a user wants to create, edit, or review aliases.yaml or vars.yaml files for SAM, add new aliases to a .sam folder, or structure a new SAM namespace.
---

# SAM Alias Authoring

## File placement and namespaces

Place files under any directory declared as `root_dir` in `~/.sam_rc.toml`.
The parent directory name becomes the namespace automatically:

```
~/.sam/
├── aliases.yaml        # no namespace → my_alias
├── vars.yaml
└── docker/
    ├── aliases.yaml    # namespace: docker → docker::my_alias
    └── vars.yaml       # namespace: docker → docker::my_var
```

A local `.sam_rc.toml` in the current directory overrides the global one — useful for project-scoped aliases.

## aliases.yaml schema

```yaml
- name: deploy                          # required, no spaces
  desc: Tag and push image              # required
  alias: docker push {{ image }}:{{ tag }}  # required; use {{ var }} and [[ ns::alias ]]
```

- Reference vars with `{{ var_name }}` or `{{ namespace::var_name }}`
- Embed other aliases with `[[ namespace::alias_name ]]`
- SAM resolves all dependencies before execution

## vars.yaml schema

Three mutually exclusive modes:

```yaml
# Static choices
- name: environment
  desc: target environment
  choices:
    - value: staging
      desc: staging environment
    - value: production

# Dynamic choices — from_command MUST be read-only (no side effects)
- name: container
  desc: a running Docker container
  from_command: 'docker ps --format="{{.ID}}\t{{.Names}}"'

# Templated command — declare dependency explicitly via {{ var }}
- name: container_ip
  desc: IP of the selected container
  from_command: "docker inspect {{ container }} | jq -r '.[0].NetworkSettings.IPAddress'"

# Free-text input
- name: tag
  desc: Docker image tag
  from_input: "enter a tag (e.g. v1.2.3)"
```

## Critical constraint: vars are read-only

`from_command` values **must be pure read operations** — no writes, no mutations, no side effects.
SAM caches command output and may replay or skip execution; a command that modifies state will produce unpredictable results.

Good: `docker ps`, `kubectl get pods`, `git branch`, `gh issue list`
Bad: `docker rm ...`, `kubectl delete ...`, `git commit ...`

## Dependency resolution

SAM detects `{{ var }}` references inside `from_command` and prompts for dependencies first — no explicit ordering needed. Cross-namespace references work everywhere: `{{ kubernetes::registry }}`.
