use std::path::Path;

/// Errors that can occur when reading file content.
#[derive(Debug)]
pub enum FileReadError {
    /// The repo_path is not a valid git repository.
    NotAGitRepo,
    /// The file was not found.
    FileNotFound(String),
    /// Git command failed.
    GitError(String),
}

impl std::fmt::Display for FileReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileReadError::NotAGitRepo => write!(f, "not a git repository"),
            FileReadError::FileNotFound(path) => write!(f, "file not found: {path}"),
            FileReadError::GitError(msg) => write!(f, "git error: {msg}"),
        }
    }
}

impl std::error::Error for FileReadError {}

/// Validate that a path is a git repository.
pub fn validate_repo_path(repo_path: &Path) -> Result<(), FileReadError> {
    if repo_path.join(".git").exists() {
        Ok(())
    } else {
        Err(FileReadError::NotAGitRepo)
    }
}

/// Read the current (new) version of a file from the working directory.
pub fn read_new_file(repo_path: &Path, file_path: &str) -> Result<String, FileReadError> {
    let full_path = repo_path.join(file_path);
    std::fs::read_to_string(&full_path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => FileReadError::FileNotFound(file_path.to_string()),
        _ => FileReadError::GitError(e.to_string()),
    })
}

/// Read the old version of a file from git at the given ref.
pub fn read_old_file(
    repo_path: &Path,
    file_path: &str,
    base_ref: &str,
) -> Result<String, FileReadError> {
    let output = std::process::Command::new("git")
        .args([
            "-C",
            &repo_path.to_string_lossy(),
            "show",
            &format!("{base_ref}:{file_path}"),
        ])
        .output()
        .map_err(|e| FileReadError::GitError(e.to_string()))?;

    if output.status.success() {
        String::from_utf8(output.stdout).map_err(|e| FileReadError::GitError(e.to_string()))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(FileReadError::GitError(stderr.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    /// Create a temp git repo with an initial committed file,
    /// then modify the working copy so old != new.
    fn setup_git_repo() -> TempDir {
        let dir = TempDir::new().unwrap();
        let p = dir.path();

        Command::new("git")
            .args(["init"])
            .current_dir(p)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(p)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(p)
            .output()
            .unwrap();

        // Create and commit a file
        std::fs::write(p.join("hello.rs"), "fn main() {}\n").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(p)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "init"])
            .current_dir(p)
            .output()
            .unwrap();

        // Modify the working copy
        std::fs::write(
            p.join("hello.rs"),
            "fn main() {\n    println!(\"hello\");\n}\n",
        )
        .unwrap();

        dir
    }

    #[test]
    fn read_new_file_returns_working_copy() {
        let dir = setup_git_repo();
        let content = read_new_file(dir.path(), "hello.rs").unwrap();
        assert!(content.contains("println"));
    }

    #[test]
    fn read_new_file_not_found() {
        let dir = setup_git_repo();
        let result = read_new_file(dir.path(), "nonexistent.rs");
        assert!(matches!(result, Err(FileReadError::FileNotFound(_))));
    }

    #[test]
    fn read_old_file_returns_committed_version() {
        let dir = setup_git_repo();
        let content = read_old_file(dir.path(), "hello.rs", "HEAD").unwrap();
        assert_eq!(content, "fn main() {}\n");
        assert!(!content.contains("println"));
    }

    #[test]
    fn read_old_file_bad_ref() {
        let dir = setup_git_repo();
        let result = read_old_file(dir.path(), "hello.rs", "nonexistent-ref");
        assert!(matches!(result, Err(FileReadError::GitError(_))));
    }

    #[test]
    fn read_old_file_not_in_commit() {
        let dir = setup_git_repo();
        let result = read_old_file(dir.path(), "doesnotexist.rs", "HEAD");
        assert!(matches!(result, Err(FileReadError::GitError(_))));
    }

    #[test]
    fn validate_repo_path_valid() {
        let dir = setup_git_repo();
        assert!(validate_repo_path(dir.path()).is_ok());
    }

    #[test]
    fn validate_repo_path_not_a_repo() {
        let dir = TempDir::new().unwrap();
        assert!(matches!(
            validate_repo_path(dir.path()),
            Err(FileReadError::NotAGitRepo)
        ));
    }
}
