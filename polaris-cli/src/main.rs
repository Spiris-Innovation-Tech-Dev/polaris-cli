#![warn(clippy::unwrap_used, clippy::expect_used)]

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use polaris_api::client::{PolarisClient, PolarisConfig, TriageValues};

const KEYRING_SERVICE: &str = "polaris-cli";
const KEYRING_USER: &str = "api-token";

#[derive(Debug, Clone, ValueEnum)]
enum OutputFormat {
    /// Pretty terminal output (default)
    Pretty,
    /// JSON output
    Json,
    /// TOON format (token-efficient)
    Toon,
}

#[derive(Parser)]
#[command(name = "polaris", about = "BlackDuck Polaris CLI client")]
struct Cli {
    /// Base URL for the Polaris instance
    #[arg(long, env = "POLARIS_BASE_URL", default_value = "https://visma.cop.blackduck.com")]
    base_url: String,

    /// API token for authentication
    #[arg(long, env = "POLARIS_API_TOKEN")]
    api_token: Option<String>,

    /// Output format
    #[arg(long, value_enum, default_value = "pretty", global = true)]
    format: OutputFormat,

    /// Shorthand for --format json
    #[arg(long, global = true)]
    json: bool,

    /// Shorthand for --format toon
    #[arg(long, global = true)]
    toon: bool,

    #[command(subcommand)]
    command: Commands,
}

impl Cli {
    fn output_format(&self) -> OutputFormat {
        if self.json {
            OutputFormat::Json
        } else if self.toon {
            OutputFormat::Toon
        } else {
            self.format.clone()
        }
    }
}

/// Emit a serde_json::Value in the requested format.
fn emit(val: &serde_json::Value, fmt: &OutputFormat) -> Result<()> {
    match fmt {
        OutputFormat::Pretty => {
            println!("{}", serde_json::to_string_pretty(val)?);
        }
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(val)?);
        }
        OutputFormat::Toon => {
            let toon = toon_rs::encode_to_string(val, &toon_rs::Options::default())
                .map_err(|e| anyhow::anyhow!("TOON encode error: {e}"))?;
            println!("{toon}");
        }
    }
    Ok(())
}

#[derive(Subcommand)]
enum Commands {
    /// Manage authentication
    Auth {
        #[command(subcommand)]
        action: AuthCommands,
    },

    /// List projects
    Projects {
        /// Filter by project name
        #[arg(long)]
        name: Option<String>,
    },

    /// List branches for a project
    Branches {
        /// Project ID
        #[arg(long)]
        project_id: String,
    },

    /// List issues for a project
    Issues {
        /// Project ID
        #[arg(long)]
        project_id: String,

        /// Branch ID
        #[arg(long)]
        branch_id: Option<String>,
    },

    /// Show full details for a single issue
    #[command(name = "issue")]
    IssueShow {
        /// Issue ID
        #[arg(long)]
        issue_id: String,

        /// Project ID (needed to resolve main branch)
        #[arg(long)]
        project_id: String,

        /// Branch ID (auto-resolves main branch if omitted)
        #[arg(long)]
        branch_id: Option<String>,
    },

    /// Show event tree with source code for a finding
    Events {
        /// Finding key (from issue attributes)
        #[arg(long)]
        finding_key: String,

        /// Run ID (from issue latest-observed-on-run relationship)
        #[arg(long)]
        run_id: String,

        /// Occurrence number (default 1)
        #[arg(long)]
        occurrence: Option<u32>,

        /// Max depth of nested events
        #[arg(long)]
        max_depth: Option<u32>,
    },

    /// Triage operations
    Triage {
        #[command(subcommand)]
        action: TriageAction,
    },
}

#[derive(Subcommand)]
enum TriageAction {
    /// Get current triage status for an issue
    Get {
        /// Project ID
        #[arg(long)]
        project_id: String,

        /// Issue key
        #[arg(long)]
        issue_key: String,
    },

