use std::sync::Arc;
use tokio::sync::RwLock;

use crate::auth::AuthClient;
use crate::common::{CommonClient, JsonApiResponse, Project, Branch};
use crate::error::{PolarisError, Result};

/// Configuration for the Polaris client.
#[derive(Debug, Clone)]
pub struct PolarisConfig {
    pub base_url: String,
    pub api_token: String,
}

impl PolarisConfig {
    pub fn from_env() -> Result<Self> {
        let api_token = std::env::var("POLARIS_API_TOKEN")
            .map_err(|_| PolarisError::Other("POLARIS_API_TOKEN env var not set".into()))?;
        let base_url = std::env::var("POLARIS_BASE_URL")
            .unwrap_or_else(|_| "https://your-instance.polaris.blackduck.com".into());
        Ok(Self {
            base_url,
            api_token,
        })
    }
}

/// High-level client for the BlackDuck Polaris API.
pub struct PolarisClient {
    config: PolarisConfig,
    auth: AuthClient,
    jwt: Arc<RwLock<Option<String>>>,
}

impl PolarisClient {
    pub fn new(config: PolarisConfig) -> Self {
        let auth = AuthClient::new(&config.base_url);
        Self {
            config,
            auth,
            jwt: Arc::new(RwLock::new(None)),
        }
    }

    /// Authenticate and return the JWT. Caches the JWT for subsequent calls.
    pub async fn authenticate(&self) -> Result<String> {
        let jwt = self
            .auth
            .authenticate_with_token(&self.config.api_token)
            .await?;
        *self.jwt.write().await = Some(jwt.clone());
        Ok(jwt)
    }

    /// Get the current JWT, authenticating if needed.
    async fn get_jwt(&self) -> Result<String> {
        {
            let jwt = self.jwt.read().await;
            if let Some(ref j) = *jwt {
                return Ok(j.clone());
            }
        }
        self.authenticate().await
    }

    fn common_client(&self, jwt: &str) -> CommonClient {
        CommonClient::new(&self.config.base_url, jwt)
    }

