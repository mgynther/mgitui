use std::ffi::OsStr;

use super::{Commit, Ref, RefKind, parse_log, parse_name_status, parse_refs, with_less_default};
use crate::command::Command;
use crate::runner::{Git, GitError, Output};
use crate::status::{ChangeKind, FileChange};
use crate::test_repo::{Logged, TempDir, TestRepo, fake_git};

#[test]
fn parses_log_records() {
    let output: &str = "a\u{1f}b c\u{1f}Ada\u{1f}ada@a\u{1f}2025-01-02 03:04\u{1f}Cy\u{1f}cy@c\u{1f}2025-01-03 04:05\u{1f}Merge x\u{1f}y\0\nb\u{1f}\u{1f}Bo\u{1f}\u{1f}2025-01-01 00:00\u{1f}Bo\u{1f}\u{1f}2025-01-01 00:00\u{1f}\0";
    assert_eq!(
        parse_log(output).unwrap(),
        vec![
            Commit {
                hash: "a".to_string(),
                parents: vec!["b".to_string(), "c".to_string()],
                author: "Ada".to_string(),
                author_email: "ada@a".to_string(),
                date: "2025-01-02 03:04".to_string(),
                committer: "Cy".to_string(),
                committer_email: "cy@c".to_string(),
                committer_date: "2025-01-03 04:05".to_string(),
                subject: "Merge x\u{1f}y".to_string(),
            },
            Commit {
                hash: "b".to_string(),
                parents: vec![],
                author: "Bo".to_string(),
                author_email: String::new(),
                date: "2025-01-01 00:00".to_string(),
                committer: "Bo".to_string(),
                committer_email: String::new(),
                committer_date: "2025-01-01 00:00".to_string(),
                subject: String::new(),
            },
        ]
    );
    assert!(matches!(parse_log("a\u{1f}b"), Err(GitError::Parse(_))));
    assert_eq!(parse_log(""), Ok(vec![]));
}

#[test]
fn parses_refs() {
    let output: &str = "*\0h1\0\0refs/heads/main\n \0h2\0\0refs/remotes/origin/feature/x\n \0h3\0\0refs/remotes/origin/HEAD\n \0t1\0h1\0refs/tags/v1\n \0h4\0\0refs/tags/light\n \0h5\0\0refs/other\n \0h6\0\0refs/remotes/bad\n";
    assert_eq!(
        parse_refs(output).unwrap(),
        vec![
            Ref {
                target: "h1".to_string(),
                kind: RefKind::Local("main".to_string()),
                is_head: true,
            },
            Ref {
                target: "h2".to_string(),
                kind: RefKind::Remote {
                    remote: "origin".to_string(),
                    branch: "feature/x".to_string(),
                },
                is_head: false,
            },
            Ref {
                target: "h1".to_string(),
                kind: RefKind::Tag("v1".to_string()),
                is_head: false,
            },
            Ref {
                target: "h4".to_string(),
                kind: RefKind::Tag("light".to_string()),
                is_head: false,
            },
        ]
    );
    assert!(matches!(parse_refs("x\0y\n"), Err(GitError::Parse(_))));
}

#[test]
fn history_of_real_repository() {
    let repo: TestRepo = TestRepo::new();
    let logged: Logged = Logged::new();
    assert_eq!(repo.git().log(0, 10, &logged.log()), Ok(vec![]));
    assert_eq!(repo.git().refs(&logged.log()), Ok(vec![]));
    assert_eq!(repo.git().head(&logged.log()), Ok(None));
    repo.git_ok(&["commit", "-q", "--allow-empty", "-m", "first"]);
    repo.git_ok(&["commit", "-q", "--allow-empty", "-m", "second\n\nbody"]);
    repo.git_ok(&["tag", "-a", "-m", "annotated", "v1", "HEAD~1"]);
    let commits: Vec<Commit> = repo.git().log(0, 10, &logged.log()).unwrap();
    assert_eq!(commits.len(), 2);
    assert_eq!(commits[0].subject, "second");
    assert_eq!(commits[0].parents, vec![commits[1].hash.clone()]);
    assert_eq!(commits[0].author, "Ada Author");
    assert_eq!(commits[0].date, "2025-01-02 03:04");
    assert_eq!(commits[0].committer, "Cy Committer");
    assert_eq!(commits[0].author_email, "ada@example.com");
    assert_eq!(commits[0].committer_email, "cy@example.com");
    assert_eq!(commits[0].committer_date, "2025-01-02 03:04");
    assert_eq!(
        repo.git().log(1, 10, &logged.log()).unwrap(),
        vec![commits[1].clone()]
    );
    let refs: Vec<Ref> = repo.git().refs(&logged.log()).unwrap();
    assert_eq!(refs[0].kind, RefKind::Local("main".to_string()));
    assert!(refs[0].is_head);
    assert_eq!(refs[1].target, commits[1].hash);
    assert_eq!(
        repo.git().head(&logged.log()),
        Ok(Some(commits[0].hash.clone()))
    );
}

