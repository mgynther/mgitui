//! The validation layer of mgitui: turns raw user input into strongly typed values.

pub mod log;

mod branch;
mod error;
mod input;

pub use branch::{BranchName, BranchNameProblem, branch_name};
pub use error::ValidationError;
pub use input::{
    CheckoutChoice, CommitMessage, SearchText, checkout_choice, commit_message, search_text,
};
