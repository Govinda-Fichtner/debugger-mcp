use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Adapter not found for language: {0}")]
    AdapterNotFound(String),

    #[error("DAP error: {0}")]
    Dap(String),

    #[error("Process error: {0}")]
    Process(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Method not found: {0}")]
    MethodNotFound(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl Error {
    pub fn error_code(&self) -> i32 {
        match self {
            Error::SessionNotFound(_) => -32001,
            Error::AdapterNotFound(_) => -32002,
            Error::Dap(_) => -32003,
            Error::Process(_) => -32004,
            Error::InvalidRequest(_) => -32600,
            Error::MethodNotFound(_) => -32601,
            Error::Internal(_) => -32603,
            Error::Io(_) | Error::Json(_) => -32603,
        }
    }
}
