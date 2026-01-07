//! Stratum Workshop - Lightweight IDE bundled with Stratum
//!
//! Usage:
//!   workshop                 Open Workshop with empty editor
//!   workshop <folder>        Open Workshop with folder
//!   workshop <file.strat>    Open Workshop with file

use std::path::PathBuf;

fn main() -> iced::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let initial_path = args.first().cloned().map(PathBuf::from);

    stratum_workshop::launch(initial_path)
}
