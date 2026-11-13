use super::{Commit, Ref, RefKind, parse_log, parse_refs};
use crate::runner::GitError;
use crate::test_repo::{Logged, TestRepo};

#[test]
fn parses_log_records() {
    let output: &str = "a\u{1f}b c\u{1f}Ada\u{1f}2025-01-02 03:04\u{1f}Merge x\u{1f}y\0\nb\u{1f}\u{1f}Bo\u{1f}2025-01-01 00:00\u{1f}\0";
    assert_eq!(
        parse_log(output).unwrap(),
        vec![
            Commit {
                hash: "a".to_string(),
                parents: vec!["b".to_string(), "c".to_string()],
                author: "Ada".to_string(),
                date: "2025-01-02 03:04".to_string(),
                subject: "Merge x\u{1f}y".to_string(),
            },
            Commit {
                hash: "b".to_string(),
                parents: vec![],
                author: "Bo".to_string(),
                date: "2025-01-01 00:00".to_string(),
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