    /// Update triage for one or more issues
    Update {
        /// Project ID
        #[arg(long)]
        project_id: String,

        /// Issue key(s), comma-separated
        #[arg(long, value_delimiter = ',')]
        issue_keys: Vec<String>,

        /// Dismiss value (e.g. NOT_DISMISSED, DISMISSED_BY_DESIGN, DISMISSED_AS_FP)
        #[arg(long)]
        dismiss: Option<String>,

        /// Owner email
        #[arg(long)]
        owner: Option<String>,

        /// Comment text
        #[arg(long)]
        comment: Option<String>,
    },

    /// Get triage history for an issue
    History {
        /// Project ID
        #[arg(long)]
        project_id: String,

        /// Issue key
        #[arg(long)]
        issue_key: String,

        /// Maximum results
        #[arg(long, default_value = "10")]
        limit: u32,
    },
}

#[derive(Subcommand)]
enum AuthCommands {
    /// Store API token in OS keychain
    Login {
        /// API token (will prompt if not provided)
        #[arg(long)]
        token: Option<String>,
    },
    /// Remove API token from OS keychain
    Logout,
    /// Show authentication status
    Status,
    /// Authenticate and display JWT (for debugging)
    Jwt,
}

fn keyring_entry() -> Result<keyring::Entry, keyring::Error> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
}

fn token_from_keyring() -> Option<String> {
    keyring_entry().ok().and_then(|e| e.get_password().ok())
}

fn resolve_token(cli: &Cli) -> Result<String> {
    cli.api_token
        .clone()
        .or_else(|| std::env::var("POLARIS_API_TOKEN").ok())
        .or_else(token_from_keyring)
        .context("API token required: use `polaris auth login`, set POLARIS_API_TOKEN, or pass --api-token")
}

