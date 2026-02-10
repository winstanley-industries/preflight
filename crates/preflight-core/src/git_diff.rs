use std::path::Path;

use crate::diff::FileDiff;
use crate::parser;

#[derive(Debug)]
pub enum GitDiffError {
    NotAGitRepo,
    GitFailed(String),
    ParseFailed(String),
}

impl std::fmt::Display for GitDiffError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitDiffError::NotAGitRepo => write!(f, "not a git repository"),
            GitDiffError::GitFailed(msg) => write!(f, "git diff failed: {msg}"),
            GitDiffError::ParseFailed(msg) => write!(f, "failed to parse diff: {msg}"),
        }
    }
}

impl std::error::Error for GitDiffError {}

/// Detect the default branch for a repo (main or master) and compute the merge-base with HEAD.
/// Returns the merge-base commit hash, or falls back to "HEAD" if detection fails.
pub fn detect_default_base(repo_path: &Path) -> String {
    // Try to detect the default branch via the remote HEAD symref
    let default_branch = std::process::Command::new("git")
        .args([
            "-C",
            &repo_path.to_string_lossy(),
            "symbolic-ref",
            "refs/remotes/origin/HEAD",
        ])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            s.strip_prefix("refs/remotes/origin/")
                .map(|b| b.to_string())
        })
        .or_else(|| {
            // Fallback: check if main or master exists locally
            for branch in &["main", "master"] {
                let result = std::process::Command::new("git")
                    .args([
                        "-C",
                        &repo_path.to_string_lossy(),
                        "rev-parse",
                        "--verify",
                        branch,
                    ])
                    .output();
                if result.is_ok_and(|o| o.status.success()) {
                    return Some(branch.to_string());
                }
            }
            None
        });

    let Some(branch) = default_branch else {
        return "HEAD".to_string();
    };

    // Compute merge-base between the default branch and HEAD
    std::process::Command::new("git")
        .args([
            "-C",
            &repo_path.to_string_lossy(),
            "merge-base",
            &branch,
            "HEAD",
        ])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "HEAD".to_string())
}

/// Run `git diff <base_ref>` in the given repo and return parsed file diffs.
pub fn diff_against_base(repo_path: &Path, base_ref: &str) -> Result<Vec<FileDiff>, GitDiffError> {
    if !repo_path.join(".git").exists() {
        return Err(GitDiffError::NotAGitRepo);
    }
    let output = std::process::Command::new("git")
        .args(["-C", &repo_path.to_string_lossy(), "diff", base_ref, "--"])
        .output()
        .map_err(|e| GitDiffError::GitFailed(e.to_string()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(GitDiffError::GitFailed(stderr.to_string()));
    }
    let diff_text = String::from_utf8_lossy(&output.stdout);
    Ok(parser::parse_diff(&diff_text).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::TempDir;

    fn setup_repo() -> TempDir {
        let dir = TempDir::new().unwrap();
        let p = dir.path();
        Command::new("git")
            .args(["init"])
            .current_dir(p)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "t@t.com"])
            .current_dir(p)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "T"])
            .current_dir(p)
            .output()
            .unwrap();
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
        dir
    }

    #[test]
    fn diff_with_no_changes_returns_empty() {
        let dir = setup_repo();
        let files = diff_against_base(dir.path(), "HEAD").unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn diff_detects_modification() {
        let dir = setup_repo();
        std::fs::write(
            dir.path().join("hello.rs"),
            "fn main() { println!(\"hi\"); }\n",
        )
        .unwrap();
        let files = diff_against_base(dir.path(), "HEAD").unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].new_path.as_deref(), Some("hello.rs"));
    }

    #[test]
    fn diff_detects_new_file() {
        let dir = setup_repo();
        std::fs::write(dir.path().join("new.rs"), "fn new() {}\n").unwrap();
        Command::new("git")
            .args(["add", "new.rs"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        let files = diff_against_base(dir.path(), "HEAD").unwrap();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn diff_not_a_repo() {
        let dir = TempDir::new().unwrap();
        let result = diff_against_base(dir.path(), "HEAD");
        assert!(matches!(result, Err(GitDiffError::NotAGitRepo)));
    }

    #[test]
    fn detect_default_base_falls_back_to_head_for_no_branches() {
        // A repo with only one commit on the default branch — no main/master to diff against
        // if the initial branch is not named main or master.
        let dir = TempDir::new().unwrap();
        let p = dir.path();
        Command::new("git")
            .args(["init", "-b", "trunk"])
            .current_dir(p)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "t@t.com"])
            .current_dir(p)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "T"])
            .current_dir(p)
            .output()
            .unwrap();
        std::fs::write(p.join("f.txt"), "hello\n").unwrap();
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

        let base = detect_default_base(p);
        assert_eq!(base, "HEAD");
    }

    #[test]
    fn detect_default_base_finds_main_branch() {
        let dir = setup_repo();
        let p = dir.path();

        // Rename the default branch to "main"
        Command::new("git")
            .args(["branch", "-M", "main"])
            .current_dir(p)
            .output()
            .unwrap();

        // Create a feature branch with a new commit
        Command::new("git")
            .args(["checkout", "-b", "feature"])
            .current_dir(p)
            .output()
            .unwrap();
        std::fs::write(p.join("feature.rs"), "fn feature() {}\n").unwrap();
        Command::new("git")
            .args(["add", "."])
            .current_dir(p)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "feature"])
            .current_dir(p)
            .output()
            .unwrap();

        let base = detect_default_base(p);
        // Should return the merge-base hash (which is the main branch tip)
        assert_ne!(base, "HEAD");
        assert!(!base.is_empty());
        // Verify it's a valid commit hash
        let verify = Command::new("git")
            .args(["-C", &p.to_string_lossy(), "rev-parse", "--verify", &base])
            .output()
            .unwrap();
        assert!(verify.status.success());
    }

    #[test]
    fn detect_default_base_not_a_repo() {
        let dir = TempDir::new().unwrap();
        let base = detect_default_base(dir.path());
        assert_eq!(base, "HEAD");
    }
}
