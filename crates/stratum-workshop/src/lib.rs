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

use iced::{window, Size, Subscription, Task};
use std::path::PathBuf;
use std::sync::OnceLock;

/// Embedded application icon (PNG)
const ICON_PNG: &[u8] = include_bytes!("../../../assets/icon.png");

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

    // Load application icon from embedded PNG
    let icon = load_icon();

    let mut app = iced::application(boot, update, view)
        .title("Stratum Workshop")
        .window_size(Size::new(700.0, 500.0))
        .subscription(subscription);

    // Set window icon if successfully loaded
    if let Some(icon) = icon {
        app = app.window(window::Settings {
            icon: Some(icon),
            ..Default::default()
        });
    }

    app.run()
}

/// Load the application icon from embedded PNG data
fn load_icon() -> Option<window::Icon> {
    let img = image::load_from_memory(ICON_PNG).ok()?.into_rgba8();
    let (width, height) = img.dimensions();
    let rgba = img.into_raw();
    window::icon::from_rgba(rgba, width, height).ok()
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
