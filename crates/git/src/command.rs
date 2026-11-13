//! Typed git commands.

/// A git command this application runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// `git rev-parse --show-toplevel`: the root directory of the repository.
    ShowToplevel,
}

impl Command {
    /// The command line arguments, without the leading `git`.
    #[must_use]
    pub fn args(&self) -> Vec<String> {
        let args: &[&str] = match self {
            Command::ShowToplevel => &["rev-parse", "--show-toplevel"],
        };
        args.iter().map(|a: &&str| (*a).to_string()).collect()
    }

    /// Exit codes that mean success for this command.
    #[must_use]
    pub fn success_codes(&self) -> &'static [i32] {
        match self {
            Command::ShowToplevel => &[0],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Command;

    #[test]
    fn show_toplevel() {
        assert_eq!(
            Command::ShowToplevel.args(),
            vec!["rev-parse", "--show-toplevel"]
        );
        assert_eq!(Command::ShowToplevel.success_codes(), &[0]);
    }
}
