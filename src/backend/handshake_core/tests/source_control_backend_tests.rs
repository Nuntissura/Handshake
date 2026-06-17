use std::{fs, path::Path, process::Command};

use handshake_core::source_control::{
    DiffScope, SourceControlError, SourceControlRepository, StatusCode,
};
use tempfile::tempdir;

#[test]
fn source_control_status_diff_stage_unstage_and_discard_use_real_git_repo() {
    let repo_dir = tempdir().expect("temp git repo");
    init_repo(repo_dir.path());
    write(repo_dir.path(), "tracked.txt", "initial\n");
    git(repo_dir.path(), &["add", "tracked.txt"]);
    git(repo_dir.path(), &["commit", "-m", "initial commit"]);

    write(repo_dir.path(), "tracked.txt", "initial\nchanged\n");
    write(repo_dir.path(), "new.txt", "new file\n");

    let repo = SourceControlRepository::open(repo_dir.path()).expect("open source control repo");
    let status = repo.status().expect("status");

    let tracked = status
        .entries
        .iter()
        .find(|entry| entry.path == "tracked.txt")
        .expect("tracked file status");
    assert_eq!(tracked.index, None);
    assert_eq!(tracked.worktree, Some(StatusCode::Modified));

    let untracked = status
        .entries
        .iter()
        .find(|entry| entry.path == "new.txt")
        .expect("untracked file status");
    assert_eq!(untracked.index, Some(StatusCode::Untracked));
    assert_eq!(untracked.worktree, Some(StatusCode::Untracked));

    let worktree_diff = repo
        .diff("tracked.txt", DiffScope::Worktree)
        .expect("worktree diff");
    assert!(worktree_diff.patch.contains("+changed"));

    repo.stage(&["tracked.txt"]).expect("stage tracked file");
    let staged_status = repo.status().expect("staged status");
    let tracked = staged_status
        .entries
        .iter()
        .find(|entry| entry.path == "tracked.txt")
        .expect("tracked staged status");
    assert_eq!(tracked.index, Some(StatusCode::Modified));
    assert_eq!(tracked.worktree, None);

    let staged_diff = repo
        .diff("tracked.txt", DiffScope::Staged)
        .expect("staged diff");
    assert!(staged_diff.patch.contains("+changed"));

    repo.unstage(&["tracked.txt"])
        .expect("unstage tracked file");
    let unstaged_status = repo.status().expect("unstaged status");
    let tracked = unstaged_status
        .entries
        .iter()
        .find(|entry| entry.path == "tracked.txt")
        .expect("tracked unstaged status");
    assert_eq!(tracked.index, None);
    assert_eq!(tracked.worktree, Some(StatusCode::Modified));

    let cancel_error = repo
        .discard(&["tracked.txt"], false)
        .expect_err("discard without confirmation must be blocked");
    assert!(matches!(
        cancel_error,
        SourceControlError::DiscardRequiresConfirmation
    ));
    assert_eq!(
        fs::read_to_string(repo_dir.path().join("tracked.txt")).expect("tracked content"),
        "initial\nchanged\n"
    );

    repo.discard(&["tracked.txt"], true)
        .expect("confirmed discard");
    assert_eq!(
        fs::read_to_string(repo_dir.path().join("tracked.txt")).expect("tracked content"),
        "initial\n"
    );
    assert!(repo
        .status()
        .expect("clean status after discard")
        .entries
        .iter()
        .all(|entry| entry.path != "tracked.txt"));
}

#[test]
fn source_control_commit_log_branch_and_blame_use_real_git_history() {
    let repo_dir = tempdir().expect("temp git repo");
    init_repo(repo_dir.path());
    write(repo_dir.path(), "story.txt", "one\n");
    let repo = SourceControlRepository::open(repo_dir.path()).expect("open source control repo");

    repo.stage(&["story.txt"]).expect("stage first story");
    let first_commit = repo
        .commit("add story")
        .expect("commit appears in real git history");
    assert_eq!(first_commit.message, "add story");
    assert_eq!(first_commit.id.len(), 40);

    let log = repo.log(5).expect("git log");
    assert_eq!(
        log.entries.first().expect("latest log").message,
        "add story"
    );

    repo.create_branch("review-lane").expect("create branch");
    repo.switch_branch("review-lane").expect("switch branch");
    let branches = repo.branches().expect("branches");
    assert!(branches
        .iter()
        .any(|branch| branch.name == "review-lane" && branch.current));

    write(repo_dir.path(), "story.txt", "one\ntwo\n");
    repo.stage(&["story.txt"]).expect("stage second story");
    let second_commit = repo.commit("extend story").expect("second commit");
    let blame = repo.blame("story.txt").expect("blame story");

    assert!(blame.lines.iter().any(|line| line.line_number == 2
        && line.commit_id == second_commit.id
        && line.content == "two"));
}

#[test]
fn source_control_rejects_path_escape_before_invoking_git() {
    let repo_dir = tempdir().expect("temp git repo");
    init_repo(repo_dir.path());
    let repo = SourceControlRepository::open(repo_dir.path()).expect("open source control repo");

    let error = repo
        .stage(&["../outside.txt"])
        .expect_err("path escape must be rejected");

    match error {
        SourceControlError::InvalidPath { path, .. } => assert_eq!(path, "../outside.txt"),
        other => panic!("expected InvalidPath, got {other:?}"),
    }
}

