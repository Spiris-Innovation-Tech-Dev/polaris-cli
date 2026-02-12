use serde::Deserialize;

/// Response from POST /api/auth/v2/authenticate
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthenticateResponse {
    pub jwt: String,
}

/// Request body is application/x-www-form-urlencoded with `accesstoken` field.
/// The response returns a JWT in the body for API token auth.
pub struct AuthClient {
    http: reqwest::Client,
    base_url: String,
}

impl AuthClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Authenticate with an API token to get a JWT.
    pub async fn authenticate_with_token(&self, api_token: &str) -> crate::error::Result<String> {
        let url = format!("{}/api/auth/v2/authenticate", self.base_url);

        let resp = self
            .http
            .post(&url)
            .header("Accept", "application/json")
            .form(&[("accesstoken", api_token)])
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(crate::error::PolarisError::AuthFailed(format!(
                "HTTP {status}: {body}"
            )));
        }

        let auth_resp: AuthenticateResponse = resp
            .json::<AuthenticateResponse>()
            .await
            .map_err(|e: reqwest::Error| crate::error::PolarisError::Deserialize(e.to_string()))?;

        Ok(auth_resp.jwt)
    }
}