#[test]
fn history_failures() {
    let repo: TestRepo = TestRepo::new();
    let logged: Logged = Logged::new();
    let git: crate::runner::Git = repo.git().with_program("/nonexistent/git");
    assert!(git.log(0, 1, &logged.log()).is_err());
    assert!(git.refs(&logged.log()).is_err());
    assert!(git.head(&logged.log()).is_err());
}

#[test]
fn parses_name_status() {
    let change = |path: &str, original: Option<&str>, kind: ChangeKind| FileChange {
        path: path.to_string(),
        original_path: original.map(str::to_string),
        kind,
        mode_changed: false,
    };
    assert_eq!(
        parse_name_status("M\0a\0A\0b\0D\0c\0R100\0old\0new\0C75\0x\0y\0T\0l\0").unwrap(),
        vec![
            change("a", None, ChangeKind::Modified),
            change("b", None, ChangeKind::Added),
            change("c", None, ChangeKind::Deleted),
            change("new", Some("old"), ChangeKind::Renamed),
            change("y", Some("x"), ChangeKind::Copied),
            change("l", None, ChangeKind::TypeChanged),
        ]
    );
    for bad in ["X\0a\0", "M\0", "R100\0old\0"] {
        assert!(
            matches!(parse_name_status(bad), Err(GitError::Parse(_))),
            "{bad:?}"
        );
    }
}

#[test]
fn commit_details_of_real_repository() {
    let repo: TestRepo = TestRepo::new();
    repo.write("a.txt", "a\n");
    repo.git_ok(&["add", "."]);
    repo.git_ok(&["commit", "-q", "-m", "first\n\nbody"]);
    repo.git_ok(&["mv", "a.txt", "b.txt"]);
    repo.git_ok(&["commit", "-q", "-m", "rename"]);
    let logged: Logged = Logged::new();
    let head: String = repo.git().head(&logged.log()).unwrap().unwrap();
    let first: String = repo.git_ok(&["rev-parse", "HEAD~1"]).trim().to_string();
    assert_eq!(
        repo.git().commit_message(&first, &logged.log()).unwrap(),
        "first\n\nbody\n\n"
    );
    let files: Vec<FileChange> = repo.git().commit_files(&head, &logged.log()).unwrap();
    assert_eq!(files[0].kind, ChangeKind::Renamed);
    let diff = repo
        .git()
        .commit_file_diff(&first, &["a.txt".to_string()], &logged.log())
        .unwrap();
    assert_eq!(diff.hunks[0].lines[0].text, "a");
    let git: crate::runner::Git = repo.git().with_program("/nonexistent/git");
    assert!(git.commit_message(&head, &logged.log()).is_err());
    assert!(git.commit_files(&head, &logged.log()).is_err());
    assert!(git.commit_file_diff(&head, &[], &logged.log()).is_err());
}

#[test]
fn show_commit_runs_git_show() {
    let repo: TestRepo = TestRepo::new();
    repo.git_ok(&["commit", "-q", "--allow-empty", "-m", "shown"]);
    let hash: String = repo.git_ok(&["rev-parse", "HEAD"]).trim().to_string();
    let logged: Logged = Logged::new();
    repo.git().show_commit(&hash, &logged.log()).unwrap();
    assert_eq!(
        logged.lines(),
        vec![format!("Info: git show {hash} (exit 0)")]
    );
    assert!(matches!(
        repo.git().show_commit("0000000", &logged.log()),
        Err(GitError::Failed { .. })
    ));
}

#[test]
fn less_keeps_the_user_setting_or_stays_open() {
    let dir: TempDir = TempDir::new();
    let program: String = fake_git(&dir, "printf %s \"$LESS\"");
    // A fixed value stands in for whatever LESS the environment running the tests has.
    let git: Git = Git::new(dir.path())
        .with_program(&program)
        .with_env("LESS", "user");
    let logged: Logged = Logged::new();
    let less = |user: Option<&OsStr>| -> String {
        let output: Output = with_less_default(git.clone(), user)
            .run(&Command::Status, None, &logged.log())
            .unwrap();
        output.stdout
    };
    assert_eq!(less(None), "R");
    assert_eq!(less(Some(OsStr::new("user"))), "user");
}
