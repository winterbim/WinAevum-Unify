//! Aevum Unify — branch provider (M8).
//!
//! All git operations on the agentic path go through the `BranchProvider`
//! trait. The production implementation is `LocalGit` (which shells out to
//! the real `git` binary with a typed argv[]) and the test-friendly
//! implementation is `InMemoryGit`. Both share the same `ProviderError`
//! shape so callers can swap them without code changes.

use std::fmt;
use std::sync::Mutex;

// sentinel-kernel is not strictly needed at this layer; the dependency
// is kept in Cargo.toml so future M11 wiring can swap in the real
// capability check without a churn in callers.

const MAX_BRANCH_LEN: usize = 64;
const SHELL_METACHARS: &[char] = &[';', '&', '|', '$', '`', '>', '<', '*', '?', '!', '\n', '\r'];

#[derive(Debug)]
pub enum ProviderError {
    /// The sentinel rejected the argument (shell metachar, writes to main,
    /// empty/overlong name, etc). Caller should NOT retry.
    RejectedCapability(String),
    /// The provider failed to perform the operation (e.g. network, IO).
    ExecutionFailed(String),
}

impl fmt::Display for ProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProviderError::RejectedCapability(m) => write!(f, "rejected: {m}"),
            ProviderError::ExecutionFailed(m) => write!(f, "execution failed: {m}"),
        }
    }
}

impl std::error::Error for ProviderError {}

#[derive(Debug, Clone)]
pub struct MemoryRepo {
    pub path: String,
    pub default_branch: String,
}

impl MemoryRepo {
    pub fn new(path: impl Into<String>, default_branch: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            default_branch: default_branch.into(),
        }
    }
}

/// Common shape of every git provider we ship. Implementations must be
/// safe to substitute in tests because the `unify` CLI uses the same
/// trait object.
pub trait BranchProvider {
    /// Create a branch with the given name. Refuses to operate on `main`
    /// directly and refuses any shell metacharacter in the name.
    fn create_branch(&mut self, name: &str) -> Result<String, ProviderError> {
        self.create_branch_on(&MemoryRepo::new(".", "main"), name)
    }

    fn create_branch_on(&mut self, repo: &MemoryRepo, name: &str) -> Result<String, ProviderError>;
}

/// In-memory provider, used by tests and as the local-first default
/// (no real git calls are made). Records every branch creation in an
/// audit log so callers can verify the gate was called.
#[derive(Debug, Default)]
pub struct InMemoryGit {
    inner: Mutex<InMemoryGitState>,
}

#[derive(Debug, Default)]
struct InMemoryGitState {
    branches: Vec<String>,
    audit: Vec<String>,
}

impl InMemoryGit {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn list_branches(&self) -> Vec<String> {
        self.inner.lock().unwrap().branches.clone()
    }

    pub fn audit_log(&self) -> Vec<String> {
        self.inner.lock().unwrap().audit.clone()
    }
}

fn validate_branch_name(name: &str) -> Result<(), ProviderError> {
    if name.is_empty() {
        return Err(ProviderError::RejectedCapability(
            "branch name is empty".into(),
        ));
    }
    if name == "main" || name == "master" {
        return Err(ProviderError::RejectedCapability(
            "writes against the default branch are denied (D04)".into(),
        ));
    }
    if name.len() > MAX_BRANCH_LEN {
        return Err(ProviderError::RejectedCapability(format!(
            "branch name exceeds {MAX_BRANCH_LEN} chars"
        )));
    }
    if name.chars().any(|c| SHELL_METACHARS.contains(&c)) {
        return Err(ProviderError::RejectedCapability(
            "shell metacharacter in branch name (D16)".into(),
        ));
    }
    Ok(())
}

impl BranchProvider for InMemoryGit {
    fn create_branch_on(&mut self, repo: &MemoryRepo, name: &str) -> Result<String, ProviderError> {
        validate_branch_name(name)?;
        let mut s = self.inner.lock().unwrap();
        s.branches.push(name.to_string());
        let audit_line = format!("{} @ {} -> {}", repo.path, repo.default_branch, name);
        s.audit.push(audit_line);
        Ok(format!("memory://{}/branches/{}", repo.path, name))
    }
}

/// LocalGit driver — wraps the system `git` binary using a typed argv[]
/// (no shell, no `sh -c`). Spawns `git` directly via `std::process::Command`
/// and returns the resulting `refs/heads/<name>` ref. Caller must run inside
/// an existing repository (the `MemoryRepo.path` is the `-C <path>` target).
pub struct LocalGit {
    pub git_path: String,
}

impl LocalGit {
    pub fn new() -> Self {
        Self {
            git_path: "git".to_string(),
        }
    }
}

impl Default for LocalGit {
    fn default() -> Self {
        Self::new()
    }
}

impl BranchProvider for LocalGit {
    fn create_branch_on(&mut self, repo: &MemoryRepo, name: &str) -> Result<String, ProviderError> {
        validate_branch_name(name)?;
        // No shell, no `sh -c`. `argv` is a typed array that we hand straight
        // to `Command::new(...).args(...)`. Any non-zero exit or spawn error
        // becomes a `ProviderError::ExecutionFailed` with the captured stderr.
        let argv: [&str; 5] = [
            self.git_path.as_str(),
            "-C",
            repo.path.as_str(),
            "checkout",
            "-b",
        ];
        if argv.iter().any(|a| a.is_empty()) || name.is_empty() {
            return Err(ProviderError::ExecutionFailed("empty argv entry".into()));
        }
        let output = std::process::Command::new(argv[0])
            .args(&argv[1..])
            .arg(name)
            .output()
            .map_err(|e| ProviderError::ExecutionFailed(format!("spawn {}: {e}", argv[0])))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            return Err(ProviderError::ExecutionFailed(format!(
                "git checkout -b {name} exited {}: stderr={stderr:?} stdout={stdout:?}",
                output.status
            )));
        }
        Ok(format!("refs/heads/{name}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn validate_branch_name_accepts_normal_names() {
        assert!(validate_branch_name("aevum/sec-fix").is_ok());
        assert!(validate_branch_name("feature-add-thing").is_ok());
    }

    #[test]
    fn validate_branch_name_rejects_metachars() {
        assert!(validate_branch_name("a;b").is_err());
        assert!(validate_branch_name("a&b").is_err());
        assert!(validate_branch_name("a|b").is_err());
        assert!(validate_branch_name("a$b").is_err());
        assert!(validate_branch_name("a`b").is_err());
    }

    #[test]
    fn local_git_returns_real_branch_ref() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().to_str().unwrap();
        std::process::Command::new("git").args(["init", "--initial-branch=main"]).arg(p).output().unwrap();
        std::process::Command::new("git").args(["-C", p, "config", "user.email", "t@t"]).output().unwrap();
        std::process::Command::new("git").args(["-C", p, "config", "user.name", "t"]).output().unwrap();
        std::fs::write(tmp.path().join("f"), "x").unwrap();
        std::process::Command::new("git").args(["-C", p, "add", "f"]).output().unwrap();
        std::process::Command::new("git").args(["-C", p, "commit", "-m", "i"]).output().unwrap();
        let mut g = LocalGit::new();
        let r = g.create_branch_on(&MemoryRepo::new(p, "main"), "aevum/x").unwrap();
        assert_eq!(r, "refs/heads/aevum/x");
    }
}
