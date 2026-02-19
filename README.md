# Polaris CLI + API Client for Black Duck Polaris

Rust workspace for working with the Black Duck Polaris platform from code and the terminal.

[![crate: polaris-api](https://img.shields.io/crates/v/polaris-api.svg)](https://crates.io/crates/polaris-api)
[![crate: polaris-cli](https://img.shields.io/crates/v/polaris-cli.svg)](https://crates.io/crates/polaris-cli)
[![docs.rs](https://docs.rs/polaris-api/badge.svg)](https://docs.rs/polaris-api)
[![license: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/Spiris-Innovation-Tech-Dev/polaris-cli)

## Workspace crates

| Crate | Type | Purpose |
| --- | --- | --- |
| `polaris-api` | Library | Async Rust API client for Black Duck Polaris (auth, issues, triage, metrics). |
| `polaris-cli` | Binary | Command-line workflow for querying Polaris projects/issues and managing triage. |

## Quick start

```bash
# Clone and build
git clone https://github.com/Spiris-Innovation-Tech-Dev/polaris-cli.git
cd polaris-cli
cargo build --release

# Authenticate once (stores token in OS keychain)
./target/release/polaris auth login --token "your-api-token"

# Or use env vars instead of keychain
export POLARIS_BASE_URL="https://your-instance.polaris.blackduck.com"
export POLARIS_API_TOKEN="your-api-token"

# Basic workflows
./target/release/polaris projects
./target/release/polaris issues --project-id <PROJECT_ID>
./target/release/polaris issue --project-id <PROJECT_ID> --issue-id <ISSUE_ID>
```

## Install the CLI

From crates.io:

```bash
cargo install polaris-cli
```

Or from source:

```bash
cargo install --path polaris-cli
```

Then use:

```bash
polaris --help
```

## Authentication and configuration

### API token resolution order

1. `--api-token`
2. `POLARIS_API_TOKEN`
3. OS keychain (set via `polaris auth login`)

### Base URL resolution order

1. `--base-url`
2. `POLARIS_BASE_URL`
3. `~/.config/polaris/config.toml` (`base_url = "..."`)
4. Default placeholder: `https://your-instance.polaris.blackduck.com`

## Output formats

Global output flags are available on all commands:

- `--format pretty` (default)
- `--format json` or `--json`
- `--format toon` or `--toon`

## Command overview

| Command | Description |
| --- | --- |
| `polaris auth login` | Verify and store API token in OS keychain |
| `polaris auth status` | Show where token is sourced from |
| `polaris auth jwt` | Print the current JWT (debugging) |
| `polaris projects [--name ...]` | List projects |
| `polaris branches --project-id ...` | List branches for a project |
| `polaris issues --project-id ... [--branch-id ...]` | List issues |
| `polaris issue --project-id ... --issue-id ...` | Show full issue detail |
| `polaris events --finding-key ... --run-id ...` | Show Coverity event tree with source |
| `polaris triage get/update/history ...` | Query or update triage |
| `polaris counts/trends/age ...` | Issue metrics and trend endpoints |
| `polaris discovery --type filter-keys|group-bys` | Query supported filter/group fields |

## Using the Rust library (`polaris-api`)

`polaris-api` is published on crates.io:

- Crate: https://crates.io/crates/polaris-api
- API docs: https://docs.rs/polaris-api
- Crate README: [`polaris-api/README.md`](./polaris-api/README.md)

Install:

```bash
cargo add polaris-api
```

Minimal example:

```rust
use polaris_api::client::{PolarisClient, PolarisConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = PolarisClient::new(PolarisConfig {
        base_url: std::env::var("POLARIS_BASE_URL")?,
        api_token: std::env::var("POLARIS_API_TOKEN")?,
    });

    let projects = client.list_all_projects(None, 50).await?;
    println!("projects: {}", projects.data.len());
    Ok(())
}
```

## Install as Claude Code skill

```bash
npx skills add Spiris-Innovation-Tech-Dev/polaris-cli -g -a claude-code -y
```

## Development

```bash
# Build everything
cargo build --workspace

# Run tests
cargo test --workspace

# Run CLI locally
cargo run -p polaris-cli -- projects
```

## Security notes

- API tokens are never written to plaintext config by default; use OS keychain storage via `polaris auth login`.
- `polaris-api` caches JWT values in memory using `zeroize::Zeroizing`.
- Keep your Polaris token scoped and rotated according to organizational policy.

## License

MIT