    fn authed_http(&self, jwt: &str) -> reqwest::Client {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {jwt}").parse().unwrap(),
        );
        headers.insert(
            reqwest::header::ACCEPT,
            "application/vnd.api+json".parse().unwrap(),
        );
        reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap()
    }

    // ── Projects ──

    /// List projects, optionally filtering by name.
    pub async fn list_projects(
        &self,
        name_filter: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> Result<JsonApiResponse<Project>> {
        let jwt = self.get_jwt().await?;
        self.common_client(&jwt)
            .list_projects(name_filter, limit, offset)
            .await
    }

    /// Fetch all projects by auto-paginating.
    pub async fn list_all_projects(
        &self,
        name_filter: Option<&str>,
        page_size: u32,
    ) -> Result<JsonApiResponse<Project>> {
        let mut all_data = Vec::new();
        let mut all_included = Vec::new();
        let mut offset = 0u32;
        let mut total = None;

        loop {
            let resp = self.list_projects(name_filter, page_size, offset).await?;
            if let Some(ref meta) = resp.meta {
                total = meta.total;
            }
            let count = resp.data.len();
            all_data.extend(resp.data);
            all_included.extend(resp.included);
            if count < page_size as usize {
                break;
            }
            offset += page_size;
            if let Some(t) = total {
                if offset as u64 >= t {
                    break;
                }
            }
        }

        Ok(JsonApiResponse {
            data: all_data,
            included: all_included,
            meta: total.map(|t| crate::common::PaginationMeta {
                offset: Some(0),
                limit: None,
                total: Some(t),
            }),
        })
    }

    /// List branches for a project.
    pub async fn list_branches(
        &self,
        project_id: &str,
        limit: u32,
        offset: u32,
    ) -> Result<JsonApiResponse<Branch>> {
        let jwt = self.get_jwt().await?;
        self.common_client(&jwt)
            .list_branches(project_id, limit, offset)
            .await
    }

    /// Fetch all branches for a project by auto-paginating.
    pub async fn list_all_branches(
        &self,
        project_id: &str,
        page_size: u32,
    ) -> Result<JsonApiResponse<Branch>> {
        let mut all_data = Vec::new();
        let mut offset = 0u32;
        let mut total = None;

        loop {
            let resp = self.list_branches(project_id, page_size, offset).await?;
            if let Some(ref meta) = resp.meta {
                total = meta.total;
            }
            let count = resp.data.len();
            all_data.extend(resp.data);
            if count < page_size as usize {
                break;
            }
            offset += page_size;
            if let Some(t) = total {
                if offset as u64 >= t {
                    break;
                }
            }
        }

        Ok(JsonApiResponse {
            data: all_data,
            included: vec![],
            meta: total.map(|t| crate::common::PaginationMeta {
                offset: Some(0),
                limit: None,
                total: Some(t),
            }),
        })
    }

    // ── Issues ──

    /// List issues for a project + branch (or run).
    pub async fn list_issues(
        &self,
        project_id: &str,
        branch_id: Option<&str>,
        run_ids: Option<&[&str]>,
        limit: u32,
        offset: u32,
    ) -> Result<IssuesResponse> {
        let jwt = self.get_jwt().await?;
        let http = self.authed_http(&jwt);

        let mut url = format!(
            "{}/api/query/v1/issues?project-id={project_id}&page[limit]={limit}&page[offset]={offset}",
            self.config.base_url
        );

        if let Some(bid) = branch_id {
            url.push_str(&format!("&branch-id={bid}"));
        }
        if let Some(rids) = run_ids {
            for rid in rids {
                url.push_str(&format!("&run-id[]={rid}"));
            }
        }

        // Include common relationships
        url.push_str("&include[issue][]=severity&include[issue][]=issue-type&include[issue][]=tool-domain-service");

        let resp = http.get(&url).send().await?;
        check_response(resp).await
    }

    /// Fetch all issues by auto-paginating.
    pub async fn list_all_issues(
        &self,
        project_id: &str,
        branch_id: Option<&str>,
        run_ids: Option<&[&str]>,
        page_size: u32,
    ) -> Result<IssuesResponse> {
        let mut all_data = Vec::new();
        let mut all_included = Vec::new();
        let mut offset = 0u32;
        let mut total = None;

        loop {
            let resp = self
                .list_issues(project_id, branch_id, run_ids, page_size, offset)
                .await?;
            if let Some(ref meta) = resp.meta {
                total = meta.total;
            }
            let count = resp.data.len();
            all_data.extend(resp.data);
            all_included.extend(resp.included);
            if count < page_size as usize {
                break;
            }
            offset += page_size;
            if let Some(t) = total {
                if offset as u64 >= t {
                    break;
                }
            }
        }

        Ok(IssuesResponse {
            data: all_data,
            included: all_included,
            meta: total.map(|t| IssuesMeta {
                offset: Some(0),
                limit: None,
                total: Some(t),
            }),
        })
    }

    /// Get a single issue by ID.
    pub async fn get_issue(
        &self,
        issue_id: &str,
        project_id: &str,
        branch_id: &str,
    ) -> Result<serde_json::Value> {
        let jwt = self.get_jwt().await?;
        let http = self.authed_http(&jwt);

        let url = format!(
            "{}/api/query/v1/issues/{issue_id}?project-id={project_id}&branch-id={branch_id}&include[issue][]=severity&include[issue][]=issue-type&include[issue][]=tool-domain-service&include[issue][]=path&include[issue][]=transitions",
            self.config.base_url
        );

        let resp = http.get(&url).send().await?;
        check_response(resp).await
    }

    // ── Code Analysis Events ──

    /// Get the event tree with source code snippets for a finding.
    pub async fn get_events_with_source(
        &self,
        finding_key: &str,
        run_id: &str,
        occurrence_number: Option<u32>,
        max_depth: Option<u32>,
    ) -> Result<serde_json::Value> {
        let jwt = self.get_jwt().await?;
        let http = self.authed_http(&jwt);

        let mut url = format!(
            "{}/api/code-analysis/v0/events-with-source?finding-key={finding_key}&run-id={run_id}",
            self.config.base_url
        );
        if let Some(occ) = occurrence_number {
            url.push_str(&format!("&occurrence-number={occ}"));
        }
        if let Some(depth) = max_depth {
            url.push_str(&format!("&max-depth={depth}"));
        }

        let resp = http
            .get(&url)
            .header("Accept-Language", "en")
            .header("Accept", "application/json")
            .send()
            .await?;
        check_response(resp).await
    }

    /// Get full source code for a file in a run.
    pub async fn get_source_code(
        &self,
        run_id: &str,
        path: &str,
    ) -> Result<String> {
        let jwt = self.get_jwt().await?;
        let http = self.authed_http(&jwt);

        let url = format!(
            "{}/api/code-analysis/v0/source-code?run-id={run_id}&path={path}",
            self.config.base_url
        );

        let resp = http
            .get(&url)
            .header("Accept", "text/plain")
            .send()
            .await?;
        let status = resp.status();
        if !status.is_success() {
            let detail = resp.text().await.unwrap_or_default();
            return Err(PolarisError::Api {
                status: status.as_u16(),
                detail,
            });
        }
        Ok(resp.text().await?)
    }

    // ── Triage ──

    /// Get current triage status for an issue.
    pub async fn get_triage(
        &self,
        project_id: &str,
        issue_key: &str,
    ) -> Result<TriageCurrentResponse> {
        let jwt = self.get_jwt().await?;
        let http = self.authed_http(&jwt);

        let url = format!(
            "{}/api/triage-query/v1/triage-current?filter[triage-current][project-id][$eq]={project_id}&filter[triage-current][issue-key][$eq]={issue_key}",
            self.config.base_url
        );

        let resp = http.get(&url).send().await?;
        check_response(resp).await
    }

    /// Update triage for one or more issues.
    pub async fn update_triage(
        &self,
        project_id: &str,
        issue_keys: &[&str],
        triage_values: &TriageValues,
    ) -> Result<serde_json::Value> {
        let jwt = self.get_jwt().await?;
        let http = self.authed_http(&jwt);

        let url = format!(
            "{}/api/triage-command/v1/triage-issues",
            self.config.base_url
        );

        let mut triage_map = serde_json::Map::new();
        if let Some(ref dismiss) = triage_values.dismiss {
            triage_map.insert("DISMISS".into(), serde_json::Value::String(dismiss.clone()));
        }
        if let Some(ref owner) = triage_values.owner {
            triage_map.insert("OWNER".into(), serde_json::Value::String(owner.clone()));
        }
        if let Some(ref comment) = triage_values.commentary {
            triage_map.insert("COMMENTARY".into(), serde_json::Value::String(comment.clone()));
        }

        let body = serde_json::json!({
            "data": {
                "type": "triage-issues",
                "attributes": {
                    "project-id": project_id,
                    "issue-keys": issue_keys,
                    "triage-values": triage_map,
                }
            }
        });

        let resp = http
            .post(&url)
            .header("Content-Type", "application/vnd.api+json")
            .json(&body)
            .send()
            .await?;

        check_response(resp).await
    }

    /// Get triage history for an issue.
    pub async fn get_triage_history(
        &self,
        project_id: &str,
        issue_key: &str,
        limit: u32,
        offset: u32,
    ) -> Result<serde_json::Value> {
        let jwt = self.get_jwt().await?;
        let http = self.authed_http(&jwt);

        let url = format!(
            "{}/api/triage-query/v1/triage-history-items?filter[triage-history-items][project-id][$eq]={project_id}&filter[triage-history-items][issue-key][$eq]={issue_key}&page[limit]={limit}&page[offset]={offset}",
            self.config.base_url
        );

        let resp = http.get(&url).send().await?;
        check_response(resp).await
    }
}

