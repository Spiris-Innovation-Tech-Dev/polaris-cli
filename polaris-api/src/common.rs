use serde::Deserialize;

// JSON:API resource types for Common Object Service

#[derive(Debug, Deserialize)]
pub struct JsonApiResponse<T> {
    pub data: Vec<T>,
    #[serde(default)]
    pub included: Vec<serde_json::Value>,
    #[serde(default)]
    pub meta: Option<PaginationMeta>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PaginationMeta {
    #[serde(default)]
    pub offset: Option<u64>,
    #[serde(default)]
    pub limit: Option<u64>,
    #[serde(default)]
    pub total: Option<u64>,
}

impl<T> JsonApiResponse<T> {
    /// Returns true if there are more pages to fetch.
    pub fn has_more(&self) -> bool {
        if let Some(meta) = &self.meta {
            if let (Some(offset), Some(limit), Some(total)) = (meta.offset, meta.limit, meta.total)
            {
                return offset + limit < total;
            }
        }
        false
    }

    /// Returns the offset for the next page, or None if no more pages.
    pub fn next_offset(&self) -> Option<u64> {
        if let Some(meta) = &self.meta {
            if let (Some(offset), Some(limit), Some(total)) = (meta.offset, meta.limit, meta.total)
            {
                let next = offset + limit;
                if next < total {
                    return Some(next);
                }
            }
        }
        None
    }
}

#[derive(Debug, Deserialize)]
pub struct JsonApiSingleResponse<T> {
    pub data: T,
    #[serde(default)]
    pub included: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Project {
    #[serde(rename = "type")]
    pub resource_type: String,
    pub id: String,
    pub attributes: ProjectAttributes,
    #[serde(default)]
    pub relationships: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProjectAttributes {
    pub name: String,
    #[serde(rename = "description", default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Branch {
    #[serde(rename = "type")]
    pub resource_type: String,
    pub id: String,
    pub attributes: BranchAttributes,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BranchAttributes {
    pub name: String,
    #[serde(rename = "main-for-project", default)]
    pub main_for_project: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Run {
    #[serde(rename = "type")]
    pub resource_type: String,
    pub id: String,
    pub attributes: RunAttributes,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RunAttributes {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(rename = "date-created", default)]
    pub date_created: Option<String>,
    #[serde(rename = "date-completed", default)]
    pub date_completed: Option<String>,
}

pub struct CommonClient {
    http: reqwest::Client,
    base_url: String,
}

impl CommonClient {
    pub fn new(base_url: &str, jwt: &str) -> Self {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {jwt}").parse().unwrap(),
        );
        headers.insert(
            reqwest::header::ACCEPT,
            "application/vnd.api+json".parse().unwrap(),
        );

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .unwrap();

        Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// List projects, optionally filtering by name.
    pub async fn list_projects(
        &self,
        name_filter: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> crate::error::Result<JsonApiResponse<Project>> {
        let mut url = format!(
            "{}/api/common/v0/projects?page[limit]={limit}&page[offset]={offset}",
            self.base_url
        );

        if let Some(name) = name_filter {
            url.push_str(&format!(
                "&filter[project][name][$eq]={}",
                urlencoding::encode(name)
            ));
        }

        // Always include branches
        url.push_str("&include[project][]=branches");

        let resp = self.http.get(&url).send().await?;
        Self::check_response(resp).await
    }

    /// List branches for a project.
    pub async fn list_branches(
        &self,
        project_id: &str,
        limit: u32,
        offset: u32,
    ) -> crate::error::Result<JsonApiResponse<Branch>> {
        let url = format!(
            "{}/api/common/v0/branches?filter[branch][project][id][$eq]={project_id}&page[limit]={limit}&page[offset]={offset}",
            self.base_url
        );

        let resp = self.http.get(&url).send().await?;
        Self::check_response(resp).await
    }

    /// List runs for a project/revision.
    pub async fn list_runs(
        &self,
        project_id: &str,
        revision_id: Option<&str>,
        limit: u32,
        offset: u32,
    ) -> crate::error::Result<JsonApiResponse<Run>> {
        let mut url = format!(
            "{}/api/common/v0/runs?filter[run][project][id][$eq]={project_id}&page[limit]={limit}&page[offset]={offset}",
            self.base_url
        );

        if let Some(rev) = revision_id {
            url.push_str(&format!("&filter[run][revision][id][$eq]={rev}"));
        }

        let resp = self.http.get(&url).send().await?;
        Self::check_response(resp).await
    }

    async fn check_response<T: serde::de::DeserializeOwned>(
        resp: reqwest::Response,
    ) -> crate::error::Result<T> {
        let status = resp.status();
        if !status.is_success() {
            let code = status.as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(crate::error::PolarisError::Api {
                status: code,
                detail: body,
            });
        }

        resp.json::<T>()
            .await
            .map_err(|e| crate::error::PolarisError::Deserialize(e.to_string()))
    }
}
