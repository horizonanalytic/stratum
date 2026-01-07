//! Custom widgets for Stratum Workshop
//!
//! This module contains specialized widgets that provide features
//! not available in iced's built-in widget library.

pub mod code_editor;

pub use code_editor::{
    code_editor, CodeEditorMessage, CodeEditorState, CursorMovement, Position, Selection,
};
