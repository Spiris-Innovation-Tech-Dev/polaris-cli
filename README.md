# Polaris CLI & API Client

Rust workspace providing an API library crate and CLI client for the [BlackDuck Coverity on Polaris](https://your-instance.polaris.blackduck.com) platform. Query projects, browse issues with source code context, manage triage — all from the terminal.

## Install as Claude Code Skill

```bash
npx skills add Spiris-Innovation-Tech-Dev/polaris-cli -g -a claude-code -y
```

## Quick Start

```bash
# Store your API token securely in OS keychain (macOS Keychain / Linux Secret Service / Windows Credential Manager)
polaris auth login --token "your-token-here"

# Or set via environment variable
export POLARIS_API_TOKEN="your-token-here"

# Build
cargo build --release

# List all projects
polaris projects

# List issues on the main branch (auto-resolved)
polaris issues --project-id <PROJECT_ID>

# Show full issue detail with source code events
polaris issue --issue-id <ISSUE_ID> --project-id <PROJECT_ID>
```

## Installation

```bash
git clone <repo-url>
cd orangeduck
cargo install --path polaris-cli
```

Or build from source:

```bash
cargo build --release
# Binary at target/release/polaris
```

## Configuration

| Environment Variable | CLI Flag | Default | Description |
|---|---|---|---|
| `POLARIS_API_TOKEN` | `--api-token` | — | API token for authentication |
| `POLARIS_BASE_URL` | `--base-url` | `https://your-instance.polaris.blackduck.com` | Polaris instance URL |

### Token Resolution Order

The CLI resolves the API token in this order:

1. `--api-token` flag
2. `POLARIS_API_TOKEN` environment variable
3. OS keychain (stored via `polaris auth login`)

Generate an API token from the Polaris web UI under your user settings. Tokens authenticate via `POST /api/auth/v2/authenticate` and produce a JWT valid for 12 hours; the client handles this transparently.

## Output Formats

All commands support three output formats via a global flag:

| Flag | Description |
|---|---|
| `--format pretty` | Human-readable terminal output (default) |
| `--format json` or `--json` | Pretty-printed JSON |
| `--format toon` or `--toon` | [TOON](https://toonformat.dev) — token-efficient format for LLM prompts |

```bash
polaris projects --json
polaris issue --issue-id <ID> --project-id <PID> --toon
```

## Commands

### `polaris auth`

Manage authentication and API token storage.

```bash
# Store token in OS keychain (verifies token first)
polaris auth login --token "your-token"

# Interactive prompt (omit --token)
polaris auth login

# Show where the active token comes from
polaris auth status
# Token source:  OS keychain
#   --api-token: not set
#   env var:     not set
#   keychain:    stored

# Remove token from keychain
polaris auth logout

# Print raw JWT (for debugging)
polaris auth jwt
# eyJhbGciOiJSUzI1NiIs...
```

The keychain backend is cross-platform via the [`keyring`](https://crates.io/crates/keyring) crate: macOS Keychain, Linux Secret Service (GNOME Keyring / KDE Wallet), Windows Credential Manager.

### `polaris projects`

List all projects. Automatically paginates through all results.

```bash
polaris projects
polaris projects --name "my-project"    # filter by name
```

```
142 projects found.

ID                                       NAME                                     DESCRIPTION
----------------------------------------------------------------------------------------------------
d30a336b-3072-4448-8dbc-8224e3e883d1     my-service                               Backend service
...
```

### `polaris branches`

List all branches for a project.

```bash
polaris branches --project-id <PROJECT_ID>
```

```
5 branches found.

ID                                       NAME                           MAIN
--------------------------------------------------------------------------------
8a6b7de1-afbb-4970-bc71-84b1cfe60541     main                           ✓
a1b2c3d4-...                             feature/auth                   
...
```

### `polaris issues`

List all issues for a project. Auto-paginates and auto-resolves the main branch if `--branch-id` is omitted.

```bash
polaris issues --project-id <PROJECT_ID>
polaris issues --project-id <PROJECT_ID> --branch-id <BRANCH_ID>
```

```
47 issues found.

ID (short)   ISSUE-KEY                                                        CHECKER              SEVERITY   TYPE
----------------------------------------------------------------------------------------------------------------------------------
fb98cd8100   fb98cd8100586e224488...                                          STRING_OVERFLOW      Low        Copy into fixed size buffer
1033e65c05   1033e65c05a39065bf8c...                                          NULL_RETURNS         High       Dereference null return value
...
```

### `polaris issue`

Show full details for a single issue, including resolved severity, type, tool, file path, a direct web URL, and source code events from the Coverity analysis.

```bash
polaris issue --issue-id <ISSUE_ID> --project-id <PROJECT_ID>
polaris issue --issue-id <ISSUE_ID> --project-id <PROJECT_ID> --branch-id <BRANCH_ID>
```

```
Issue:          fb98cd8100586e2244881262f4069dff6a9b9f98952c661f1de95f099c0f53fa
ID:             fb98cd8100586e2244881262f4069dff6a9b9f98952c661f1de95f099c0f53fa
Severity:       Low
Type:           Copy into fixed size buffer
Checker:        STRING_OVERFLOW
Tool:           code-analysis
Path:           Source/LIB/Saf/SafCalc.cpp
Finding key:    57182ebfa19fc0e722553d0eb615123f
First detected: -
URL:            https://your-instance.polaris.blackduck.com/projects/.../issues/...?pagingOffset=0&path=...

── Event Summary ──
Main event:     Source/LIB/Saf/SafCalc.cpp:130 (cpp)
    Source/LIB/Saf/SafCalc.cpp:130: You might overrun the 12-character fixed-size string
                                     "this->str" by copying "str" without checking the length.
        125 │      OpDefItem(const TCHAR* str, int8_t op, int8_t level)
        126 │          : op(op), level(level)
        127 │      {
        128 │          len = static_cast<int16_t>(_tcslen(str));
        129 │          ASSERT(len < 12);
    Source/LIB/Saf/SafCalc.cpp:130: Note: This defect has an elevated risk because the source
                                     argument is a parameter of the current function.
        130 │          _tcscpy(this->str, str);
        131 │      }
```

The URL links directly to the issue in the Polaris web UI with the correct project, branch, revision, and file path.

When `--branch-id` is omitted, the main branch is auto-resolved.

### `polaris events`

Show the full Coverity event tree with source code snippets for a finding. The finding key and run ID come from the issue detail (visible via `polaris issue --json`).

```bash
polaris events --finding-key <FINDING_KEY> --run-id <RUN_ID>
polaris events --finding-key <KEY> --run-id <RID> --max-depth 3
polaris events --finding-key <KEY> --run-id <RID> --occurrence 2
```

Event types are indicated by markers:
- `►` — main event (the defect location)
- `→` — path event (control/data flow leading to the defect)
- `╴` — evidence event (supporting detail)
- `◆` — example event

Each event shows the file, line number, description, and surrounding source code.

### `polaris triage get`

Get the current triage status for an issue.

```bash
polaris triage get --project-id <PROJECT_ID> --issue-key <ISSUE_KEY>
```

```
Issue key:        fb98cd8100586e2244881262f4069dff6a9b9f98952c661f1de95f099c0f53fa
Project ID:       d30a336b-3072-4448-8dbc-8224e3e883d1
Dismissal status: N/A
Triage values:
  { "attribute_name": "DISMISS", "attribute_value": "NOT_DISMISSED" }
```

### `polaris triage update`

Update triage for one or more issues. At least one of `--dismiss`, `--owner`, or `--comment` is required.

```bash
# Dismiss as false positive
polaris triage update --project-id <PID> --issue-keys <KEY> --dismiss DISMISSED_AS_FP

# Dismiss with comment
polaris triage update --project-id <PID> --issue-keys <KEY> \
  --dismiss DISMISSED_BY_DESIGN --comment "Intentional behavior per spec"

# Assign owner
polaris triage update --project-id <PID> --issue-keys <KEY> --owner user@example.com

# Batch update multiple issues
polaris triage update --project-id <PID> --issue-keys <KEY1>,<KEY2>,<KEY3> \
  --dismiss DISMISSED_AS_FP --comment "All confirmed FP"
```

**Dismiss values:**

| Value | Description |
|---|---|
| `NOT_DISMISSED` | Reset to not dismissed |
| `DISMISSED_BY_DESIGN` | Intentional / by design |
| `DISMISSED_AS_FP` | False positive |

**Dismissal review statuses** (set by the system or reviewers):

| Status | Description |
|---|---|
| `REQUESTED` | Dismissal requested, pending review |
| `SYSTEM_APPROVED` | Auto-approved by system policy |
| `USER_APPROVED` | Approved by a reviewer |
| `USER_REJECTED` | Rejected by a reviewer |

### `polaris triage history`

View the triage change history for an issue.

```bash
polaris triage history --project-id <PROJECT_ID> --issue-key <ISSUE_KEY>
polaris triage history --project-id <PID> --issue-key <KEY> --limit 50
```

### `polaris counts`

Get roll-up counts of issues, optionally grouped by a field (severity, issue type, tool, etc.). Auto-resolves the main branch when `--branch-id` is omitted.

```bash
polaris counts --project-id <PROJECT_ID>
polaris counts --project-id <PID> --branch-id <BID>
polaris counts --project-id <PID> --group-by '[issue][taxonomy][id][011dfe05-00e5-4d8c-8746-a81fe44a120b]'
```

```
GROUP                                    COUNT
--------------------------------------------------
Audit                                    0
High                                     4
Low                                      12
Medium                                   3
Not Specified                            0
```

The `--group-by` value must be a discovery value — use `polaris discovery --type group-bys` to list valid options.

### `polaris trends`

Get issue counts over time, grouped by status or severity. Auto-resolves the main branch when `--branch-id` is omitted.

```bash
polaris trends --project-id <PROJECT_ID>
polaris trends --project-id <PID> --granularity month
polaris trends --project-id <PID> --start-date 2025-01-01 --end-date 2025-12-31
polaris trends --project-id <PID> --group-by '[issue][status]'
```

```
Series: Open
  2023-02-23T15:20:15.000Z: 0
  2023-02-24T15:20:15.000Z: 6
  ...

Series: Closed
  2023-02-23T15:20:15.000Z: 0
  ...
```

### `polaris age`

Get issue age metrics (average age for outstanding or resolved issues). Auto-resolves the main branch when `--branch-id` is omitted.

```bash
polaris age --project-id <PROJECT_ID>
polaris age --project-id <PID> --branch-id <BID>
polaris age --project-id <PID> --metric resolved
```

```
Average age (Low): 814.0 days
```

Metric options: `outstanding` (default), `resolved`.

### `polaris discovery`

Query available group-by values and filter keys. Useful for finding valid `--group-by` arguments for `counts` and `trends`.

```bash
polaris discovery --type group-bys
polaris discovery --type filter-keys
```

## Library Usage (`polaris-api`)

The `polaris-api` crate can be used independently in Rust projects:

```rust
use polaris_api::client::{PolarisClient, PolarisConfig, TriageValues};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = PolarisConfig {
        base_url: "https://your-instance.polaris.blackduck.com".into(),
        api_token: std::env::var("POLARIS_API_TOKEN")?,
    };
    let client = PolarisClient::new(config);

    // List all projects (auto-paginates)
    let projects = client.list_all_projects(None, 25).await?;
    for p in &projects.data {
        println!("{}: {}", p.id, p.attributes.name);
    }

    // List all issues on main branch (auto-paginates)
    let issues = client
        .list_all_issues(&project_id, Some(&branch_id), None, 25)
        .await?;
    for issue in &issues.data {
        println!("{}: {}", issue.id, issue.attributes.issue_key);
    }

    // Get single issue with full detail
    let detail = client.get_issue(&issue_id, &project_id, &branch_id).await?;

    // Get event tree with source code
    let events = client
        .get_events_with_source(&finding_key, &run_id, None, None)
        .await?;

    // Get full source file
    let source = client.get_source_code(&run_id, "Source/LIB/Saf/SafCalc.cpp").await?;

    // Update triage
    let values = TriageValues {
        dismiss: Some("DISMISSED_AS_FP".into()),
        commentary: Some("Confirmed false positive".into()),
        ..Default::default()
    };
    client.update_triage(&project_id, &["issue-key-here"], &values).await?;

    Ok(())
}
```

### Key Types

| Type | Description |
|---|---|
| `PolarisClient` | High-level client with JWT caching and all API methods |
| `PolarisConfig` | Configuration (base URL + API token) |
| `JsonApiResponse<T>` | Generic JSON:API list response with pagination metadata |
| `PaginationMeta` | Pagination info: `offset`, `limit`, `total` |
| `Project` / `Branch` | Common object types |
| `IssuesResponse` / `Issue` | Issue query types |
| `TriageCurrentResponse` | Current triage state |
| `TriageValues` | Triage update payload (dismiss, owner, commentary) |
| `PolarisError` | Error enum (auth, API, network, deserialization) |

### Pagination

List methods come in two flavors:
- `list_projects(filter, limit, offset)` — single page
- `list_all_projects(filter, page_size)` — auto-paginates, collects all results

The CLI always uses `list_all_*` methods. The library exposes both for flexibility.

## API Coverage

| Service | Base Path | Methods | Implementation |
|---|---|---|---|
| Auth | `/api/auth/v2` | `POST /authenticate` | Hand-crafted |
| Common Objects | `/api/common/v0` | Projects, branches, runs | Hand-crafted |
| Issue Query | `/api/query/v1` | List issues, get issue, roll-up counts, issues over time, issue age | Progenitor + hand-crafted |
| Issue Discovery | `/api/query/v1/discovery` | Filter keys, group-bys | Hand-crafted |
| Code Analysis | `/api/code-analysis/v0` | Events with source, source code | Hand-crafted |
| Triage Command | `/api/triage-command/v1` | Update triage | Progenitor |
| Triage Query | `/api/triage-query/v1` | Current triage, history | Progenitor |

## Architecture

```
orangeduck/
├── polaris-api/           # Library crate
│   ├── build.rs           # Progenitor code generation from OpenAPI specs
│   ├── specs/             # OpenAPI 3.0 YAML specs (converted from Swagger 2.0)
│   └── src/
│       ├── lib.rs         # Module exports (generated + hand-crafted)
│       ├── client.rs      # High-level PolarisClient with JWT caching
│       ├── auth.rs        # Auth client (POST /authenticate)
│       ├── common.rs      # Common objects (projects, branches) + JSON:API types
│       └── error.rs       # Error types
├── polaris-cli/           # CLI binary crate
│   └── src/
│       └── main.rs        # Clap-based CLI with all commands
├── Cargo.toml             # Workspace root
└── README.md
```

The workspace uses [progenitor](https://crates.io/crates/progenitor) to generate typed API clients from OpenAPI 3.0 specs (converted from the Swagger 2.0 specs served by the Polaris instance). Several services (Auth, Common Objects, Code Analysis) are hand-crafted because their specs cause progenitor to panic on complex response types.

The high-level `PolarisClient` wraps both generated and hand-crafted clients, handling JWT authentication and caching transparently. All list endpoints auto-paginate by following the `offset`/`limit`/`total` metadata in JSON:API responses.
