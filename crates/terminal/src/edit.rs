//! Editing text in an external editor through a temporary file.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_FILE: AtomicUsize = AtomicUsize::new(0);

/// A file removed when dropped.
struct TempFile(PathBuf);

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Writes `text` to a temporary file in `dir`, runs `editor` on it and returns the file's
/// content afterwards. The file is removed in every case.
///
/// # Errors
///
/// Fails when the file cannot be written or read, or the editor fails.
pub fn edit_text(
    text: &str,
    dir: &Path,
    editor: &dyn Fn(&Path) -> io::Result<()>,
) -> io::Result<String> {
    let id: usize = NEXT_FILE.fetch_add(1, Ordering::Relaxed);
    let file: TempFile =
        TempFile(dir.join(format!("mgitui-message-{}-{id}.txt", std::process::id())));
    std::fs::write(&file.0, text)?;
    editor(&file.0)?;
    std::fs::read_to_string(&file.0)
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::{Path, PathBuf};
    use std::process;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::edit_text;

    static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

    /// A directory under the system temp directory, removed when dropped.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let id: usize = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
            let path: PathBuf =
                std::env::temp_dir().join(format!("mgitui-terminal-test-{}-{id}", process::id()));
            std::fs::create_dir_all(&path).unwrap();
            TempDir(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn files_in(dir: &TempDir) -> usize {
        std::fs::read_dir(dir.path()).unwrap().count()
    }

    #[test]
    fn returns_edited_content_and_removes_the_file() {
        let dir: TempDir = TempDir::new();
        let editor = |path: &Path| -> io::Result<()> {
            let old: String = std::fs::read_to_string(path).unwrap();
            std::fs::write(path, format!("{old}more\n"))
        };
        assert_eq!(
            edit_text("text\n", dir.path(), &editor).unwrap(),
            "text\nmore\n"
        );
        assert_eq!(files_in(&dir), 0);
    }

    #[test]
    fn failures() {
        let dir: TempDir = TempDir::new();
        let failing = |_: &Path| -> io::Result<()> { Err(io::Error::other("no editor")) };
        assert!(edit_text("x", dir.path(), &failing).is_err());
        assert_eq!(files_in(&dir), 0);
        let removing = |path: &Path| -> io::Result<()> { std::fs::remove_file(path) };
        assert!(edit_text("x", dir.path(), &removing).is_err());
        let missing: std::path::PathBuf = dir.path().join("missing");
        assert!(edit_text("x", &missing, &removing).is_err());
    }
}
