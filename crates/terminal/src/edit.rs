//! Editing text in an external editor through a temporary file.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_FILE: AtomicUsize = AtomicUsize::new(0);

/// How the editor exited. Starting it failing, or a signal ending it, are errors instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorExit {
    /// Exited with code 0.
    Success,
    /// Exited with another code, as Vim does on `:cq`.
    Code(i32),
}

/// The file's content after the editor exited, and how it exited.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edited {
    /// The file's content, also when the editor exited with an error code.
    pub text: String,
    /// How the editor exited.
    pub exit: EditorExit,
}

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
/// Fails when the file cannot be written or read, or the editor cannot be run.
pub fn edit_text(
    text: &str,
    dir: &Path,
    editor: &dyn Fn(&Path) -> io::Result<EditorExit>,
) -> io::Result<Edited> {
    let id: usize = NEXT_FILE.fetch_add(1, Ordering::Relaxed);
    let file: TempFile =
        TempFile(dir.join(format!("mgitui-message-{}-{id}.txt", std::process::id())));
    std::fs::write(&file.0, text)?;
    let exit: EditorExit = editor(&file.0)?;
    let edited: String = std::fs::read_to_string(&file.0)?;
    Ok(Edited { text: edited, exit })
}

#[cfg(test)]
mod tests {
    use std::io;
    use std::path::{Path, PathBuf};
    use std::process;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::{Edited, EditorExit, edit_text};

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
        let editor = |path: &Path| -> io::Result<EditorExit> {
            let old: String = std::fs::read_to_string(path).unwrap();
            std::fs::write(path, format!("{old}more\n")).unwrap();
            Ok(EditorExit::Success)
        };
        assert_eq!(
            edit_text("text\n", dir.path(), &editor).unwrap(),
            Edited {
                text: "text\nmore\n".to_string(),
                exit: EditorExit::Success,
            }
        );
        assert_eq!(files_in(&dir), 0);
    }

    #[test]
    fn an_error_code_is_returned_with_the_content() {
        let dir: TempDir = TempDir::new();
        let quitting = |path: &Path| -> io::Result<EditorExit> {
            std::fs::write(path, "written before :cq\n").unwrap();
            Ok(EditorExit::Code(1))
        };
        assert_eq!(
            edit_text("text\n", dir.path(), &quitting).unwrap(),
            Edited {
                text: "written before :cq\n".to_string(),
                exit: EditorExit::Code(1),
            }
        );
        assert_eq!(files_in(&dir), 0);
    }

    #[test]
    fn failures() {
        let dir: TempDir = TempDir::new();
        let failing = |_: &Path| -> io::Result<EditorExit> { Err(io::Error::other("no editor")) };
        assert!(edit_text("x", dir.path(), &failing).is_err());
        assert_eq!(files_in(&dir), 0);
        let removing = |path: &Path| -> io::Result<EditorExit> {
            std::fs::remove_file(path).unwrap();
            Ok(EditorExit::Success)
        };
        assert!(edit_text("x", dir.path(), &removing).is_err());
        let missing: std::path::PathBuf = dir.path().join("missing");
        assert!(edit_text("x", &missing, &removing).is_err());
    }
}
