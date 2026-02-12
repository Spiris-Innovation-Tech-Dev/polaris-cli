---
name: polaris-cli
description: >
  Query and manage BlackDuck Coverity static analysis issues via the Polaris CLI.
  Use when the user asks about Coverity issues, code analysis findings, static analysis
  results, Polaris projects/branches, triage status, dismissing findings, or any
  BlackDuck/Polaris-related task. Trigger phrases include "check coverity", "show issues",
  "list projects", "triage issue", "dismiss finding", "polaris", "static analysis",
  "code analysis findings", "coverity issues", "security findings".
---

# Polaris CLI

CLI for querying BlackDuck Coverity on Polaris.

## Setup

If `bin/polaris` is missing, run the install script first:
```bash
scripts/install.sh
```
This downloads the correct platform binary from GitHub Releases (requires `gh` CLI).

The wrapper at `scripts/polaris` calls `bin/polaris`. All commands below use this wrapper.

## Output Format

**Always use `--toon` flag** on every command. TOON is a token-efficient format
optimized for LLM context windows. Never use `--format pretty` or omit the flag.

```bash
scripts/polaris --toon <command> [options]
```

## Authentication

Token resolution order: `--api-token` flag > `POLARIS_API_TOKEN` env var > OS keychain.

Store token in keychain:
```bash
scripts/polaris auth login --token <TOKEN>
```

Check auth status:
```bash
scripts/polaris auth status --toon
```

## Commands

### List projects

```bash
scripts/polaris projects --toon
scripts/polaris projects --toon --name "exact-project-name"
```

### List branches

```bash
scripts/polaris branches --toon --project-id <PROJECT_UUID>
```

### List issues

```bash
# Uses main branch automatically when --branch-id omitted
scripts/polaris issues --toon --project-id <PROJECT_UUID>
scripts/polaris issues --toon --project-id <PROJECT_UUID> --branch-id <BRANCH_UUID>
```

### Show issue detail

```bash
scripts/polaris issue --toon --issue-id <ISSUE_UUID> --project-id <PROJECT_UUID>
```

Returns full detail including severity, checker, file path, event summary, and web URL.

### Show event tree

```bash
scripts/polaris events --toon --finding-key <FINDING_KEY> --run-id <RUN_ID>
scripts/polaris events --toon --finding-key <KEY> --run-id <ID> --max-depth 3
```

Get `finding-key` and `run-id` from issue detail output. Shows full Coverity event tree
with source code context.

### Triage

Get current triage status:
```bash
scripts/polaris triage get --toon --project-id <PROJECT_UUID> --issue-key <ISSUE_KEY>
```

Update triage (at least one of `--dismiss`, `--owner`, `--comment` required):
```bash
scripts/polaris triage update --toon --project-id <PID> --issue-keys <KEY1>,<KEY2> \
  --dismiss DISMISSED_AS_FP --comment "False positive: checked manually"
```

Dismiss values: `NOT_DISMISSED`, `DISMISSED_BY_DESIGN`, `DISMISSED_AS_FP`.

View triage history:
```bash
scripts/polaris triage history --toon --project-id <PROJECT_UUID> --issue-key <ISSUE_KEY> --limit 20
```

## Typical Workflow

1. Find the project: `scripts/polaris projects --toon --name "my-project"`
2. List issues on main branch: `scripts/polaris issues --toon --project-id <PID>`
3. Inspect a specific issue: `scripts/polaris issue --toon --issue-id <IID> --project-id <PID>`
4. View full event tree if needed: `scripts/polaris events --toon --finding-key <FK> --run-id <RID>`
5. Triage: `scripts/polaris triage update --toon --project-id <PID> --issue-keys <IK> --dismiss DISMISSED_AS_FP`

## Global Options

| Flag | Env Var | Default |
|---|---|---|
| `--base-url` | `POLARIS_BASE_URL` | `https://visma.cop.blackduck.com` |
| `--api-token` | `POLARIS_API_TOKEN` | (keychain) |
| `--toon` | - | Use this always |
