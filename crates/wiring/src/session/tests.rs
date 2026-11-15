use super::Session;
use crate::log::LogBuffer;
use crate::test_support::{TempDir, fixed_clock, git_ok, init_repo, test_git};
use ui::layout::Size;

const SIZE: Size = Size {
    width: 80,
    height: 24,
};

#[expect(
    clippy::unnecessary_wraps,
    reason = "it has the signature of a terminal handoff"
)]
fn run_here(run: &mut dyn FnMut()) -> std::io::Result<()> {
    run();
    Ok(())
}

fn session_in(dir: &TempDir) -> Session {
    let log: LogBuffer = LogBuffer::new(fixed_clock);
    log.wiring(logic::log::LogLevel::Info, "started");
    let git: git::Git = test_git(dir.path()).with_temp_dir(dir.path());
    let editor = |path: &std::path::Path| -> std::io::Result<()> {
        let old: String = std::fs::read_to_string(path).unwrap();
        let new: String = if old.is_empty() {
            "New commit\n".to_string()
        } else {
            old.replace("New", "Amended")
        };
        std::fs::write(path, new)
    };
    let mut session: Session = Session::new(git, log, Box::new(editor), Box::new(run_here));
    session.resize(SIZE);
    session
}

/// A session in a new empty repository. Keep the returned directory alive: dropping it removes
/// the repository.
fn session() -> (TempDir, Session) {
    let dir: TempDir = TempDir::new();
    init_repo(dir.path());
    let session: Session = session_in(&dir);
    (dir, session)
}

#[test]
fn shows_tabs_and_quits_after_confirmation() {
    let (_dir, mut s): (TempDir, Session) = session();
    assert!(s.render(SIZE).row_text(0).starts_with(" F1 Branches "));
    s.input(b"q");
    assert_eq!(s.render(SIZE).row_text(23).trim_end(), "Quit? y/n");
    s.input(b"n");
    assert!(!s.should_quit());
    s.input(b"qy");
    assert!(s.should_quit());
}

#[test]
fn function_keys_switch_views_and_unknown_keys_are_logged() {
    let (_dir, mut s): (TempDir, Session) = session();
    s.input(b"\x1bOR\x1b[Z");
    let screen: ui::buffer::Buffer = s.render(SIZE);
    assert!(screen.row_text(1).starts_with("─ Log ─"));
    s.input(b"x\x1bOQ");
    assert!(s.render(SIZE).row_text(1).starts_with("─ Unstaged ─"));
}

#[test]
fn diff_view_shows_changes_and_reloads() {
    let (dir, mut s): (TempDir, Session) = session();
    s.input(b"\x1bOQ");
    assert!(s.render(SIZE).row_text(2).contains("No changes"));
    std::fs::write(dir.path().join("new.txt"), "hello\n").unwrap();
    s.input(b"r");
    let screen: ui::buffer::Buffer = s.render(SIZE);
    assert!(screen.row_text(1).contains("Diff new.txt"));
    assert!(screen.row_text(2).starts_with("? new.txt"));
    assert!(screen.row_text(3).contains("+hello"));
    assert_eq!(screen.cell(15, 0).unwrap().style.fg, ui::style::Color::Red);
    assert!(screen.cell(0, 2).unwrap().style.reverse);
    s.input(b"s");
    let staged: String = s.render(SIZE).row_text(1);
    assert!(staged.contains(" Diff ") && !staged.contains("new.txt"));
    s.input(b"sw");
    assert!(s.render(SIZE).row_text(2).contains("▶@@ -0,0 +1 @@"));
}

#[test]
fn log_view_search_with_validation() {
    let (_dir, mut s): (TempDir, Session) = session();
    s.input(b"\x1bOR/");
    assert_eq!(s.render(SIZE).row_text(23).trim_end(), "/");
    s.input(b"\r");
    let screen: ui::buffer::Buffer = s.render(SIZE);
    assert_eq!(screen.row_text(23).trim_end(), "search text is empty");
    let lines: Vec<String> = (0..24).map(|y: usize| screen.row_text(y)).collect();
    let position = |text: &str| lines.iter().position(|l: &String| l.contains(text));
    assert_eq!(position("wiring     Info  started"), Some(2));
    assert_eq!(position("logic      Info  reload"), Some(3));
    assert!(position("git        Info  git log --all") > Some(3));
    assert!(
        position("validation Error search text is empty") > position("git        Info  git status")
    );
}

