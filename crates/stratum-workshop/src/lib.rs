//! Stratum Shell - Interactive Environment
//!
//! A clean, minimal REPL-focused environment for the Stratum programming language.
//! Inspired by Python's IDLE - simple, approachable, effective.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────┐
//! │  New  Open  Save  Close  |  Run  |  About      │
//! ├─────────────────────────────────────────────────┤
//! │  [Optional: Editor pane when file is open]     │
//! ├─────────────────────────────────────────────────┤
//! │                                                 │
//! │  >>> REPL                                       │
//! │  >>> _                                          │
//! │                                                 │
//! ├─────────────────────────────────────────────────┤
//! │  Ready                                          │
//! └─────────────────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ```no_run
//! use stratum_workshop::launch;
//! use std::path::PathBuf;
//!
//! // Launch with no initial path (REPL only)
//! launch(None).unwrap();
//!
//! // Launch with a file
//! launch(Some(PathBuf::from("/path/to/file.strat"))).unwrap();
//! ```

pub mod panels;
pub mod workshop;

pub use panels::{ReplMessage, ReplPanel};
pub use workshop::{Workshop, WorkshopMessage, WorkshopState};

use iced::{Size, Subscription, Task};
use std::path::PathBuf;
use std::sync::OnceLock;

/// Global storage for initial path to pass to boot function
static INITIAL_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Launch Stratum Shell
///
/// # Arguments
///
/// * `initial_path` - Optional path to open on startup (file only)
///
/// # Returns
///
/// Returns `iced::Result` which is `Ok(())` on successful exit or an error.
///
/// # Examples
///
/// ```no_run
/// use stratum_workshop::launch;
/// use std::path::PathBuf;
///
/// // Open Shell (REPL only)
/// launch(None).unwrap();
///
/// // Open Shell with a file
/// launch(Some(PathBuf::from("/my/script.strat"))).unwrap();
/// ```
pub fn launch(initial_path: Option<PathBuf>) -> iced::Result {
    // Store initial path for boot function to access
    let _ = INITIAL_PATH.set(initial_path);

    iced::application(boot, update, view)
        .title("Stratum Shell")
        .window_size(Size::new(700.0, 500.0))
        .subscription(subscription)
        .run()
}

/// Boot function - initializes application state
fn boot() -> (Workshop, Task<WorkshopMessage>) {
    let mut workshop = Workshop::new();

    // Handle initial path argument
    if let Some(Some(path)) = INITIAL_PATH.get() {
        if path.is_file() {
            if let Ok(content) = std::fs::read_to_string(path) {
                // Directly open the file
                let _ = workshop.update(WorkshopMessage::FileDialogOpened(Some((
                    path.clone(),
                    content,
                ))));
            }
        }
    }

    (workshop, Task::none())
}

/// Update function
fn update(workshop: &mut Workshop, message: WorkshopMessage) -> Task<WorkshopMessage> {
    workshop.update(message)
}

/// View function
fn view(workshop: &Workshop) -> iced::Element<'_, WorkshopMessage> {
    workshop.view()
}

/// Subscription function for keyboard shortcuts
fn subscription(workshop: &Workshop) -> Subscription<WorkshopMessage> {
    workshop.subscription()
}
