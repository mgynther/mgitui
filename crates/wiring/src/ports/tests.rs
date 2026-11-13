use super::{SessionPorts, diff_paths};
use crate::log::LogBuffer;
use crate::test_support::{TempDir, fixed_clock, git_ok, init_repo};
use logic::log::{Layer, LogEntry, LogLevel};
use logic::model::{ChangeKind, ChangedFile, FileDiff, LineKind, Side, WorkStatus};
use logic::ports::{Failed, Ports, SearchText};

fn with_ports(dir: &std::path::Path, test: impl Fn(&SessionPorts<'_>, &LogBuffer)) {
    let log: LogBuffer = LogBuffer::new(fixed_clock);
    let git: git::Git = git::Git::new(dir);
    let ports: SessionPorts<'_> = SessionPorts {
        log: &log,
        git: &git,
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
    let git: git::Git = git::Git::new(dir.path()).with_program("/nonexistent/git");
    let ports: SessionPorts<'_> = SessionPorts {
        log: &log,
        git: &git,
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