/// A repository with `f.txt` containing lines 1 to 10, changed so that line 2 is `two` and
/// line 9 is `nine`: two hunks in the unstaged diff.
fn two_hunk_session() -> (TempDir, Session) {
    let dir: TempDir = TempDir::new();
    init_repo(dir.path());
    let numbers: &str = "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n";
    std::fs::write(dir.path().join("f.txt"), numbers).unwrap();
    git_ok(dir.path(), &["add", "."]);
    git_ok(dir.path(), &["commit", "-q", "-m", "init"]);
    let changed: String = numbers.replace("2\n", "two\n").replace("9\n", "nine\n");
    std::fs::write(dir.path().join("f.txt"), changed).unwrap();
    let mut session: Session = session_in(&dir);
    session.input(b"\x1bOQ");
    (dir, session)
}

fn index_content(dir: &TempDir) -> String {
    git_ok(dir.path(), &["show", ":f.txt"])
}

#[test]
fn stage_second_hunk_then_unstage_one_line_of_it() {
    let (dir, mut s): (TempDir, Session) = two_hunk_session();
    s.input(b"wGk k");
    let screen: ui::buffer::Buffer = s.render(SIZE);
    assert!(
        screen.row_text(12).contains("▶-9"),
        "{}",
        screen.row_text(12)
    );
    assert!(
        screen.row_text(13).contains("┃+nine"),
        "{}",
        screen.row_text(13)
    );
    s.input(b"\r");
    assert_eq!(index_content(&dir), "1\n2\n3\n4\n5\n6\n7\n8\nnine\n10\n");
    s.input(b"wsw");
    s.input(b"jjjjj\r");
    assert_eq!(index_content(&dir), "1\n2\n3\n4\n5\n6\n7\n8\n10\n");
}

#[test]
fn stage_lines_of_first_hunk_and_whole_file() {
    let (dir, mut s): (TempDir, Session) = two_hunk_session();
    s.input(b"wjj jj\r");
    assert_eq!(index_content(&dir), "1\ntwo\n3\n4\n5\n6\n7\n8\n9\n10\n");
    s.input(b"w\r");
    assert_eq!(index_content(&dir), "1\ntwo\n3\n4\n5\n6\n7\n8\nnine\n10\n");
    let screen: ui::buffer::Buffer = s.render(SIZE);
    assert!(
        screen.row_text(13).starts_with("M f.txt"),
        "{}",
        screen.row_text(13)
    );
    s.input(b"s\r");
    assert_eq!(index_content(&dir), "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n");
}

#[test]
fn edit_message_commit_and_amend() {
    let (dir, mut s): (TempDir, Session) = two_hunk_session();
    s.input(b"c");
    assert_eq!(
        s.render(SIZE).row_text(23).trim_end(),
        "commit message is empty"
    );
    s.input(b"e");
    assert!(s.render(SIZE).row_text(16).contains("│New commit"));
    s.input(b"\rc");
    assert_eq!(
        git_ok(dir.path(), &["log", "--format=%s"]),
        "New commit\ninit\n"
    );
    assert!(!s.render(SIZE).row_text(16).contains("New commit"));
    s.input(b"a");
    let screen: ui::buffer::Buffer = s.render(SIZE);
    assert!(
        screen.row_text(15).contains("Amend message"),
        "{}",
        screen.row_text(15)
    );
    assert!(screen.row_text(16).contains("│New commit"));
    s.input(b"ec");
    assert_eq!(
        git_ok(dir.path(), &["log", "--format=%s"]),
        "Amended commit\ninit\n"
    );
    let after: ui::buffer::Buffer = s.render(SIZE);
    assert!(after.row_text(15).contains("Commit message"));
    assert!(!after.row_text(16).contains("commit"));
}