fn make_client(cli: &Cli) -> Result<PolarisClient> {
    let api_token = resolve_token(cli)?;
    let config = PolarisConfig {
        base_url: cli.base_url.clone(),
        api_token,
    };
    Ok(PolarisClient::new(config))
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let fmt = cli.output_format();

    // Auth subcommands that don't need a client
    if let Commands::Auth { ref action } = cli.command {
        match action {
            AuthCommands::Login { token } => {
                let token = match token {
                    Some(t) => t.clone(),
                    None => {
                        eprint!("Enter API token: ");
                        let mut buf = String::new();
                        std::io::stdin().read_line(&mut buf)?;
                        buf.trim().to_string()
                    }
                };
                if token.is_empty() {
                    anyhow::bail!("Token cannot be empty");
                }
                // Verify the token works before storing
                let config = PolarisConfig {
                    base_url: cli.base_url.clone(),
                    api_token: token.clone(),
                };
                let test_client = PolarisClient::new(config);
                test_client.authenticate().await.context("Token verification failed — not stored")?;

                let entry = keyring_entry().context("Failed to access OS keychain")?;
                entry.set_password(&token).context("Failed to store token in keychain")?;
                eprintln!("✓ Token verified and stored in OS keychain");
                return Ok(());
            }
            AuthCommands::Logout => {
                match keyring_entry() {
                    Ok(entry) => match entry.delete_credential() {
                        Ok(()) => eprintln!("✓ Token removed from OS keychain"),
                        Err(keyring::Error::NoEntry) => eprintln!("No token stored in keychain"),
                        Err(e) => anyhow::bail!("Failed to remove token: {e}"),
                    },
                    Err(e) => anyhow::bail!("Failed to access OS keychain: {e}"),
                }
                return Ok(());
            }
            AuthCommands::Status => {
                let has_arg = cli.api_token.is_some();
                let has_env = std::env::var("POLARIS_API_TOKEN").is_ok();
                let has_keychain = token_from_keyring().is_some();
                let source = if has_arg {
                    "--api-token flag"
                } else if has_env {
                    "POLARIS_API_TOKEN env var"
                } else if has_keychain {
                    "OS keychain"
                } else {
                    "none"
                };
                match fmt {
                    OutputFormat::Pretty => {
                        println!("Token source:  {source}");
                        println!("  --api-token: {}", if has_arg { "set" } else { "not set" });
                        println!("  env var:     {}", if has_env { "set" } else { "not set" });
                        println!("  keychain:    {}", if has_keychain { "stored" } else { "empty" });
                    }
                    _ => emit(&serde_json::json!({
                        "active_source": source,
                        "api_token_flag": has_arg,
                        "env_var": has_env,
                        "keychain": has_keychain,
                    }), &fmt)?,
                }
                return Ok(());
            }
            AuthCommands::Jwt => {} // handled below with client
        }
    }

    let client = make_client(&cli)?;

    match cli.command {
        Commands::Auth { action } => {
            // Only Jwt reaches here
            match action {
                AuthCommands::Jwt => {
                    let jwt = client.authenticate().await.context("Authentication failed")?;
                    match fmt {
                        OutputFormat::Pretty => println!("{jwt}"),
                        _ => emit(&serde_json::json!({ "jwt": jwt }), &fmt)?,
                    }
                }
                _ => unreachable!(),
            }
        }

        Commands::Projects { name } => {
            let resp = client
                .list_all_projects(name.as_deref(), 25)
                .await
                .context("Failed to list projects")?;

            match fmt {
                OutputFormat::Pretty => {
                    if resp.data.is_empty() {
                        println!("No projects found.");
                        return Ok(());
                    }
                    println!("{} projects found.\n", resp.data.len());
                    println!("{:<40} {:<40} DESCRIPTION", "ID", "NAME");
                    println!("{}", "-".repeat(100));
                    for p in &resp.data {
                        println!(
                            "{:<40} {:<40} {}",
                            p.id,
                            p.attributes.name,
                            p.attributes.description.as_deref().unwrap_or("-")
                        );
                    }
                }
                _ => {
                    let items: Vec<serde_json::Value> = resp
                        .data
                        .iter()
                        .map(|p| {
                            serde_json::json!({
                                "id": p.id,
                                "name": p.attributes.name,
                                "description": p.attributes.description,
                            })
                        })
                        .collect();
                    emit(&serde_json::json!(items), &fmt)?;
                }
            }
        }

        Commands::Branches { project_id } => {
            let resp = client
                .list_all_branches(&project_id, 25)
                .await
                .context("Failed to list branches")?;

            match fmt {
                OutputFormat::Pretty => {
                    if resp.data.is_empty() {
                        println!("No branches found.");
                        return Ok(());
                    }
                    println!("{} branches found.\n", resp.data.len());
                    println!("{:<40} {:<30} MAIN", "ID", "NAME");
                    println!("{}", "-".repeat(80));
                    for b in &resp.data {
                        println!(
                            "{:<40} {:<30} {}",
                            b.id,
                            b.attributes.name,
                            if b.attributes.main_for_project.unwrap_or(false) { "✓" } else { "" }
                        );
                    }
                }
                _ => {
                    let items: Vec<serde_json::Value> = resp
                        .data
                        .iter()
                        .map(|b| {
                            serde_json::json!({
                                "id": b.id,
                                "name": b.attributes.name,
                                "main": b.attributes.main_for_project.unwrap_or(false),
                            })
                        })
                        .collect();
                    emit(&serde_json::json!(items), &fmt)?;
                }
            }
        }

        Commands::Issues {
            project_id,
            branch_id,
        } => {
            let branch_id = resolve_branch(&client, &project_id, branch_id).await?;

            let resp = client
                .list_all_issues(&project_id, Some(&branch_id), None, 25)
                .await
                .context("Failed to list issues")?;

            match fmt {
                OutputFormat::Pretty => {
                    if resp.data.is_empty() {
                        println!("No issues found.");
                        return Ok(());
                    }
                    println!("{} issues found.\n", resp.data.len());

                    let included_map = build_included_map(&resp.included);

                    println!(
                        "{:<12} {:<64} {:<20} {:<10} TYPE",
                        "ID (short)", "ISSUE-KEY", "CHECKER", "SEVERITY",
                    );
                    println!("{}", "-".repeat(130));

                    for issue in &resp.data {
                        let short_id = &issue.id[..issue.id.len().min(10)];
                        let severity = resolve_included(&issue.relationships, "/severity/data/id", "taxon", &included_map);
                        let issue_type = resolve_included(&issue.relationships, "/issue-type/data/id", "issue-type", &included_map);

                        println!(
                            "{:<12} {:<64} {:<20} {:<10} {}",
                            short_id,
                            issue.attributes.issue_key,
                            issue.attributes.sub_tool.as_deref().unwrap_or("-"),
                            severity,
                            issue_type,
                        );
                    }
                }
                _ => {
                    let included_map = build_included_map(&resp.included);
                    let items: Vec<serde_json::Value> = resp
                        .data
                        .iter()
                        .map(|issue| {
                            let severity = resolve_included(&issue.relationships, "/severity/data/id", "taxon", &included_map);
                            let issue_type = resolve_included(&issue.relationships, "/issue-type/data/id", "issue-type", &included_map);
                            serde_json::json!({
                                "id": issue.id,
                                "issue_key": issue.attributes.issue_key,
                                "finding_key": issue.attributes.finding_key,
                                "checker": issue.attributes.sub_tool,
                                "severity": severity,
                                "type": issue_type,
                            })
                        })
                        .collect();
                    emit(&serde_json::json!(items), &fmt)?;
                }
            }
        }

        Commands::IssueShow {
            issue_id,
            project_id,
            branch_id,
        } => {
            let branch_id = resolve_branch(&client, &project_id, branch_id).await?;

            let val: serde_json::Value = client
                .get_issue(&issue_id, &project_id, &branch_id)
                .await
                .context("Failed to get issue")?;

            match fmt {
                OutputFormat::Pretty => {
                    print_issue_detail(&val, &cli.base_url, &project_id, &branch_id);

                    // Also fetch and show main event if we have finding-key and run-id
                    let data = val.get("data").unwrap_or(&val);
                    let finding_key = data
                        .pointer("/attributes/finding-key")
                        .and_then(|v| v.as_str());
                    let run_id = data
                        .pointer("/relationships/latest-observed-on-run/data/id")
                        .and_then(|v| v.as_str());

                    if let (Some(fk), Some(rid)) = (finding_key, run_id) {
                        match client.get_events_with_source(fk, rid, None, Some(1)).await {
                            Ok(events) => {
                                print_events_summary(&events);
                            }
                            Err(e) => {
                                eprintln!("\n(Could not fetch events: {e})");
                            }
                        }
                    }
                }
                _ => emit(&val, &fmt)?,
            }
        }

        Commands::Events {
            finding_key,
            run_id,
            occurrence,
            max_depth,
        } => {
            let events = client
                .get_events_with_source(&finding_key, &run_id, occurrence, max_depth)
                .await
                .context("Failed to get events")?;

            match fmt {
                OutputFormat::Pretty => {
                    print_event_tree(&events);
                }
                _ => emit(&events, &fmt)?,
            }
        }

        Commands::Triage { action } => match action {
            TriageAction::Get {
                project_id,
                issue_key,
            } => {
                let resp = client
                    .get_triage(&project_id, &issue_key)
                    .await
                    .context("Failed to get triage")?;

                match fmt {
                    OutputFormat::Pretty => {
                        if resp.data.is_empty() {
                            println!("No triage data found.");
                            return Ok(());
                        }
                        for tc in &resp.data {
                            println!("Issue key:        {}", tc.attributes.issue_key);
                            println!("Project ID:       {}", tc.attributes.project_id);
                            println!(
                                "Dismissal status: {}",
                                tc.attributes.dismissal_status.as_deref().unwrap_or("N/A")
                            );
                            if !tc.attributes.triage_current_values.is_empty() {
                                println!("Triage values:");
                                for val in &tc.attributes.triage_current_values {
                                    println!("  {}", serde_json::to_string_pretty(val).unwrap_or_default());
                                }
                            }
                        }
                    }
                    _ => {
                        let val = serde_json::to_value(&resp)?;
                        emit(&val, &fmt)?;
                    }
                }
            }

            TriageAction::Update {
                project_id,
                issue_keys,
                dismiss,
                owner,
                comment,
            } => {
                if dismiss.is_none() && owner.is_none() && comment.is_none() {
                    anyhow::bail!("At least one of --dismiss, --owner, or --comment is required");
                }

                let keys: Vec<&str> = issue_keys.iter().map(|s| s.as_str()).collect();
                let values = TriageValues {
                    dismiss,
                    owner,
                    commentary: comment,
                };

                let resp = client
                    .update_triage(&project_id, &keys, &values)
                    .await
                    .context("Failed to update triage")?;

                match fmt {
                    OutputFormat::Pretty => println!("Triage updated successfully."),
                    _ => emit(&resp, &fmt)?,
                }
            }

            TriageAction::History {
                project_id,
                issue_key,
                limit,
            } => {
                let resp = client
                    .get_triage_history(&project_id, &issue_key, limit, 0)
                    .await
                    .context("Failed to get triage history")?;

                emit(&resp, &fmt)?;
            }
        },
    }

    Ok(())
}

