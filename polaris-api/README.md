# polaris-api

Async Rust API client for the Black Duck Polaris platform.

[![crates.io](https://img.shields.io/crates/v/polaris-api.svg)](https://crates.io/crates/polaris-api)
[![docs.rs](https://docs.rs/polaris-api/badge.svg)](https://docs.rs/polaris-api)
[![license](https://img.shields.io/crates/l/polaris-api.svg)](https://github.com/Spiris-Innovation-Tech-Dev/polaris-cli)

`polaris-api` provides a high-level, async client for common Polaris workflows: authentication, project and branch discovery, issue queries, triage updates, and analysis metrics.

## Features

- **Async-first** client built on `tokio` + `reqwest`
- **API token authentication** with automatic JWT retrieval and caching
- **Pagination helpers** (`list_all_*`) for project/branch/issue traversal
- **Triage workflows** (`get_triage`, `update_triage`, `get_triage_history`)
- **Issue analytics** (roll-up counts, trends over time, issue age, discovery endpoints)
- **Typed models** for common JSON:API entities and response metadata
- **Consistent error handling** via `PolarisError`

## Installation

```bash
cargo add polaris-api
```

## Quick start

```rust
use polaris_api::client::{PolarisClient, PolarisConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = PolarisConfig {
        base_url: std::env::var("POLARIS_BASE_URL")?,
        api_token: std::env::var("POLARIS_API_TOKEN")?,
    };
    let client = PolarisClient::new(config);

    let projects = client.list_all_projects(None, 50).await?;
    for project in projects.data {
        println!("{} ({})", project.attributes.name, project.id);
    }

    Ok(())
}
```

Environment variables:

- `POLARIS_BASE_URL` (example: `https://your-instance.polaris.blackduck.com`)
- `POLARIS_API_TOKEN` (API token from Polaris user settings)

## API surface (high level)

### Authentication

- `authenticate`

### Projects and branches

- `list_projects`, `list_all_projects`
- `list_branches`, `list_all_branches`

### Issues and details

- `list_issues`, `list_all_issues`
- `get_issue`
- `get_events_with_source`
- `get_source_code`

### Triage

- `get_triage`
- `update_triage`
- `get_triage_history`

### Metrics and discovery

- `get_roll_up_counts`
- `get_issues_over_time`
- `get_issue_age`
- `get_filter_keys`
- `get_group_bys`

## Errors

Most operations return `Result<T, PolarisError>`, where `PolarisError` includes:

- HTTP transport errors
- auth failures
- API status/detail errors
- deserialization errors
- typed `NotFound` cases

## Security notes

- The client uses bearer-token auth over HTTPS and sets explicit API headers.
- JWTs are cached in memory and wrapped with `zeroize::Zeroizing`.
- The crate denies unsafe operations in unsafe functions (`#![deny(unsafe_op_in_unsafe_fn)]`).

## Relationship to this repository

This crate lives in the `polaris-cli` workspace and is used by the companion CLI binary crate (`polaris-cli/`) for terminal workflows.

## License

MIT
