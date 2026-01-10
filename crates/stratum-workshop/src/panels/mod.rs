//! Panel implementations for Stratum Shell
//!
//! For the simplified IDLE-style interface, we only need the REPL panel.

mod repl;

pub use repl::{ReplMessage, ReplPanel};