#[test]
fn branches_view_shows_graph_details_and_panes() {
    let dir: TempDir = TempDir::new();
    init_repo(dir.path());
    std::fs::write(dir.path().join("a.txt"), "a\n").unwrap();
    git_ok(dir.path(), &["add", "."]);
    git_ok(dir.path(), &["commit", "-q", "-m", "first\n\nbody text"]);
    git_ok(dir.path(), &["tag", "v1"]);
    git_ok(dir.path(), &["checkout", "-q", "-b", "feature"]);
    std::fs::write(dir.path().join("b.txt"), "b\n").unwrap();
    git_ok(dir.path(), &["add", "."]);
    git_ok(dir.path(), &["commit", "-q", "-m", "second"]);
    let log: LogBuffer = LogBuffer::new(fixed_clock);
    let git: git::Git = test_git(dir.path());
    let mut s: Session = Session::new(
        git,
        log,
        Box::new(|_: &std::path::Path| Ok(())),
        Box::new(run_here),
    );
    s.resize(SIZE);
    let first_selected: ui::buffer::Buffer = s.render(SIZE);
    assert!(
        first_selected
            .row_text(2)
            .starts_with("◉  [feature] second"),
        "{}",
        first_selected.row_text(2)
    );
    assert!(
        first_selected
            .row_text(2)
            .ends_with("Ada Author         2025-01-02 03:04")
    );
    assert!(
        first_selected
            .row_text(3)
            .starts_with("●  [main] <v1> first"),
        "{}",
        first_selected.row_text(3)
    );
    assert!(
        first_selected.row_text(9).starts_with("commit "),
        "{}",
        first_selected.row_text(9)
    );
    assert!(first_selected.row_text(9).contains("│A b.txt"));
    s.input(b"j");
    let second_selected: ui::buffer::Buffer = s.render(SIZE);
    // The box is 53 columns wide at 80x24: the dates stay visible and the emails are cut off.
    let in_box =
        |row: usize| -> String { second_selected.row_text(row).chars().take(53).collect() };
    assert_eq!(
        in_box(10),
        "Author:    2025-01-02 03:04  Ada Author <ada@example."
    );
    assert_eq!(
        in_box(11),
        "Committer: 2025-01-02 03:04  Cy Committer <cy@example"
    );
    assert!(
        second_selected.row_text(12).starts_with("first"),
        "{}",
        second_selected.row_text(12)
    );
    assert_eq!(
        second_selected.cell(7, 9).unwrap().style.fg,
        ui::style::Color::Blue,
        "the hash has the color of the commit's lane"
    );
    assert!(second_selected.cell(7, 9).unwrap().style.bold);
    assert!(second_selected.row_text(9).contains("│A a.txt"));
    s.input(b"w");
    assert!(s.render(SIZE).cell(55, 9).unwrap().style.reverse);
    s.input(b"w");
    let diff_focused: ui::buffer::Buffer = s.render(SIZE);
    assert!(
        diff_focused.row_text(15).contains("Diff a.txt"),
        "{}",
        diff_focused.row_text(15)
    );
    assert!(
        diff_focused.row_text(16).starts_with("▶@@"),
        "{}",
        diff_focused.row_text(16)
    );
    s.input(b"w/sec\r");
    assert!(s.render(SIZE).row_text(9).contains("│A b.txt"));
}

#[test]
fn create_and_check_out_branches() {
    let dir: TempDir = TempDir::new();
    init_repo(dir.path());
    git_ok(dir.path(), &["commit", "-q", "--allow-empty", "-m", "one"]);
    git_ok(dir.path(), &["commit", "-q", "--allow-empty", "-m", "two"]);
    let mut s: Session = session_in(&dir);
    s.input(b"jc");
    assert_eq!(s.render(SIZE).row_text(23).trim_end(), "New branch name:");
    s.input(b"bad name\r");
    assert!(
        s.render(SIZE)
            .row_text(23)
            .starts_with("invalid branch name: it contains")
    );
    s.input(b"cold\r");
    assert_eq!(s.render(SIZE).row_text(23).trim_end(), "Created branch old");
    assert!(s.render(SIZE).row_text(3).contains("[old] one"));
    s.input(b"h");
    assert_eq!(git_ok(dir.path(), &["branch", "--show-current"]), "old\n");
    s.input(b"kcnew\rh");
    assert!(
        s.render(SIZE)
            .row_text(23)
            .starts_with("Check out (1 main, 2 new):")
    );
    s.input(b"7\r");
    assert_eq!(
        s.render(SIZE).row_text(23).trim_end(),
        "Check out (1 main, 2 new): 7   choice must be a number from 1 to 2"
    );
    s.input(b"x");
    let prompt: String = s.render(SIZE).row_text(23);
    assert_eq!(
        prompt.trim_end(),
        "Check out (1 main, 2 new): 7x",
        "{prompt}"
    );
    s.input(b"\x7f\x7f2\r");
    assert_eq!(git_ok(dir.path(), &["branch", "--show-current"]), "new\n");
    s.input(b"jjh");
    assert_eq!(s.render(SIZE).row_text(23).trim_end(), "Checked out old");
    assert_eq!(git_ok(dir.path(), &["branch", "--show-current"]), "old\n");
}