// ── Response types ──

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct IssuesResponse {
    pub data: Vec<Issue>,
    #[serde(default)]
    pub included: Vec<serde_json::Value>,
    #[serde(default)]
    pub meta: Option<IssuesMeta>,
}

#[derive(Debug, Deserialize)]
pub struct IssuesMeta {
    #[serde(rename = "total")]
    pub total: Option<u64>,
    #[serde(rename = "offset")]
    pub offset: Option<u64>,
    #[serde(rename = "limit")]
    pub limit: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct Issue {
    #[serde(rename = "type")]
    pub resource_type: String,
    pub id: String,
    pub attributes: IssueAttributes,
    #[serde(default)]
    pub relationships: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct IssueAttributes {
    #[serde(rename = "issue-key")]
    pub issue_key: String,
    #[serde(rename = "finding-key")]
    pub finding_key: String,
    #[serde(rename = "sub-tool", default)]
    pub sub_tool: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TriageCurrentResponse {
    pub data: Vec<TriageCurrent>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TriageCurrent {
    #[serde(rename = "type")]
    pub resource_type: String,
    pub id: String,
    pub attributes: TriageCurrentAttributes,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TriageCurrentAttributes {
    #[serde(rename = "issue-key")]
    pub issue_key: String,
    #[serde(rename = "project-id")]
    pub project_id: String,
    #[serde(rename = "dismissal-status", default)]
    pub dismissal_status: Option<String>,
    #[serde(rename = "triage-current-values", default)]
    pub triage_current_values: Vec<serde_json::Value>,
}

/// Values for updating triage on issues.
#[derive(Debug, Clone, Default, Serialize)]
pub struct TriageValues {
    /// Dismiss value: NOT_DISMISSED, DISMISSED_BY_DESIGN, DISMISSED_AS_FP, etc.
    pub dismiss: Option<String>,
    /// Owner email address.
    pub owner: Option<String>,
    /// Free text comment.
    pub commentary: Option<String>,
}

async fn check_response<T: serde::de::DeserializeOwned>(
    resp: reqwest::Response,
) -> Result<T> {
    let status = resp.status();
    if !status.is_success() {
        let code = status.as_u16();
        let body = resp.text().await.unwrap_or_default();
        if code == 404 {
            return Err(PolarisError::NotFound(body));
        }
        return Err(PolarisError::Api {
            status: code,
            detail: body,
        });
    }

    resp.json::<T>()
        .await
        .map_err(|e| PolarisError::Deserialize(e.to_string()))
}