// ── Helpers ──

async fn resolve_branch(
    client: &PolarisClient,
    project_id: &str,
    branch_id: Option<String>,
) -> Result<String> {
    match branch_id {
        Some(id) => Ok(id),
        None => {
            let branches = client
                .list_all_branches(project_id, 25)
                .await
                .context("Failed to list branches to find main branch")?;
            branches
                .data
                .iter()
                .find(|b| b.attributes.main_for_project.unwrap_or(false))
                .map(|b| b.id.clone())
                .context("No main branch found; specify --branch-id explicitly")
        }
    }
}

fn build_included_map(included: &[serde_json::Value]) -> std::collections::HashMap<String, &serde_json::Value> {
    let mut map = std::collections::HashMap::new();
    for inc in included {
        if let (Some(t), Some(id)) = (
            inc.get("type").and_then(|v| v.as_str()),
            inc.get("id").and_then(|v| v.as_str()),
        ) {
            map.insert(format!("{t}:{id}"), inc);
        }
    }
    map
}

fn resolve_included<'a>(
    relationships: &Option<serde_json::Value>,
    rel_path: &str,
    type_prefix: &str,
    included_map: &'a std::collections::HashMap<String, &serde_json::Value>,
) -> &'a str {
    relationships
        .as_ref()
        .and_then(|r| r.pointer(rel_path))
        .and_then(|id| id.as_str())
        .and_then(|id| included_map.get(&format!("{type_prefix}:{id}")))
        .and_then(|v| v.pointer("/attributes/name"))
        .and_then(|v| v.as_str())
        .unwrap_or("-")
}

