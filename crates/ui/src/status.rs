//! The wording of the prompts and questions in the status line.

/// What a text prompt asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptKind {
    /// A search term.
    Search,
    /// The name of a new branch.
    BranchName,
    /// The number of the branch to check out.
    CheckoutChoice(Vec<String>),
    /// The number of the branch to delete.
    DeleteBranch(Vec<String>),
    /// The number of the branch to rebase.
    RebaseBranch {
        /// The branches to choose from.
        branches: Vec<String>,
        /// Whether the rebase is interactive.
        interactive: bool,
    },
    /// The number of the branch to rebase onto.
    RebaseOnto {
        /// The branch being rebased.
        branch: String,
        /// The branches to choose from.
        onto: Vec<String>,
        /// Whether the rebase is interactive.
        interactive: bool,
    },
}

/// A question answered with y or n.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Question {
    /// Whether to quit.
    Quit,
    /// Whether to delete a branch, also when it is not merged.
    DeleteBranch {
        /// The branch name.
        name: String,
    },
    /// Whether to reset the checked out branch to a commit.
    ResetBranch {
        /// The checked out branch.
        branch: String,
        /// The full commit hash.
        hash: String,
    },
    /// Whether to delete an untracked file.
    DeleteUntracked {
        /// The file's path.
        path: String,
    },
    /// Whether to discard the unstaged changes of a tracked file.
    DiscardChanges {
        /// The file's path.
        path: String,
    },
}

/// The label in front of the typed text.
#[must_use]
pub fn prompt_label(kind: &PromptKind) -> String {
    match kind {
        PromptKind::Search => "/".to_string(),
        PromptKind::BranchName => "New branch name: ".to_string(),
        PromptKind::CheckoutChoice(branches) => format!("Check out ({}): ", numbered(branches)),
        PromptKind::DeleteBranch(branches) => {
            format!("Delete which branch ({}): ", numbered(branches))
        }
        PromptKind::RebaseBranch {
            branches,
            interactive,
        } => format!(
            "{} which branch ({}): ",
            rebase_verb(*interactive),
            numbered(branches)
        ),
        PromptKind::RebaseOnto {
            branch,
            onto,
            interactive,
        } => format!(
            "{} {branch} onto ({}): ",
            rebase_verb(*interactive),
            numbered(onto)
        ),
    }
}

/// The question, ending in `y/n`. Commits are shown by their short hash.
#[must_use]
pub fn question_label(question: &Question) -> String {
    match question {
        Question::Quit => "Quit? y/n".to_string(),
        Question::DeleteBranch { name } => {
            format!("Delete branch {name} even if it is not merged? y/n")
        }
        Question::ResetBranch { branch, hash } => {
            format!("Reset {branch} to {}? y/n", hash.get(..7).unwrap_or(hash))
        }
        Question::DeleteUntracked { path } => format!("Delete untracked file {path}? y/n"),
        Question::DiscardChanges { path } => format!("Discard unstaged changes in {path}? y/n"),
    }
}

fn rebase_verb(interactive: bool) -> &'static str {
    if interactive {
        "Interactively rebase"
    } else {
        "Rebase"
    }
}

/// `1 a, 2 b, ...` for a numbered prompt.
fn numbered(items: &[String]) -> String {
    let choices: Vec<String> = items
        .iter()
        .enumerate()
        .map(|(index, item): (usize, &String)| format!("{} {item}", index + 1))
        .collect();
    choices.join(", ")
}

#[cfg(test)]
mod tests {
    use super::{PromptKind, Question, prompt_label, question_label};

    #[test]
    fn prompt_labels() {
        let branches: Vec<String> = vec!["a".to_string(), "b".to_string()];
        let cases: [(PromptKind, &str); 7] = [
            (PromptKind::Search, "/"),
            (PromptKind::BranchName, "New branch name: "),
            (
                PromptKind::CheckoutChoice(branches.clone()),
                "Check out (1 a, 2 b): ",
            ),
            (
                PromptKind::DeleteBranch(branches.clone()),
                "Delete which branch (1 a, 2 b): ",
            ),
            (
                PromptKind::RebaseBranch {
                    branches: branches.clone(),
                    interactive: false,
                },
                "Rebase which branch (1 a, 2 b): ",
            ),
            (
                PromptKind::RebaseBranch {
                    branches: branches.clone(),
                    interactive: true,
                },
                "Interactively rebase which branch (1 a, 2 b): ",
            ),
            (
                PromptKind::RebaseOnto {
                    branch: "t".to_string(),
                    onto: branches,
                    interactive: false,
                },
                "Rebase t onto (1 a, 2 b): ",
            ),
        ];
        for (kind, label) in cases {
            assert_eq!(prompt_label(&kind), label);
        }
    }

    #[test]
    fn question_labels() {
        let cases: [(Question, &str); 6] = [
            (Question::Quit, "Quit? y/n"),
            (
                Question::DeleteBranch {
                    name: "old".to_string(),
                },
                "Delete branch old even if it is not merged? y/n",
            ),
            (
                Question::ResetBranch {
                    branch: "main".to_string(),
                    hash: "1234567890".to_string(),
                },
                "Reset main to 1234567? y/n",
            ),
            (
                Question::ResetBranch {
                    branch: "main".to_string(),
                    hash: "123".to_string(),
                },
                "Reset main to 123? y/n",
            ),
            (
                Question::DeleteUntracked {
                    path: "new.txt".to_string(),
                },
                "Delete untracked file new.txt? y/n",
            ),
            (
                Question::DiscardChanges {
                    path: "a.txt".to_string(),
                },
                "Discard unstaged changes in a.txt? y/n",
            ),
        ];
        for (question, label) in cases {
            assert_eq!(question_label(&question), label);
        }
    }
}
