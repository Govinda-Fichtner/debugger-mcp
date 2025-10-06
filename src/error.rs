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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_not_found_error() {
        let err = Error::SessionNotFound("test-id".to_string());
        assert_eq!(err.error_code(), -32001);
        assert_eq!(err.to_string(), "Session not found: test-id");
    }

    #[test]
    fn test_adapter_not_found_error() {
        let err = Error::AdapterNotFound("ruby".to_string());
        assert_eq!(err.error_code(), -32002);
        assert_eq!(err.to_string(), "Adapter not found for language: ruby");
    }

    #[test]
    fn test_dap_error() {
        let err = Error::Dap("connection failed".to_string());
        assert_eq!(err.error_code(), -32003);
        assert_eq!(err.to_string(), "DAP error: connection failed");
    }

    #[test]
    fn test_process_error() {
        let err = Error::Process("spawn failed".to_string());
        assert_eq!(err.error_code(), -32004);
        assert_eq!(err.to_string(), "Process error: spawn failed");
    }

    #[test]
    fn test_invalid_request_error() {
        let err = Error::InvalidRequest("malformed JSON".to_string());
        assert_eq!(err.error_code(), -32600);
        assert_eq!(err.to_string(), "Invalid request: malformed JSON");
    }

    #[test]
    fn test_method_not_found_error() {
        let err = Error::MethodNotFound("unknown_method".to_string());
        assert_eq!(err.error_code(), -32601);
        assert_eq!(err.to_string(), "Method not found: unknown_method");
    }

    #[test]
    fn test_internal_error() {
        let err = Error::Internal("unexpected state".to_string());
        assert_eq!(err.error_code(), -32603);
        assert_eq!(err.to_string(), "Internal error: unexpected state");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: Error = io_err.into();
        assert_eq!(err.error_code(), -32603);
    }

    #[test]
    fn test_json_error_conversion() {
        let json_err = serde_json::from_str::<i32>("not a number").unwrap_err();
        let err: Error = json_err.into();
        assert_eq!(err.error_code(), -32603);
    }
}
