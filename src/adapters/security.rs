//! Security utilities for adapter path validation
//!
//! Provides functions to prevent path traversal attacks when debugging programs.

use crate::{Error, Result};
use std::path::{Component, Path, PathBuf};

/// Validates a source file path to prevent path traversal attacks
///
/// Security checks:
/// 1. Canonicalizes path to resolve symlinks and .. components
/// 2. Validates path exists
/// 3. (Optional) Ensures path is within WORKSPACE_ROOT if set
/// 4. (Optional) Validates file extension matches expected language
///
/// # Arguments
///
/// * `path_str` - The path to validate (from user input)
/// * `expected_extension` - Optional file extension to enforce (e.g., Some("rs"), Some("py"))
///
/// # Returns
///
/// Canonicalized path if valid, error otherwise
///
/// # Examples
///
/// ```rust
/// // Rust source file
/// let path = validate_source_path("/workspace/main.rs", Some("rs"))?;
///
/// // Any file type
/// let path = validate_source_path("/workspace/script.py", None)?;
/// ```
///
/// # Security
///
/// This function prevents:
/// - Path traversal: `../../../../etc/passwd`
/// - Symlink attacks: `/workspace/link -> /etc/shadow`
/// - Workspace escapes: `/home/user/secrets.txt` (when WORKSPACE_ROOT is set)
pub fn validate_source_path(path_str: &str, expected_extension: Option<&str>) -> Result<PathBuf> {
    // First validation: check for suspicious patterns before canonicalization
    let path = Path::new(path_str);

    // Reject paths with .. components
    for component in path.components() {
        if component == Component::ParentDir {
            return Err(Error::Compilation(format!(
                "Security: Path contains '..' component: {}",
                path_str
            )));
        }
    }

    // Try to canonicalize (resolves symlinks and .. components, validates existence)
    let canonical = path.canonicalize().map_err(|e| {
        Error::Compilation(format!(
            "Invalid or inaccessible path '{}': {}",
            path_str, e
        ))
    })?;

    // If WORKSPACE_ROOT environment variable is set, ensure path is within workspace
    if let Ok(workspace) = std::env::var("WORKSPACE_ROOT") {
        let workspace_canonical = PathBuf::from(&workspace).canonicalize().map_err(|e| {
            Error::Compilation(format!("Invalid WORKSPACE_ROOT '{}': {}", workspace, e))
        })?;

        if !canonical.starts_with(&workspace_canonical) {
            return Err(Error::Compilation(format!(
                "Security: Path outside workspace. Path: '{}', Workspace: '{}'",
                canonical.display(),
                workspace_canonical.display()
            )));
        }
    }

    // Validate file extension if specified
    if let Some(expected_ext) = expected_extension {
        let actual_ext = canonical.extension().and_then(|s| s.to_str()).unwrap_or("");

        if actual_ext != expected_ext {
            return Err(Error::Compilation(format!(
                "Invalid file extension. Expected '.{}', got: '{}'",
                expected_ext,
                canonical.display()
            )));
        }
    }

    Ok(canonical)
}

/// Validates a working directory path
///
/// Similar to `validate_source_path` but for directory paths (cwd parameter)
///
/// # Arguments
///
/// * `path_str` - The directory path to validate
///
/// # Returns
///
/// Canonicalized directory path if valid, error otherwise
pub fn validate_directory_path(path_str: &str) -> Result<PathBuf> {
    let path = Path::new(path_str);

    // Reject paths with .. components
    for component in path.components() {
        if component == Component::ParentDir {
            return Err(Error::Compilation(format!(
                "Security: Path contains '..' component: {}",
                path_str
            )));
        }
    }

    // Canonicalize and validate existence
    let canonical = path.canonicalize().map_err(|e| {
        Error::Compilation(format!(
            "Invalid or inaccessible directory '{}': {}",
            path_str, e
        ))
    })?;

    // Ensure it's actually a directory
    if !canonical.is_dir() {
        return Err(Error::Compilation(format!(
            "Not a directory: '{}'",
            canonical.display()
        )));
    }

    // If WORKSPACE_ROOT is set, ensure within workspace
    if let Ok(workspace) = std::env::var("WORKSPACE_ROOT") {
        let workspace_canonical = PathBuf::from(&workspace).canonicalize().map_err(|e| {
            Error::Compilation(format!("Invalid WORKSPACE_ROOT '{}': {}", workspace, e))
        })?;

        if !canonical.starts_with(&workspace_canonical) {
            return Err(Error::Compilation(format!(
                "Security: Directory outside workspace. Path: '{}', Workspace: '{}'",
                canonical.display(),
                workspace_canonical.display()
            )));
        }
    }

    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_validate_source_path_rejects_parent_dir() {
        let result = validate_source_path("../../../etc/passwd", None);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("contains '..' component"));
    }

    #[test]
    fn test_validate_source_path_rejects_nonexistent() {
        let result = validate_source_path("/nonexistent/file.rs", Some("rs"));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid or inaccessible"));
    }

    #[test]
    fn test_validate_source_path_validates_extension() {
        // Create a temp file
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_script.py");
        fs::write(&test_file, "# test").unwrap();

        // Should succeed with correct extension
        let result = validate_source_path(test_file.to_str().unwrap(), Some("py"));
        assert!(result.is_ok());

        // Should fail with wrong extension
        let result = validate_source_path(test_file.to_str().unwrap(), Some("rs"));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid file extension"));

        fs::remove_file(test_file).ok();
    }

    #[test]
    fn test_validate_directory_path_rejects_file() {
        // Create a temp file
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_file.txt");
        fs::write(&test_file, "test").unwrap();

        let result = validate_directory_path(test_file.to_str().unwrap());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Not a directory"));

        fs::remove_file(test_file).ok();
    }
}
