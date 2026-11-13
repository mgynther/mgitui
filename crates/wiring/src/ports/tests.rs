use super::{SessionPorts, diff_paths};
use crate::log::LogBuffer;
use crate::test_support::{TempDir, fixed_clock, git_ok, init_repo};
use logic::log::{Layer, LogEntry, LogLevel};
use logic::model::{ChangeKind, ChangedFile, FileDiff, LineKind, Side, WorkStatus};
use logic::ports::{CommitMessage, Failed, Ports, SearchText};

#[expect(
    clippy::unnecessary_wraps,
    reason = "it has the signature of a terminal handoff"
)]
fn run_here(run: &mut dyn FnMut()) -> std::io::Result<()> {
    run();
    Ok(())
}

fn append_line(path: &std::path::Path) -> std::io::Result<()> {
    let old: String = std::fs::read_to_string(path).unwrap();
    std::fs::write(path, format!("{old}line\n"))
}

fn with_ports(dir: &std::path::Path, test: impl Fn(&SessionPorts<'_>, &LogBuffer)) {
    let log: LogBuffer = LogBuffer::new(fixed_clock);
    let git: git::Git = git::Git::new(dir).with_temp_dir(dir);
    let ports: SessionPorts<'_> = SessionPorts {
        log: &log,
        git: &git,
        editor: &append_line,
        handoff: &run_here,
    };
    test(&ports, &log);
}

fn file(path: &str, kind: ChangeKind) -> ChangedFile {
    ChangedFile {
        path: path.to_string(),
        original_path: None,
        kind,
        mode_changed: false,
    }
}

#[test]
fn logic_log_goes_to_buffer() {
    with_ports(
        std::path::Path::new("."),
        |ports: &SessionPorts<'_>, log: &LogBuffer| {
            ports.log(LogLevel::Info, "hello");
            let entries: Vec<LogEntry> = log.drain();
            assert_eq!(entries[0].layer, Layer::Logic);
            assert_eq!(entries[0].message, "hello");
        },
    );
}

#[test]
fn search_is_validated_by_validation_layer() {
    with_ports(
        std::path::Path::new("."),
        |ports: &SessionPorts<'_>, log: &LogBuffer| {
            assert_eq!(ports.validate_search("x"), Ok(SearchText("x".to_string())));
            assert_eq!(
                ports.validate_search(""),
                Err("search text is empty".to_string())
            );
            let entries: Vec<LogEntry> = log.drain();
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].layer, Layer::Validation);
            assert_eq!(entries[0].level, LogLevel::Error);
        },
    );
}

#[test]
fn status_and_diffs_from_real_repository() {
    let dir: TempDir = TempDir::new();
    init_repo(dir.path());
    std::fs::write(dir.path().join("a.txt"), "a\n").unwrap();
    git_ok(dir.path(), &["add", "."]);
    git_ok(dir.path(), &["commit", "-q", "-m", "init"]);
    std::fs::write(dir.path().join("a.txt"), "b\n").unwrap();
    std::fs::write(dir.path().join("new.txt"), "n\n").unwrap();
    git_ok(dir.path(), &["mv", "a.txt", "moved.txt"]);
    with_ports(dir.path(), |ports: &SessionPorts<'_>, log: &LogBuffer| {
        let status: WorkStatus = ports.status().unwrap();
        assert_eq!(status.staged[0].path, "moved.txt");
        assert_eq!(status.staged[0].kind, ChangeKind::Renamed);
        let renamed: FileDiff = ports.file_diff(&status.staged[0], Side::Staged).unwrap();
        assert!(renamed.header.contains(&"rename from a.txt".to_string()));
        let changed: FileDiff = ports
            .file_diff(&status.unstaged[0], Side::Unstaged)
            .unwrap();
        assert_eq!(changed.hunks[0].lines[0].kind, LineKind::Removed);
        let untracked: FileDiff = ports
            .file_diff(&file("new.txt", ChangeKind::Untracked), Side::Unstaged)
            .unwrap();
        assert_eq!(untracked.hunks[0].lines[0].text, "n");
        assert!(log.drain().iter().all(|e: &LogEntry| e.layer == Layer::Git));
    });
}

