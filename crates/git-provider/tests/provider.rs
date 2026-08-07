use aevum_git_provider::{BranchProvider, InMemoryGit, MemoryRepo, ProviderError};
use tempfile::TempDir;

#[test]
fn in_memory_provider_rejects_shell_metacharacters() {
    let mut p = InMemoryGit::new();
    let err = p.create_branch("main;rm -rf /").unwrap_err();
    assert!(matches!(err, ProviderError::RejectedCapability(_)));
}

#[test]
fn in_memory_provider_rejects_writes_against_main() {
    let mut p = InMemoryGit::new();
    let err = p.create_branch("main").unwrap_err();
    assert!(matches!(err, ProviderError::RejectedCapability(_)));
}

#[test]
fn in_memory_provider_creates_branch_on_side() {
    let mut p = InMemoryGit::new();
    let repo = MemoryRepo::new("/tmp/sandbox", "main");
    let r = p.create_branch_on(&repo, "aevum/sec-fix").unwrap();
    assert!(r.contains("aevum/sec-fix"));
    assert!(p.list_branches().contains(&"aevum/sec-fix".to_string()));
}

#[test]
fn in_memory_provider_records_calls_as_audit_log() {
    let mut p = InMemoryGit::new();
    let repo = MemoryRepo::new("/tmp/sandbox", "main");
    p.create_branch_on(&repo, "aevum/sec-fix").unwrap();
    p.create_branch_on(&repo, "aevum/feat").unwrap();
    let log = p.audit_log();
    assert_eq!(log.len(), 2);
    assert!(log[0].contains("aevum/sec-fix"));
    assert!(log[1].contains("aevum/feat"));
}

#[test]
fn provider_rejects_empty_branch_name() {
    let mut p = InMemoryGit::new();
    let err = p.create_branch("").unwrap_err();
    assert!(matches!(err, ProviderError::RejectedCapability(_)));
}

#[test]
fn provider_rejects_overlong_branch_name() {
    let mut p = InMemoryGit::new();
    let long = "a".repeat(120);
    let err = p.create_branch(&long).unwrap_err();
    assert!(matches!(err, ProviderError::RejectedCapability(_)));
}

#[test]
fn local_git_creates_real_branch_in_a_repo() {
    // init a real git repo in a tempdir so the spawn actually has something
    // to operate on.
    let tmp = TempDir::new().unwrap();
    let repo_path = tmp.path().to_str().unwrap();
    std::process::Command::new("git")
        .args(["init", "--initial-branch=main"])
        .arg(repo_path)
        .output()
        .expect("git init");
    std::process::Command::new("git")
        .args(["-C", repo_path, "config", "user.email", "test@aevum"])
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["-C", repo_path, "config", "user.name", "aevum"])
        .output()
        .unwrap();
    std::fs::write(tmp.path().join("README.md"), "# init").unwrap();
    std::process::Command::new("git")
        .args(["-C", repo_path, "add", "README.md"])
        .output()
        .unwrap();
    std::process::Command::new("git")
        .args(["-C", repo_path, "commit", "-m", "init"])
        .output()
        .unwrap();

    let mut g = aevum_git_provider::LocalGit::new();
    let result = g
        .create_branch_on(&MemoryRepo::new(repo_path, "main"), "aevum/sec-fix")
        .unwrap();
    // result is now the on-disk branch ref, not a fake argv string
    assert!(result.starts_with("refs/heads/"), "got {result}");
    assert!(result.ends_with("aevum/sec-fix"));

    // Verify it really exists on disk by listing branches via the git binary.
    let out = std::process::Command::new("git")
        .args(["-C", repo_path, "branch", "--list"])
        .output()
        .unwrap();
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("aevum/sec-fix"), "git branch --list stdout: {s}");
}

#[test]
fn local_git_surfaces_process_errors() {
    // Point LocalGit at a path that doesn't exist; expect ProviderError::ExecutionFailed
    // with a non-empty message, not a fake argv string.
    let mut g = aevum_git_provider::LocalGit::new();
    let bad_path = "/tmp/aevum-nonexistent-12345/";
    let result = g.create_branch_on(&MemoryRepo::new(bad_path, "main"), "aevum/x");
    assert!(result.is_err());
    let err = format!("{}", result.unwrap_err());
    assert!(
        !err.contains("argv"),
        "should be a real process error, got: {err}"
    );
}
