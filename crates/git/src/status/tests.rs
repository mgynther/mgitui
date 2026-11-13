use super::{ChangeKind, FileChange, Status, parse_status};
use crate::runner::GitError;
use crate::test_repo::{Logged, TestRepo};

fn change(path: &str, kind: ChangeKind) -> FileChange {
    FileChange {
        path: path.to_string(),
        original_path: None,
        kind,
        mode_changed: false,
    }
}

#[test]
fn parses_every_record_type() {
    let output: &str = concat!(
        "1 M. N... 100644 100644 100644 aaa bbb staged.txt\0",
        "1 .M N... 100644 100644 100755 aaa aaa mode and space.txt\0",
        "1 A. N... 000000 100644 100644 000 bbb added.txt\0",
        "1 .D N... 100644 100644 000000 aaa aaa gone.txt\0",
        "1 T. N... 100644 120000 120000 aaa bbb link\0",
        "2 R. N... 100644 100644 100644 aaa aaa R100 new.txt\0old.txt\0",
        "2 C. N... 100644 100644 100644 aaa aaa C100 copy.txt\0orig.txt\0",
        "u UU N... 100644 100644 100644 100644 a b c conflict.txt\0",
        "? untracked dir/new file\0",
        "! ignored\0",
    );
    let status: Status = parse_status(output).unwrap();
    assert_eq!(
        status.staged,
        vec![
            change("staged.txt", ChangeKind::Modified),
            change("added.txt", ChangeKind::Added),
            FileChange {
                mode_changed: true,
                ..change("link", ChangeKind::TypeChanged)
            },
            FileChange {
                original_path: Some("old.txt".to_string()),
                ..change("new.txt", ChangeKind::Renamed)
            },
            FileChange {
                original_path: Some("orig.txt".to_string()),
                ..change("copy.txt", ChangeKind::Copied)
            },
        ]
    );
    assert_eq!(
        status.unstaged,
        vec![
            FileChange {
                mode_changed: true,
                ..change("mode and space.txt", ChangeKind::Modified)
            },
            change("gone.txt", ChangeKind::Deleted),
            change("conflict.txt", ChangeKind::Conflict),
            change("untracked dir/new file", ChangeKind::Untracked),
        ]
    );
}

#[test]
fn rejects_unexpected_records() {
    for output in [
        "x what\0",
        "1 M.\0",
        "1 MMM N... 1 1 1 a b p\0",
        "1 X. N... 100644 100644 100644 a b p\0",
        "1 .X N... 100644 100644 100644 a b p\0",
        "2 X. N... 100644 100644 100644 a a R100 new\0old\0",
        "2 R. N... 100644 100644 100644 a a R100 new\0",
        "2 R. N...\0",
        "u UU N...\0",
        "1\0",
    ] {
        assert!(
            matches!(parse_status(output), Err(GitError::Parse(_))),
            "{output:?}"
        );
    }
}

#[test]
fn status_of_real_repository() {
    let repo: TestRepo = TestRepo::new();
    repo.write("a.txt", "a\n");
    repo.write("b.txt", "b\n");
    repo.git_ok(&["add", "."]);
    repo.git_ok(&["commit", "-q", "-m", "init"]);
    repo.write("a.txt", "changed\n");
    repo.write("c.txt", "new\n");
    repo.git_ok(&["rm", "-q", "b.txt"]);
    let logged: Logged = Logged::new();
    let status: Status = repo.git().status(&logged.log()).unwrap();
    assert_eq!(status.staged, vec![change("b.txt", ChangeKind::Deleted)]);
    assert_eq!(
        status.unstaged,
        vec![
            change("a.txt", ChangeKind::Modified),
            change("c.txt", ChangeKind::Untracked),
        ]
    );
}

#[test]
fn status_failure() {
    let repo: TestRepo = TestRepo::new();
    std::fs::remove_dir_all(repo.path().join(".git")).unwrap();
    let logged: Logged = Logged::new();
    let git: crate::runner::Git = repo.git().with_env("GIT_CEILING_DIRECTORIES", "/");
    assert!(matches!(
        git.status(&logged.log()),
        Err(GitError::Failed { .. })
    ));
}