#[test]
fn empty_repository_and_small_terminal() {
    let (_dir, mut s): (TempDir, Session) = session();
    let empty: ui::buffer::Buffer = s.render(SIZE);
    assert!(empty.row_text(1).starts_with("─ Commits ─"));
    assert_eq!(empty.row_text(2).trim_end(), "");
    s.input(b"jGgwjhc\x1b");
    s.input(b"\x1bOQ");
    assert!(s.render(SIZE).row_text(2).contains("No changes"));
    s.input(b"\re a");
    let amend_refused: ui::buffer::Buffer = s.render(SIZE);
    assert_eq!(
        amend_refused.row_text(23).trim_end(),
        "Nothing to amend: there are no commits yet"
    );
    assert_eq!(
        amend_refused.cell(25, 0).unwrap().style.fg,
        ui::style::Color::Red
    );
    s.input(b"\x1bOR");
    let log: ui::buffer::Buffer = s.render(SIZE);
    let lines: Vec<String> = (2..23).map(|y: usize| log.row_text(y)).collect();
    assert!(
        lines
            .iter()
            .any(|l: &String| l.contains("logic      Warn  Nothing to amend"))
    );
    s.input(b"\x1bOP");
    assert_eq!(
        s.render(SIZE).cell(25, 0).unwrap().style.fg,
        ui::style::Color::Default
    );
    let small: Size = Size {
        width: 60,
        height: 20,
    };
    s.resize(small);
    assert_eq!(
        s.render(small).row_text(0).trim_end(),
        "terminal too small (needs 80x24)"
    );
    s.resize(SIZE);
    assert!(s.render(SIZE).row_text(0).starts_with(" F1 Branches "));
}

#[test]
fn cherry_pick_commit_under_cursor() {
    let dir: TempDir = TempDir::new();
    init_repo(dir.path());
    git_ok(dir.path(), &["commit", "-q", "--allow-empty", "-m", "base"]);
    git_ok(dir.path(), &["switch", "-q", "-c", "side"]);
    std::fs::write(dir.path().join("s.txt"), "s\n").unwrap();
    git_ok(dir.path(), &["add", "."]);
    git_ok(dir.path(), &["commit", "-q", "-m", "side work"]);
    git_ok(dir.path(), &["switch", "-q", "main"]);
    let mut s: Session = session_in(&dir);
    s.input(b"/side work\rp");
    assert!(s.render(SIZE).row_text(23).starts_with("Cherry-picked "));
    assert_eq!(
        git_ok(dir.path(), &["log", "-1", "--format=%s"]),
        "side work\n"
    );
    s.input(b"p");
    assert_eq!(
        s.render(SIZE).row_text(23).trim_end(),
        "Cherry-pick failed, see the Log view"
    );
}

#[test]
fn rebase_branch_under_cursor() {
    let dir: TempDir = TempDir::new();
    init_repo(dir.path());
    git_ok(dir.path(), &["commit", "-q", "--allow-empty", "-m", "base"]);
    git_ok(dir.path(), &["switch", "-q", "-c", "topic"]);
    git_ok(
        dir.path(),
        &["commit", "-q", "--allow-empty", "-m", "topic work"],
    );
    git_ok(dir.path(), &["branch", "also-topic"]);
    git_ok(dir.path(), &["switch", "-q", "main"]);
    git_ok(
        dir.path(),
        &["commit", "-q", "--allow-empty", "-m", "main work"],
    );
    let mut s: Session = session_in(&dir);
    s.input(b"/topic work\rb");
    assert_eq!(
        s.render(SIZE).row_text(23).trim_end(),
        "Rebase which branch (1 also-topic, 2 topic):"
    );
    s.input(b"2\r");
    assert_eq!(
        s.render(SIZE).row_text(23).trim_end(),
        "Rebase topic onto (1 main, 2 also-topic):"
    );
    s.input(b"1\r");
    assert_eq!(
        s.render(SIZE).row_text(23).trim_end(),
        "Rebased topic onto main"
    );
    assert_eq!(
        git_ok(dir.path(), &["log", "--format=%s", "topic"]),
        "topic work\nmain work\nbase\n"
    );
}

