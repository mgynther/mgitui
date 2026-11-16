use super::SessionPorts;
use crate::log::LogBuffer;
use crate::test_support::{TempDir, fixed_clock, git_ok, init_repo, test_git};
use logic::log::{Layer, LogEntry, LogLevel};
use logic::model::{AmendBase, ChangeKind, ChangedFile, FileDiff, LineKind, Side, WorkStatus};
use logic::note::{BranchNameProblem, InputProblem};
use logic::ports::{CommitMessage, EditedMessage, Failed, Ports, SearchText};

#[expect(
    clippy::unnecessary_wraps,
    reason = "it has the signature of a terminal handoff"
)]
fn run_here(run: &mut dyn FnMut()) -> std::io::Result<()> {
    run();
    Ok(())
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "it has the signature of an editor"
)]
fn append_line(path: &std::path::Path) -> std::io::Result<terminal::edit::EditorExit> {
    let old: String = std::fs::read_to_string(path).unwrap();
    std::fs::write(path, format!("{old}line\n")).unwrap();
    Ok(terminal::edit::EditorExit::Success)
}

fn with_ports(dir: &std::path::Path, test: impl Fn(&SessionPorts<'_>, &LogBuffer)) {
    let log: LogBuffer = LogBuffer::new(fixed_clock);
    let git: git::Git = test_git(dir).with_temp_dir(dir);
    let ports: SessionPorts<'_> = SessionPorts {
        log: &log,
        git: &git,
        editor: &append_line,
        handoff: &run_here,
    };
    test(&ports, &log);
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
            assert_eq!(ports.validate_search(""), Err(InputProblem::EmptySearch));
            let entries: Vec<LogEntry> = log.drain();
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].layer, Layer::Validation);
            assert_eq!(entries[0].level, LogLevel::Info);
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
        let renamed: FileDiff = ports
            .file_diff(&status.staged[0].paths(Side::Staged), Side::Staged)
            .unwrap();
        assert!(renamed.header.contains(&"rename from a.txt".to_string()));
        let changed: FileDiff = ports
            .file_diff(&status.unstaged[0].paths(Side::Unstaged), Side::Unstaged)
            .unwrap();
        assert_eq!(changed.hunks[0].lines[0].kind, LineKind::Removed);
        let untracked: FileDiff = ports.untracked_file_diff("new.txt").unwrap();
        assert_eq!(untracked.hunks[0].lines[0].text, "n");
        assert!(log.drain().iter().all(|e: &LogEntry| e.layer == Layer::Git));
    });
}

