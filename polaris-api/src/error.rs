use thiserror::Error;

#[derive(Error, Debug)]
pub enum PolarisError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("API error {status}: {detail}")]
    Api { status: u16, detail: String },

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Deserialization error: {0}")]
    Deserialize(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, PolarisError>;
