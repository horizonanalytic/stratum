//! Stratum Workshop - Lightweight IDE bundled with Stratum
//!
//! A modern IDE built with Stratum's own GUI framework (dogfooding).
//! Like Python's IDLE but modern, providing immediate productivity.
//!
//! # Architecture
//!
//! Workshop uses iced's `pane_grid` for a flexible, resizable panel layout:
//!
//! ```text
//! ┌─────────────────────────────────────────────────┐
//! │  Menu Bar                                       │
//! ├────────────┬────────────────────────────────────┤
//! │            │  Tab Bar                           │
//! │   File     ├────────────────────────────────────┤
//! │  Browser   │                                    │
//! │            │     Editor Area                    │
//! │            │                                    │
//! │            ├────────────────────────────────────┤
//! │            │  Run | Debug | Build               │
//! │            ├────────────────────────────────────┤
//! │            │     Output / REPL                  │
//! └────────────┴────────────────────────────────────┘
//! ```
//!
//! # Usage
//!
//! ```no_run
//! use stratum_workshop::launch;
//! use std::path::PathBuf;
//!
//! // Launch with no initial path
//! launch(None).unwrap();
//!
//! // Launch with a folder
//! launch(Some(PathBuf::from("/path/to/project"))).unwrap();
//!
//! // Launch with a file
//! launch(Some(PathBuf::from("/path/to/file.strat"))).unwrap();
//! ```
//!
//! # Modules
//!
//! - [`workshop`]: Main application structure
//! - [`config`]: Layout persistence and user preferences
//! - [`panels`]: Individual panel implementations
//! - [`widgets`]: Custom widgets (code editor, etc.)

pub mod config;
pub mod debug;
pub mod execution;
pub mod highlight;
pub mod panels;
pub mod widgets;
pub mod workshop;

pub use config::{LayoutConfig, PanelVisibility, WorkshopConfig};
pub use debug::{DebugResult, DebugSession, DebugSessionState};
pub use execution::{execute_source, execute_source_async, CancellationToken, ExecutionResult};
pub use highlight::{HighlightSettings, HighlightTheme, StratumHighlighter};
pub use panels::{EditorMessage, EditorPanel, EditorTab, FileBrowserPanel, OutputPanel, RecentFilesData, ReplPanel, WelcomeAction};
pub use widgets::{code_editor, CodeEditorMessage, CodeEditorState, CursorMovement, Position, Selection};
pub use workshop::{Workshop, WorkshopMessage, WorkshopState};

use iced::{Size, Subscription, Task};
use std::path::PathBuf;
use std::sync::OnceLock;

/// Global storage for initial path to pass to boot function
static INITIAL_PATH: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Launch Stratum Workshop IDE
///
/// # Arguments
///
/// * `initial_path` - Optional path to open on startup (file or folder)
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
/// // Open Workshop with empty editor
/// launch(None).unwrap();
///
/// // Open Workshop with a specific folder
/// launch(Some(PathBuf::from("/my/project"))).unwrap();
/// ```
pub fn launch(initial_path: Option<PathBuf>) -> iced::Result {
    // Store initial path for boot function to access
    let _ = INITIAL_PATH.set(initial_path);

    // Load config for window settings
    let config = WorkshopConfig::load();

    iced::application(boot, update, view)
        .title("Stratum Workshop")
        .window_size(Size::new(
            config.window_size.0 as f32,
            config.window_size.1 as f32,
        ))
        .subscription(subscription)
        .run()
}

/// Boot function - initializes application state
fn boot() -> (Workshop, Task<WorkshopMessage>) {
    let mut workshop = Workshop::new();

    // Handle initial path argument
    let task = if let Some(Some(path)) = INITIAL_PATH.get() {
        if path.is_dir() {
            let _ = workshop.file_browser.open_folder(path.clone());
            Task::none()
        } else if path.is_file() {
            match std::fs::read_to_string(path) {
                Ok(content) => Task::done(WorkshopMessage::FileOpened(path.clone(), content)),
                Err(_) => Task::none(),
            }
        } else {
            Task::none()
        }
    } else {
        Task::none()
    };

    (workshop, task)
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