#[test]
fn interactive_rebase_prompt() {
    let dir: TempDir = TempDir::new();
    init_repo(dir.path());
    git_ok(dir.path(), &["commit", "-q", "--allow-empty", "-m", "base"]);
    git_ok(dir.path(), &["switch", "-q", "-c", "topic"]);
    git_ok(
        dir.path(),
        &["commit", "-q", "--allow-empty", "-m", "topic work"],
    );
    git_ok(dir.path(), &["switch", "-q", "main"]);
    git_ok(
        dir.path(),
        &["commit", "-q", "--allow-empty", "-m", "main work"],
    );
    let log: LogBuffer = LogBuffer::new(fixed_clock);
    let git: git::Git = test_git(dir.path()).with_env("GIT_SEQUENCE_EDITOR", "true");
    let mut s: Session = Session::new(
        git,
        log,
        Box::new(|_: &std::path::Path| Ok(())),
        Box::new(run_here),
    );
    s.resize(SIZE);
    s.input(b"/topic work\ri");
    assert_eq!(
        s.render(SIZE).row_text(23).trim_end(),
        "Interactively rebase topic onto (1 main):"
    );
    s.input(b"1\r");
    assert_eq!(
        s.render(SIZE).row_text(23).trim_end(),
        "Interactively rebased topic onto main"
    );
    assert_eq!(
        git_ok(dir.path(), &["log", "--format=%s", "topic"]),
        "topic work\nmain work\nbase\n"
    );
}

#[test]
fn rebase_onto_remote_main() {
    let dir: TempDir = TempDir::new();
    init_repo(dir.path());
    git_ok(dir.path(), &["commit", "-q", "--allow-empty", "-m", "base"]);
    git_ok(dir.path(), &["switch", "-q", "-c", "topic"]);
    git_ok(
        dir.path(),
        &["commit", "-q", "--allow-empty", "-m", "topic work"],
    );
    git_ok(dir.path(), &["switch", "-q", "main"]);
    git_ok(
        dir.path(),
        &["commit", "-q", "--allow-empty", "-m", "remote work"],
    );
    git_ok(
        dir.path(),
        &["update-ref", "refs/remotes/origin/main", "HEAD"],
    );
    git_ok(
        dir.path(),
        &["update-ref", "refs/remotes/origin/stale", "HEAD"],
    );
    git_ok(dir.path(), &["reset", "-q", "--hard", "HEAD~1"]);
    let mut s: Session = session_in(&dir);
    s.input(b"/topic work\rb");
    assert_eq!(
        s.render(SIZE).row_text(23).trim_end(),
        "Rebase topic onto (1 main, 2 origin/main):"
    );
    s.input(b"2\r");
    assert_eq!(
        s.render(SIZE).row_text(23).trim_end(),
        "Rebased topic onto origin/main"
    );
    assert_eq!(
        git_ok(dir.path(), &["log", "--format=%s", "topic"]),
        "topic work\nremote work\nbase\n"
    );
}

#[test]
fn delete_branches_that_are_not_checked_out() {
    let dir: TempDir = TempDir::new();
    init_repo(dir.path());
    git_ok(dir.path(), &["commit", "-q", "--allow-empty", "-m", "base"]);
    git_ok(dir.path(), &["branch", "done"]);
    git_ok(dir.path(), &["branch", "kept"]);
    git_ok(dir.path(), &["switch", "-q", "-c", "squashed"]);
    git_ok(dir.path(), &["commit", "-q", "--allow-empty", "-m", "work"]);
    git_ok(dir.path(), &["switch", "-q", "main"]);
    let mut s: Session = session_in(&dir);
    s.input(b"/base\rd");
    assert_eq!(
        s.render(SIZE).row_text(23).trim_end(),
        "Delete which branch (1 done, 2 kept):"
    );
    s.input(b"1\r");
    assert_eq!(
        s.render(SIZE).row_text(23).trim_end(),
        "Delete branch done even if it is not merged? y/n"
    );
    s.input(b"y");
    assert_eq!(
        s.render(SIZE).row_text(23).trim_end(),
        "Deleted branch done"
    );
    s.input(b"dn");
    assert_eq!(s.render(SIZE).row_text(23).trim_end(), "");
    s.input(b"dy");
    assert_eq!(
        s.render(SIZE).row_text(23).trim_end(),
        "Deleted branch kept"
    );
    s.input(b"d");
    assert_eq!(
        s.render(SIZE).row_text(23).trim_end(),
        "The checked out branch cannot be deleted"
    );
    s.input(b"/work\rdy");
    assert_eq!(
        s.render(SIZE).row_text(23).trim_end(),
        "Deleted branch squashed"
    );
    assert_eq!(
        git_ok(dir.path(), &["branch", "--format=%(refname:short)"]),
        "main\n"
    );
}