#[test]
fn git_failures_become_failed() {
    let dir: TempDir = TempDir::new();
    let log: LogBuffer = LogBuffer::new(fixed_clock);
    let git: git::Git = git::Git::new(dir.path())
        .with_program("/nonexistent/git")
        .with_temp_dir(&dir.path().join("missing"));
    let ports: SessionPorts<'_> = SessionPorts {
        log: &log,
        git: &git,
        editor: &append_line,
        handoff: &run_here,
    };
    assert_eq!(ports.status(), Err(Failed));
    assert_eq!(
        ports.file_diff(&file("x", ChangeKind::Modified), Side::Unstaged),
        Err(Failed)
    );
    assert_eq!(
        ports.stage_file(&file("x", ChangeKind::Modified)),
        Err(Failed)
    );
    assert_eq!(
        ports.unstage_file(&file("x", ChangeKind::Modified)),
        Err(Failed)
    );
    assert_eq!(ports.apply_to_index("", false), Err(Failed));
    assert_eq!(ports.has_commits(), Err(Failed));
    assert_eq!(ports.commits(0, 1), Err(Failed));
    assert_eq!(ports.refs(), Err(Failed));
    assert_eq!(ports.commit_message("h"), Err(Failed));
    assert_eq!(
        ports.create_branch(&logic::ports::BranchName("b".to_string()), "h"),
        Err(Failed)
    );
    assert_eq!(ports.checkout("b"), Err(Failed));
    assert_eq!(ports.cherry_pick("h"), Err(Failed));
    assert_eq!(ports.remote_update(), Err(Failed));
    assert_eq!(ports.delete_branch("b"), Err(Failed));
    assert_eq!(ports.rebase("b", "main", false), Err(Failed));
    assert_eq!(ports.rebase("b", "main", true), Err(Failed));
    assert_eq!(ports.commit_files("h"), Err(Failed));
    assert_eq!(
        ports.commit_file_diff("h", &file("x", ChangeKind::Modified)),
        Err(Failed)
    );
    assert_eq!(ports.last_commit_message(), Err(Failed));
    assert_eq!(ports.edit_message("x"), Err(Failed));
    assert_eq!(
        ports.commit(&CommitMessage("m".to_string()), false),
        Err(Failed)
    );
    let logged: Vec<LogEntry> = log.drain();
    assert!(logged.iter().any(|e: &LogEntry| e.layer == Layer::Wiring
        && e.message.starts_with("editing the commit message failed: ")));
}

#[test]
fn rename_diffs_need_both_paths_in_the_index() {
    let renamed: ChangedFile = ChangedFile {
        original_path: Some("old".to_string()),
        ..file("new", ChangeKind::Renamed)
    };
    assert_eq!(diff_paths(&renamed, Side::Staged), vec!["old", "new"]);
    assert_eq!(diff_paths(&renamed, Side::Unstaged), vec!["new"]);
}

#[test]
fn staging_a_renamed_file_back_and_forth() {
    let dir: TempDir = TempDir::new();
    init_repo(dir.path());
    std::fs::write(dir.path().join("a.txt"), "a\n").unwrap();
    git_ok(dir.path(), &["add", "."]);
    git_ok(dir.path(), &["commit", "-q", "-m", "init"]);
    git_ok(dir.path(), &["mv", "a.txt", "b.txt"]);
    with_ports(dir.path(), |ports: &SessionPorts<'_>, _log: &LogBuffer| {
        let first: String = ports.commits(0, 1).unwrap()[0].hash.clone();
        let files: Vec<ChangedFile> = ports.commit_files(&first).unwrap();
        assert_eq!(files[0].kind, ChangeKind::Added);
        let shown: FileDiff = ports.commit_file_diff(&first, &files[0]).unwrap();
        assert_eq!(shown.hunks[0].lines[0].text, "a");
        let renamed: ChangedFile = ports.status().unwrap().staged[0].clone();
        ports.unstage_file(&renamed).unwrap();
        let status: WorkStatus = ports.status().unwrap();
        assert_eq!(status.staged, vec![]);
        assert_eq!(status.unstaged.len(), 2);
        for changed in &status.unstaged {
            ports.stage_file(changed).unwrap();
        }
        assert_eq!(ports.status().unwrap().staged[0].kind, ChangeKind::Renamed);
        ports
            .apply_to_index("--- a/b.txt\n+++ b/b.txt\n@@ -1 +1 @@\n-a\n+z\n", false)
            .unwrap();
        assert_eq!(git_ok(dir.path(), &["show", ":b.txt"]), "z\n");
    });
}

#[test]
fn edit_validate_commit_and_amend() {
    let dir: TempDir = TempDir::new();
    init_repo(dir.path());
    std::fs::write(dir.path().join("a.txt"), "a\n").unwrap();
    git_ok(dir.path(), &["add", "."]);
    with_ports(dir.path(), |ports: &SessionPorts<'_>, log: &LogBuffer| {
        assert_eq!(ports.has_commits(), Ok(false));
        assert_eq!(
            ports.edit_message("First\n"),
            Ok("First\nline\n".to_string())
        );
        assert_eq!(
            ports.validate_commit_message(" "),
            Err("commit message is empty".to_string())
        );
        let message: CommitMessage = ports.validate_commit_message("First\n").unwrap();
        ports.commit(&message, false).unwrap();
        assert_eq!(ports.has_commits(), Ok(true));
        assert_eq!(ports.last_commit_message(), Ok("First\n".to_string()));
        ports
            .commit(&CommitMessage("Second\n".to_string()), true)
            .unwrap();
        assert_eq!(git_ok(dir.path(), &["log", "--format=%s"]), "Second\n");
        assert!(
            log.drain()
                .iter()
                .any(|e: &LogEntry| e.layer == Layer::Validation)
        );
    });
}

