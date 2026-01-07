//! Error types for the Stratum GUI framework

use thiserror::Error;

/// GUI-related errors
#[derive(Debug, Error)]
pub enum GuiError {
    /// Window creation failed
    #[error("Failed to create window: {0}")]
    WindowCreation(String),

    /// State binding error
    #[error("State binding error: {0}")]
    StateBinding(String),

    /// Widget rendering error
    #[error("Widget rendering error: {0}")]
    WidgetRender(String),

    /// Event handling error
    #[error("Event handling error: {0}")]
    EventHandling(String),

    /// Invalid state type
    #[error("Invalid state type: expected {expected}, got {actual}")]
    InvalidStateType { expected: String, actual: String },

    /// Iced backend error
    #[error("Iced error: {0}")]
    Iced(String),
}

/// Result type alias for GUI operations
pub type GuiResult<T> = Result<T, GuiError>;