#[test]
fn git_failures_become_failed() {
    let dir: TempDir = TempDir::new();
    let log: LogBuffer = LogBuffer::new(fixed_clock);
    let git: git::Git = test_git(dir.path())
        .with_program("/nonexistent/git")
        .with_temp_dir(&dir.path().join("missing"));
    let ports: SessionPorts<'_> = SessionPorts {
        log: &log,
        git: &git,
        editor: &append_line,
        handoff: &run_here,
    };
    assert_eq!(ports.status(), Err(Failed));
    let x: Vec<String> = vec!["x".to_string()];
    assert_eq!(ports.file_diff(&x, Side::Unstaged), Err(Failed));
    assert_eq!(ports.untracked_file_diff("x"), Err(Failed));
    assert_eq!(ports.stage_paths(&x), Err(Failed));
    assert_eq!(ports.stage_all(), Err(Failed));
    assert_eq!(ports.unstage_paths(&x), Err(Failed));
    assert_eq!(ports.remove_from_index(&x), Err(Failed));
    assert_eq!(ports.restore_paths(&x), Err(Failed));
    assert_eq!(ports.delete_untracked(&x), Err(Failed));
    assert_eq!(ports.apply_to_index("", false), Err(Failed));
    assert_eq!(ports.has_commits(), Err(Failed));
    assert_eq!(ports.last_commit_has_parent(), Err(Failed));
    assert_eq!(ports.amended_files(AmendBase::ParentCommit), Err(Failed));
    assert_eq!(
        ports.amended_file_diff(&x, AmendBase::ParentCommit),
        Err(Failed)
    );
    assert_eq!(ports.unstage_from_amend(&x), Err(Failed));
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
    assert_eq!(ports.fetch_all_prune(), Err(Failed));
    assert_eq!(ports.reset_keep("h"), Err(Failed));
    assert_eq!(ports.move_branch("b", "h"), Err(Failed));
    assert_eq!(ports.show_commit("h"), Err(Failed));
    assert_eq!(ports.delete_branch("b"), Err(Failed));
    assert_eq!(ports.rebase("b", "main"), Err(Failed));
    assert_eq!(ports.rebase_interactive("b", "main"), Err(Failed));
    assert_eq!(ports.commit_files("h"), Err(Failed));
    assert_eq!(ports.commit_file_diff("h", &x), Err(Failed));
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
        let shown: FileDiff = ports
            .commit_file_diff(&first, &files[0].paths(Side::Staged))
            .unwrap();
        assert_eq!(shown.hunks[0].lines[0].text, "a");
        let renamed: ChangedFile = ports.status().unwrap().staged[0].clone();
        ports.unstage_paths(&renamed.paths(Side::Staged)).unwrap();
        let status: WorkStatus = ports.status().unwrap();
        assert_eq!(status.staged, vec![]);
        assert_eq!(status.unstaged.len(), 2);
        for changed in &status.unstaged {
            ports.stage_paths(&changed.paths(Side::Unstaged)).unwrap();
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
            Ok(EditedMessage {
                text: "First\nline\n".to_string(),
                editor_succeeded: true,
            })
        );
        assert_eq!(
            ports.validate_commit_message(" "),
            Err(InputProblem::EmptyCommitMessage)
        );
        let message: CommitMessage = ports.validate_commit_message("First\n").unwrap();
        ports.commit(&message, false).unwrap();
        assert_eq!(ports.has_commits(), Ok(true));
        assert_eq!(ports.last_commit_message(), Ok("First\n\n".to_string()));
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
        assert_eq!(ports.commit_file_diff("h", &["x".to_string()]), Err(Failed));
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
            Err(InputProblem::BranchName(BranchNameProblem::DoubleDot))
        );
        assert_eq!(ports.validate_checkout_choice("2", 3), Ok(1));
        assert_eq!(
            ports.validate_checkout_choice("9", 3),
            Err(InputProblem::OutOfRange { count: 3 })
        );
        let name: logic::ports::BranchName = ports.validate_branch_name("topic").unwrap();
        let head: String = ports.commits(0, 1).unwrap()[0].hash.clone();
        ports.create_branch(&name, &head).unwrap();
        ports.checkout("topic").unwrap();
        assert_eq!(git_ok(dir.path(), &["branch", "--show-current"]), "topic\n");
        git_ok(dir.path(), &["commit", "-q", "--allow-empty", "-m", "two"]);
        let two: String = git_ok(dir.path(), &["rev-parse", "HEAD"])
            .trim()
            .to_string();
        ports.move_branch("main", &two).unwrap();
        assert_eq!(git_ok(dir.path(), &["rev-parse", "main"]).trim(), two);
        let validation_entries: usize = log
            .drain()
            .iter()
            .filter(|e: &&LogEntry| e.layer == Layer::Validation)
            .count();
        assert_eq!(validation_entries, 2);
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
    let git: git::Git = test_git(dir.path())
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
    assert_eq!(ports.rebase_interactive("topic", "main"), Ok(()));
    assert_eq!(handed.get(), 1);
    assert_eq!(
        git_ok(dir.path(), &["log", "--format=%s", "topic"]),
        "one\nmain work\nbase\n"
    );
    assert_eq!(ports.rebase("main", "topic"), Ok(()));
    let broken =
        |_: &mut dyn FnMut()| -> std::io::Result<()> { Err(std::io::Error::other("no tty")) };
    let failing: SessionPorts<'_> = SessionPorts {
        log: &log,
        git: &git,
        editor: &append_line,
        handoff: &broken,
    };
    assert_eq!(failing.rebase_interactive("topic", "main"), Err(Failed));
    assert!(
        log.drain()
            .iter()
            .any(|e: &LogEntry| e.layer == Layer::Wiring
                && e.message == "handing the terminal to git failed: no tty")
    );
}