#[test]
fn history_and_refs() {
    let dir: TempDir = TempDir::new();
    init_repo(dir.path());
    git_ok(dir.path(), &["commit", "-q", "--allow-empty", "-m", "one"]);
    git_ok(dir.path(), &["tag", "v1"]);
    with_ports(dir.path(), |ports: &SessionPorts<'_>, _log: &LogBuffer| {
        let commits: Vec<logic::model::Commit> = ports.commits(0, 10).unwrap();
        assert_eq!(commits[0].subject, "one");
        let refs: logic::model::Refs = ports.refs().unwrap();
        assert_eq!(refs.head, Some(commits[0].hash.clone()));
        assert_eq!(refs.labels.len(), 2);
        assert_eq!(
            ports.commit_message(&commits[0].hash),
            Ok("one\n\n".to_string())
        );
        assert_eq!(ports.commit_files(&commits[0].hash), Ok(vec![]));
    });
}

#[test]
fn refs_fail_when_head_fails() {
    let dir: TempDir = TempDir::new();
    init_repo(dir.path());
    std::fs::write(dir.path().join(".git/HEAD"), "garbage\n").unwrap();
    with_ports(dir.path(), |ports: &SessionPorts<'_>, _log: &LogBuffer| {
        assert_eq!(ports.refs(), Err(Failed));
        assert_eq!(ports.commit_message("h"), Err(Failed));
        assert_eq!(
            ports.create_branch(&logic::ports::BranchName("b".to_string()), "h"),
            Err(Failed)
        );
        assert_eq!(ports.checkout("b"), Err(Failed));
        assert_eq!(ports.commit_files("h"), Err(Failed));
        assert_eq!(
            ports.commit_file_diff("h", &file("x", ChangeKind::Modified)),
            Err(Failed)
        );
    });
}

#[test]
fn branch_validation_creation_and_checkout() {
    let dir: TempDir = TempDir::new();
    init_repo(dir.path());
    git_ok(dir.path(), &["commit", "-q", "--allow-empty", "-m", "one"]);
    with_ports(dir.path(), |ports: &SessionPorts<'_>, log: &LogBuffer| {
        assert_eq!(
            ports.validate_branch_name("a..b"),
            Err("invalid branch name: it contains '..'".to_string())
        );
        assert_eq!(ports.validate_checkout_choice("2", 3), Ok(1));
        assert_eq!(
            ports.validate_checkout_choice("9", 3),
            Err("choice must be a number from 1 to 3".to_string())
        );
        let name: logic::ports::BranchName = ports.validate_branch_name("topic").unwrap();
        let head: String = ports.commits(0, 1).unwrap()[0].hash.clone();
        ports.create_branch(&name, &head).unwrap();
        ports.checkout("topic").unwrap();
        assert_eq!(git_ok(dir.path(), &["branch", "--show-current"]), "topic\n");
        let validation_errors: usize = log
            .drain()
            .iter()
            .filter(|e: &&LogEntry| e.layer == Layer::Validation)
            .count();
        assert_eq!(validation_errors, 2);
    });
}

#[test]
fn interactive_rebase_hands_the_terminal_to_git() {
    let dir: TempDir = TempDir::new();
    init_repo(dir.path());
    git_ok(dir.path(), &["commit", "-q", "--allow-empty", "-m", "base"]);
    git_ok(dir.path(), &["switch", "-q", "-c", "topic"]);
    git_ok(dir.path(), &["commit", "-q", "--allow-empty", "-m", "one"]);
    git_ok(dir.path(), &["commit", "-q", "--allow-empty", "-m", "two"]);
    git_ok(dir.path(), &["switch", "-q", "main"]);
    git_ok(
        dir.path(),
        &["commit", "-q", "--allow-empty", "-m", "main work"],
    );
    let log: LogBuffer = LogBuffer::new(fixed_clock);
    let git: git::Git = git::Git::new(dir.path())
        .with_env("GIT_SEQUENCE_EDITOR", "sed -i 2s/^pick/fixup/")
        .with_env("GIT_EDITOR", "true");
    let handed: std::cell::Cell<usize> = std::cell::Cell::new(0);
    let counting = |run: &mut dyn FnMut()| -> std::io::Result<()> {
        handed.set(handed.get() + 1);
        run();
        Ok(())
    };
    let ports: SessionPorts<'_> = SessionPorts {
        log: &log,
        git: &git,
        editor: &append_line,
        handoff: &counting,
    };
    assert_eq!(ports.rebase("topic", "main", true), Ok(()));
    assert_eq!(handed.get(), 1);
    assert_eq!(
        git_ok(dir.path(), &["log", "--format=%s", "topic"]),
        "one\nmain work\nbase\n"
    );
    assert_eq!(ports.rebase("main", "topic", false), Ok(()));
    let broken =
        |_: &mut dyn FnMut()| -> std::io::Result<()> { Err(std::io::Error::other("no tty")) };
    let failing: SessionPorts<'_> = SessionPorts {
        log: &log,
        git: &git,
        editor: &append_line,
        handoff: &broken,
    };
    assert_eq!(failing.rebase("topic", "main", true), Err(Failed));
    assert!(
        log.drain()
            .iter()
            .any(|e: &LogEntry| e.layer == Layer::Wiring
                && e.message == "handing the terminal to git failed: no tty")
    );
}