fn print_issue_detail(val: &serde_json::Value, base_url: &str, project_id: &str, branch_id: &str) {
    let data = val.get("data").unwrap_or(val);

    let id = data.pointer("/id").and_then(|v| v.as_str()).unwrap_or("-");
    let issue_key = data
        .pointer("/attributes/issue-key")
        .and_then(|v| v.as_str())
        .unwrap_or("-");
    let finding_key = data
        .pointer("/attributes/finding-key")
        .and_then(|v| v.as_str())
        .unwrap_or("-");
    let sub_tool = data
        .pointer("/attributes/sub-tool")
        .and_then(|v| v.as_str())
        .unwrap_or("-");
    let first_detected = data
        .pointer("/attributes/first-detected-on")
        .and_then(|v| v.as_str())
        .unwrap_or("-");

    // Resolve included resources
    let included = val.get("included").and_then(|v| v.as_array());
    let included_map: std::collections::HashMap<String, &serde_json::Value> = included
        .map(|arr| {
            arr.iter()
                .filter_map(|inc| {
                    let t = inc.get("type")?.as_str()?;
                    let id = inc.get("id")?.as_str()?;
                    Some((format!("{t}:{id}"), inc))
                })
                .collect()
        })
        .unwrap_or_default();

    let severity_id = data.pointer("/relationships/severity/data/id").and_then(|v| v.as_str());
    let severity = severity_id
        .and_then(|id| included_map.get(&format!("taxon:{id}")))
        .and_then(|v| v.pointer("/attributes/name"))
        .and_then(|v| v.as_str())
        .unwrap_or("-");

    let type_id = data.pointer("/relationships/issue-type/data/id").and_then(|v| v.as_str());
    let issue_type = type_id
        .and_then(|id| included_map.get(&format!("issue-type:{id}")))
        .and_then(|v| v.pointer("/attributes/name"))
        .and_then(|v| v.as_str())
        .unwrap_or("-");

    let tool_id = data.pointer("/relationships/tool-domain-service/data/id").and_then(|v| v.as_str());
    let tool = tool_id
        .and_then(|id| included_map.get(&format!("tool-domain-service:{id}")))
        .and_then(|v| v.pointer("/attributes/name"))
        .and_then(|v| v.as_str())
        .unwrap_or("-");

    let path_val = data.pointer("/relationships/path/data/id").and_then(|v| v.as_str());
    let path = path_val
        .and_then(|id| included_map.get(&format!("path:{id}")))
        .and_then(|v| v.pointer("/attributes/path"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join("/")
        })
        .unwrap_or_else(|| "-".to_string());

    // Resolve revision ID from included transition resource
    let revision_id = included
        .and_then(|arr| {
            arr.iter()
                .filter(|inc| inc.get("type").and_then(|v| v.as_str()) == Some("transition"))
                .filter_map(|inc| {
                    inc.pointer("/attributes/revision-id")
                        .and_then(|v| v.as_str())
                })
                .next()
        });

    // Build path query param from included path resource
    let path_query = path_val
        .and_then(|id| included_map.get(&format!("path:{id}")))
        .and_then(|v| v.pointer("/attributes/path"))
        .and_then(|v| v.as_array())
        .map(|arr| {
            let parts: Vec<String> = arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| format!("\"{}\"", s)))
                .collect();
            format!("[{}]", parts.join(","))
        });

    println!("Issue:          {issue_key}");
    println!("ID:             {id}");
    println!("Severity:       {severity}");
    println!("Type:           {issue_type}");
    println!("Checker:        {sub_tool}");
    println!("Tool:           {tool}");
    println!("Path:           {path}");
    println!("Finding key:    {finding_key}");
    println!("First detected: {first_detected}");

    // Construct web URL
    let mut url = format!("{base_url}/projects/{project_id}/branches/{branch_id}");
    if let Some(rev_id) = revision_id {
        url.push_str(&format!("/revisions/{rev_id}"));
    }
    url.push_str(&format!("/issues/{id}?pagingOffset=0"));
    if let Some(ref pq) = path_query {
        url.push_str(&format!("&path={}", urlencoding::encode(pq)));
    }
    println!("URL:            {url}");
}