#[test]
fn remote_update_shows_fetched_branches() {
    let upstream: TempDir = TempDir::new();
    init_repo(upstream.path());
    git_ok(
        upstream.path(),
        &["commit", "-q", "--allow-empty", "-m", "upstream work"],
    );
    let dir: TempDir = TempDir::new();
    init_repo(dir.path());
    let url: String = upstream.path().to_string_lossy().into_owned();
    git_ok(dir.path(), &["remote", "add", "origin", &url]);
    let mut s: Session = session_in(&dir);
    s.input(b"u");
    let screen: ui::buffer::Buffer = s.render(SIZE);
    assert_eq!(screen.row_text(23).trim_end(), "Updated remotes");
    assert!(
        screen
            .row_text(2)
            .starts_with("●  [origin/main] upstream work"),
        "{}",
        screen.row_text(2)
    );
}

#[test]
fn fetch_all_prune_removes_stale_remote_branches() {
    let upstream: TempDir = TempDir::new();
    init_repo(upstream.path());
    git_ok(
        upstream.path(),
        &["commit", "-q", "--allow-empty", "-m", "upstream work"],
    );
    git_ok(upstream.path(), &["branch", "stale"]);
    let dir: TempDir = TempDir::new();
    init_repo(dir.path());
    let url: String = upstream.path().to_string_lossy().into_owned();
    git_ok(dir.path(), &["remote", "add", "origin", &url]);
    let mut s: Session = session_in(&dir);
    s.input(b"u");
    git_ok(upstream.path(), &["branch", "-D", "stale"]);
    s.input(b"u");
    assert_eq!(
        git_ok(dir.path(), &["branch", "-r"]),
        "  origin/main\n  origin/stale\n"
    );
    s.input(b"U");
    let screen: ui::buffer::Buffer = s.render(SIZE);
    assert_eq!(
        screen.row_text(23).trim_end(),
        "Fetched all remotes pruning stale remote branches"
    );
    assert_eq!(git_ok(dir.path(), &["branch", "-r"]), "  origin/main\n");
}

#[test]
fn s_shows_the_selected_commit_with_the_terminal_handed_over() {
    let dir: TempDir = TempDir::new();
    init_repo(dir.path());
    git_ok(
        dir.path(),
        &["commit", "-q", "--allow-empty", "-m", "first"],
    );
    let first: String = git_ok(dir.path(), &["rev-parse", "HEAD"])
        .trim()
        .to_string();
    git_ok(
        dir.path(),
        &["commit", "-q", "--allow-empty", "-m", "second"],
    );
    let handed: std::rc::Rc<std::cell::Cell<usize>> = std::rc::Rc::new(std::cell::Cell::new(0));
    let counter: std::rc::Rc<std::cell::Cell<usize>> = std::rc::Rc::clone(&handed);
    let handoff = move |run: &mut dyn FnMut()| -> std::io::Result<()> {
        counter.set(counter.get() + 1);
        run();
        Ok(())
    };
    let mut s: Session = Session::new(
        test_git(dir.path()),
        LogBuffer::new(fixed_clock),
        Box::new(|_: &std::path::Path| Ok(())),
        Box::new(handoff),
    );
    s.resize(SIZE);
    s.input(b"js");
    assert_eq!(handed.get(), 1);
    assert_eq!(s.render(SIZE).row_text(23).trim_end(), "");
    let wide: Size = Size {
        width: 160,
        height: 24,
    };
    s.resize(wide);
    s.input(b"\x1bOR");
    let screen: ui::buffer::Buffer = s.render(wide);
    let shown: String = format!("git show {first} (exit 0)");
    assert!(
        (0..wide.height).any(|row: usize| screen.row_text(row).contains(&shown)),
        "{shown} not in the Log view"
    );
}

