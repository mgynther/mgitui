//! The logic layer of mgitui: application state and behaviour. Logic is abstract: everything
//! outside the application state is done through the functions in [`ports::Ports`].

pub mod app;
pub mod cursor;
pub mod event;
pub mod log;
pub mod model;
pub mod patch;
pub mod ports;
pub mod search;

#[cfg(test)]
mod test_ports;
