//! The wiring layer of mgitui: connects the other layers and runs the app.

pub mod convert;
pub mod editor;
pub mod log;
pub mod model;
pub mod ports;
pub mod session;
pub mod startup;

#[cfg(test)]
mod test_support;