#[test]
fn reset_branch_to_selected_commit() {
    let dir: TempDir = TempDir::new();
    init_repo(dir.path());
    std::fs::write(dir.path().join("a.txt"), "a\n").unwrap();
    git_ok(dir.path(), &["add", "."]);
    git_ok(dir.path(), &["commit", "-q", "-m", "one"]);
    let first: String = git_ok(dir.path(), &["rev-parse", "HEAD"])
        .trim()
        .to_string();
    std::fs::write(dir.path().join("c.txt"), "c\n").unwrap();
    git_ok(dir.path(), &["add", "."]);
    git_ok(dir.path(), &["commit", "-q", "-m", "two"]);
    git_ok(dir.path(), &["branch", "later"]);
    let mut s: Session = session_in(&dir);

    std::fs::write(dir.path().join("a.txt"), "changed\n").unwrap();
    s.input(b"jR");
    assert_eq!(
        s.render(SIZE).row_text(23).trim_end(),
        "Commit or discard uncommitted changes before resetting"
    );

    git_ok(dir.path(), &["checkout", "--", "a.txt"]);
    std::fs::write(dir.path().join("b.txt"), "untracked\n").unwrap();
    s.input(b"R");
    assert_eq!(
        s.render(SIZE).row_text(23).trim_end(),
        format!("Reset main to {}? y/n", &first[..7])
    );
    s.input(b"y");
    assert_eq!(
        s.render(SIZE).row_text(23).trim_end(),
        format!("Reset main to {}", &first[..7])
    );
    assert_eq!(git_ok(dir.path(), &["rev-parse", "main"]).trim(), first);
    assert!(!dir.path().join("c.txt").exists());
    assert!(dir.path().join("b.txt").exists());

    std::fs::write(dir.path().join("c.txt"), "mine\n").unwrap();
    s.input(b"kRy");
    assert_eq!(
        s.render(SIZE).row_text(23).trim_end(),
        "Reset failed, see the Log view"
    );
    assert_eq!(git_ok(dir.path(), &["rev-parse", "main"]).trim(), first);
    assert_eq!(
        std::fs::read_to_string(dir.path().join("c.txt")).unwrap(),
        "mine\n"
    );
}

#[test]
fn shift_a_stages_all_changes_including_untracked_files() {
    let dir: TempDir = TempDir::new();
    init_repo(dir.path());
    std::fs::write(dir.path().join("a.txt"), "a\n").unwrap();
    std::fs::write(dir.path().join("gone.txt"), "g\n").unwrap();
    git_ok(dir.path(), &["add", "."]);
    git_ok(dir.path(), &["commit", "-q", "-m", "init"]);
    std::fs::write(dir.path().join("a.txt"), "changed\n").unwrap();
    std::fs::remove_file(dir.path().join("gone.txt")).unwrap();
    std::fs::create_dir(dir.path().join("sub")).unwrap();
    std::fs::write(dir.path().join("sub/new.txt"), "n\n").unwrap();
    let mut s: Session = session_in(&dir);
    s.input(b"\x1bOQA");
    assert_eq!(
        git_ok(dir.path(), &["status", "--porcelain"]),
        "M  a.txt\nD  gone.txt\nA  sub/new.txt\n"
    );
    assert_eq!(s.render(SIZE).row_text(23).trim_end(), "");
    s.input(b"A");
    assert_eq!(s.render(SIZE).row_text(23).trim_end(), "Nothing to stage");
}

/// A repository whose last commit modifies `keep.txt`, adds `added.txt` and deletes
/// `gone.txt`.
fn amendable_repo() -> TempDir {
    let dir: TempDir = TempDir::new();
    init_repo(dir.path());
    std::fs::write(dir.path().join("keep.txt"), "1\n2\n").unwrap();
    std::fs::write(dir.path().join("gone.txt"), "g\n").unwrap();
    git_ok(dir.path(), &["add", "."]);
    git_ok(dir.path(), &["commit", "-q", "-m", "base"]);
    std::fs::write(dir.path().join("keep.txt"), "1\nTWO\n").unwrap();
    std::fs::write(dir.path().join("added.txt"), "a\n").unwrap();
    git_ok(dir.path(), &["rm", "-q", "gone.txt"]);
    git_ok(dir.path(), &["add", "."]);
    git_ok(dir.path(), &["commit", "-q", "-m", "last"]);
    dir
}

