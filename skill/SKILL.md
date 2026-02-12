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

Before any command will work, an API token must be available. Resolution order:
1. `--api-token` flag
2. `POLARIS_API_TOKEN` environment variable
3. OS keychain (macOS Keychain, Linux Secret Service, Windows Credential Manager)

**First-time setup:** Get an API token from the Polaris web UI (user settings > API tokens),
then store it in the OS keychain so it persists across sessions:
```bash
scripts/polaris auth login --token <TOKEN>
```
The token is verified before being stored. If login fails, the token is invalid.

**If auth errors occur**, check the current state:
```bash
scripts/polaris auth status --toon
```
This shows which sources have a token and which one is active.

**Remove stored token:**
```bash
scripts/polaris auth logout
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