#[test]
fn source_control_treats_wildcard_filenames_as_literal_paths() {
    let repo_dir = tempdir().expect("temp git repo");
    init_repo(repo_dir.path());
    write(repo_dir.path(), "literal[ab].txt", "bracket file\n");
    write(repo_dir.path(), "literala.txt", "glob peer\n");
    git(repo_dir.path(), &["add", "literal[ab].txt", "literala.txt"]);
    git(repo_dir.path(), &["commit", "-m", "add wildcard names"]);

    let repo = SourceControlRepository::open(repo_dir.path()).expect("open source control repo");
    let blame = repo
        .blame("literal[ab].txt")
        .expect("wildcard filename must be addressed literally");

    assert_eq!(blame.lines.len(), 1);
    assert_eq!(blame.lines[0].content, "bracket file");
}

#[test]
fn source_control_discard_removes_staged_new_files_after_confirmation() {
    let repo_dir = tempdir().expect("temp git repo");
    init_repo(repo_dir.path());
    write(repo_dir.path(), "tracked.txt", "initial\n");
    git(repo_dir.path(), &["add", "tracked.txt"]);
    git(repo_dir.path(), &["commit", "-m", "initial commit"]);

    write(repo_dir.path(), "new-staged.txt", "new staged file\n");
    let repo = SourceControlRepository::open(repo_dir.path()).expect("open source control repo");
    repo.stage(&["new-staged.txt"]).expect("stage new file");

    let status = repo.status().expect("status after staged add");
    let staged_add = status
        .entries
        .iter()
        .find(|entry| entry.path == "new-staged.txt")
        .expect("staged add status");
    assert_eq!(staged_add.index, Some(StatusCode::Added));

    repo.discard(&["new-staged.txt"], true)
        .expect("confirmed discard removes staged add");

    assert!(!repo_dir.path().join("new-staged.txt").exists());
    assert!(repo
        .status()
        .expect("status after staged-add discard")
        .entries
        .iter()
        .all(|entry| entry.path != "new-staged.txt"));
}

#[test]
fn source_control_status_parses_renames_without_bogus_extra_entries() {
    let repo_dir = tempdir().expect("temp git repo");
    init_repo(repo_dir.path());
    write(repo_dir.path(), "old-name.txt", "renamed\n");
    git(repo_dir.path(), &["add", "old-name.txt"]);
    git(repo_dir.path(), &["commit", "-m", "initial commit"]);
    fs::rename(
        repo_dir.path().join("old-name.txt"),
        repo_dir.path().join("new-name.txt"),
    )
    .expect("rename fixture");
    git(repo_dir.path(), &["add", "-A"]);

    let repo = SourceControlRepository::open(repo_dir.path()).expect("open source control repo");
    let status = repo.status().expect("rename status");

    assert_eq!(status.entries.len(), 1);
    assert_eq!(status.entries[0].path, "new-name.txt");
    assert_eq!(status.entries[0].index, Some(StatusCode::Renamed));
    assert_eq!(status.entries[0].worktree, None);
}

#[test]
fn source_control_discard_removes_untracked_directories_after_confirmation() {
    let repo_dir = tempdir().expect("temp git repo");
    init_repo(repo_dir.path());
    write(repo_dir.path(), "tracked.txt", "initial\n");
    git(repo_dir.path(), &["add", "tracked.txt"]);
    git(repo_dir.path(), &["commit", "-m", "initial commit"]);
    write(repo_dir.path(), "scratch/nested.txt", "scratch\n");

    let repo = SourceControlRepository::open(repo_dir.path()).expect("open source control repo");
    repo.discard(&["scratch"], true)
        .expect("confirmed discard removes untracked directory");

    assert!(!repo_dir.path().join("scratch").exists());
}

#[test]
fn source_control_rejects_branch_shorthand_and_full_ref_names() {
    let repo_dir = tempdir().expect("temp git repo");
    init_repo(repo_dir.path());
    write(repo_dir.path(), "tracked.txt", "initial\n");
    git(repo_dir.path(), &["add", "tracked.txt"]);
    git(repo_dir.path(), &["commit", "-m", "initial commit"]);
    let repo = SourceControlRepository::open(repo_dir.path()).expect("open source control repo");

    for branch_name in ["@{-1}", "refs/heads/confusing", "-starts-with-dash"] {
        let error = repo
            .create_branch(branch_name)
            .expect_err("branch name must be rejected");
        assert!(matches!(
            error,
            SourceControlError::InvalidBranchName { .. }
        ));
    }
}

#[test]
fn source_control_empty_repo_log_returns_empty_entries() {
    let repo_dir = tempdir().expect("temp git repo");
    init_repo(repo_dir.path());
    let repo = SourceControlRepository::open(repo_dir.path()).expect("open source control repo");

    let log = repo.log(10).expect("empty repo log");

    assert!(log.entries.is_empty());
}

fn init_repo(path: &Path) {
    git(path, &["init", "-b", "main"]);
    git(path, &["config", "user.name", "Handshake Test"]);
    git(path, &["config", "user.email", "handshake@example.invalid"]);
    git(path, &["config", "core.autocrlf", "false"]);
}

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, contents).expect("write fixture");
}

fn git(path: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["-c", "core.longpaths=true"])
        .args(args)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