/// Print a short summary of events (used in issue show).
fn print_events_summary(events: &serde_json::Value) {
    let data = events.get("data").and_then(|v| v.as_array());
    let data = match data {
        Some(d) if !d.is_empty() => d,
        _ => return,
    };

    println!("\n── Event Summary ──");
    for event_tree in data {
        let main_file = event_tree
            .get("main-event-file-path")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join("/")
            })
            .unwrap_or_else(|| "-".to_string());
        let main_line = event_tree
            .get("main-event-line-number")
            .and_then(|v| v.as_u64())
            .map(|n| n.to_string())
            .unwrap_or_else(|| "-".to_string());
        let language = event_tree
            .get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("-");

        println!("Main event:     {main_file}:{main_line} ({language})");

        // Show first few events
        if let Some(evts) = event_tree.get("events").and_then(|v| v.as_array()) {
            for evt in evts.iter().take(5) {
                let desc = evt
                    .get("event-description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-");
                let file = evt
                    .get("filePath")
                    .and_then(|v| v.as_str())
                    .unwrap_or("-");
                let line = evt
                    .get("line-number")
                    .and_then(|v| v.as_u64())
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "-".to_string());
                let etype = evt
                    .get("event-type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let tag = match etype {
                    "main" => "►",
                    "path" => "→",
                    "evidence" => "╴",
                    _ => " ",
                };
                println!("  {tag} {file}:{line}: {desc}");

                // Show source snippet if available
                if let Some(src) = evt.get("source-before") {
                    print_snippet(src);
                }
                if let Some(src) = evt.get("source-after") {
                    print_snippet(src);
                }
            }
            if evts.len() > 5 {
                println!("  ... and {} more events (use `polaris events` for full tree)", evts.len() - 5);
            }
        }
    }
}