#[test]
fn amend_mode_takes_a_file_out_of_the_last_commit() {
    let dir: TempDir = amendable_repo();
    let mut s: Session = session_in(&dir);
    s.input(b"\x1bOQa");
    let amending: ui::buffer::Buffer = s.render(SIZE);
    let titled: usize = (0..SIZE.height)
        .find(|row: &usize| {
            amending
                .row_text(*row)
                .starts_with("─ Staged (amended commit) ")
        })
        .expect("the staged list is titled for amending");
    assert!(amending.row_text(titled + 1).starts_with("A added.txt"));
    assert!(amending.row_text(titled + 2).starts_with("D gone.txt"));
    assert!(amending.row_text(titled + 3).starts_with("M keep.txt"));
    s.input(b"s\rc");
    assert_eq!(
        git_ok(
            dir.path(),
            &["show", "--format=%s", "--name-status", "HEAD"]
        ),
        "last\n\nD\tgone.txt\nM\tkeep.txt\n"
    );
    assert_eq!(
        git_ok(dir.path(), &["status", "--porcelain"]),
        "?? added.txt\n"
    );
    assert_eq!(git_ok(dir.path(), &["rev-list", "--count", "HEAD"]), "2\n");
}

#[test]
fn amend_mode_takes_lines_out_of_the_last_commit() {
    let dir: TempDir = amendable_repo();
    let mut s: Session = session_in(&dir);
    s.input(b"\x1bOQasjjw");
    let diff: ui::buffer::Buffer = s.render(SIZE);
    let removed: usize = (0..SIZE.height)
        .find(|row: &usize| {
            diff.row_text(*row).contains("│▶-2") || diff.row_text(*row).contains("│ -2")
        })
        .expect("the diff against the commit before the last shows -2");
    assert!(diff.row_text(removed + 1).contains("+TWO"));
    s.input(b"jj jj\r");
    assert_eq!(git_ok(dir.path(), &["show", ":keep.txt"]), "1\n2\n");
    s.input(b"c");
    assert_eq!(
        git_ok(dir.path(), &["show", "--format=", "--name-status", "HEAD"]),
        "A\tadded.txt\nD\tgone.txt\n"
    );
}

#[test]
fn an_amend_that_would_empty_the_first_commit_is_refused() {
    let dir: TempDir = TempDir::new();
    init_repo(dir.path());
    std::fs::write(dir.path().join("a.txt"), "a\n").unwrap();
    git_ok(dir.path(), &["add", "."]);
    git_ok(dir.path(), &["commit", "-q", "-m", "first"]);
    let mut s: Session = session_in(&dir);
    s.input(b"\x1bOQas\r");
    assert_eq!(
        git_ok(dir.path(), &["status", "--porcelain"]),
        "D  a.txt\n?? a.txt\n"
    );
    s.input(b"c");
    assert_eq!(
        s.render(SIZE).row_text(23).trim_end(),
        "The amended commit would be empty: stage changes first"
    );
    s.input(b"s\rc");
    assert_eq!(
        git_ok(
            dir.path(),
            &["show", "--format=%s", "--name-status", "HEAD"]
        ),
        "first\n\nA\ta.txt\n"
    );
}

#[test]
fn d_discards_changes_and_deletes_untracked_files_after_confirming() {
    let dir: TempDir = TempDir::new();
    init_repo(dir.path());
    std::fs::write(dir.path().join("a.txt"), "a\n").unwrap();
    git_ok(dir.path(), &["add", "."]);
    git_ok(dir.path(), &["commit", "-q", "-m", "init"]);
    std::fs::write(dir.path().join("a.txt"), "changed\n").unwrap();
    std::fs::write(dir.path().join("new.txt"), "n\n").unwrap();
    let mut s: Session = session_in(&dir);
    s.input(b"\x1bOQd");
    assert_eq!(
        s.render(SIZE).row_text(23).trim_end(),
        "Discard unstaged changes in a.txt? y/n"
    );
    s.input(b"n");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("a.txt")).unwrap(),
        "changed\n"
    );
    s.input(b"dy");
    assert_eq!(
        s.render(SIZE).row_text(23).trim_end(),
        "Discarded changes in a.txt"
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("a.txt")).unwrap(),
        "a\n"
    );
    s.input(b"d");
    assert_eq!(
        s.render(SIZE).row_text(23).trim_end(),
        "Delete untracked file new.txt? y/n"
    );
    s.input(b"y");
    assert_eq!(s.render(SIZE).row_text(23).trim_end(), "Deleted new.txt");
    assert!(!dir.path().join("new.txt").exists());
}
