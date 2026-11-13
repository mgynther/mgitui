//! The logic layer of mgitui: application state and behaviour. Logic is abstract: everything
//! outside the application state is done through the functions in [`ports::Ports`].

pub mod app;
pub mod event;
pub mod log;
pub mod ports;

#[cfg(test)]
mod test_ports;