/// Print the full event tree (used in `events` command).
fn print_event_tree(events: &serde_json::Value) {
    let data = events.get("data").and_then(|v| v.as_array());
    let data = match data {
        Some(d) if !d.is_empty() => d,
        _ => {
            println!("No events found.");
            return;
        }
    };

    for event_tree in data {
        let main_file = event_tree
            .get("main-event-file-path")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join("/")
            })
            .unwrap_or_else(|| "-".to_string());
        let main_line = event_tree
            .get("main-event-line-number")
            .and_then(|v| v.as_u64())
            .map(|n| n.to_string())
            .unwrap_or_else(|| "-".to_string());
        let language = event_tree
            .get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("-");

        println!("Finding:  {}", event_tree.get("finding-key").and_then(|v| v.as_str()).unwrap_or("-"));
        println!("Main:     {main_file}:{main_line}");
        println!("Language: {language}\n");

        if let Some(evts) = event_tree.get("events").and_then(|v| v.as_array()) {
            print_events_recursive(evts, 0);
        }
    }
}

fn print_events_recursive(events: &[serde_json::Value], indent: usize) {
    let pad = "  ".repeat(indent);
    for evt in events {
        let desc = evt
            .get("event-description")
            .and_then(|v| v.as_str())
            .unwrap_or("-");
        let file = evt
            .get("filePath")
            .and_then(|v| v.as_str())
            .unwrap_or("-");
        let line = evt
            .get("line-number")
            .and_then(|v| v.as_u64())
            .map(|n| n.to_string())
            .unwrap_or_else(|| "-".to_string());
        let etype = evt
            .get("event-type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let tag = match etype {
            "main" => "►",
            "path" => "→",
            "evidence" => "╴",
            "example" => "◆",
            _ => " ",
        };

        println!("{pad}{tag} {file}:{line}: {desc}");

        // Source snippets
        if let Some(src) = evt.get("source-before") {
            print_snippet_indented(src, indent + 1);
        }
        if let Some(src) = evt.get("source-after") {
            print_snippet_indented(src, indent + 1);
        }

        // Recurse into evidence events
        if let Some(children) = evt.get("evidence-events").and_then(|v| v.as_array())
            && !children.is_empty()
        {
            print_events_recursive(children, indent + 1);
        }
    }
}

fn print_snippet(src: &serde_json::Value) {
    print_snippet_indented(src, 2);
}

fn print_snippet_indented(src: &serde_json::Value, indent: usize) {
    let code = match src.get("source-code").and_then(|v| v.as_str()) {
        Some(c) if !c.is_empty() => c,
        _ => return,
    };
    let start = src
        .get("start-line")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let pad = "  ".repeat(indent);

    for (i, line) in code.lines().enumerate() {
        let lineno = start + i as u64;
        println!("{pad}  {lineno:>5} │ {line}");
    }
}
