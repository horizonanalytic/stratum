//! Stratum Workshop - IDLE-style Interactive Environment
//!
//! A clean, minimal IDE focused on the REPL with optional file editing.
//! Inspired by Python's IDLE - simple, approachable, effective.

use crate::panels::{ReplMessage, ReplPanel};
use iced::keyboard;
use iced::keyboard::key;
use iced::widget::{button, column, container, row, rule, scrollable, text, text_editor, Space};
use iced::{Color, Element, Length, Subscription, Task, Theme};
use rfd::AsyncFileDialog;
use std::path::PathBuf;

/// Main application state
pub struct Workshop {
    /// The REPL panel (main focus)
    pub repl: ReplPanel,
    /// Optional editor state (when a file is open)
    editor: Option<EditorState>,
    /// Whether to show the editor pane
    show_editor: bool,
    /// Modal dialog state
    modal: Option<ModalState>,
    /// Status message
    status: String,
}

/// Simple editor state for a single file
struct EditorState {
    /// File path (None for untitled)
    path: Option<PathBuf>,
    /// Editor content
    content: text_editor::Content,
    /// Whether the file has been modified
    modified: bool,
}

/// Modal dialog types
#[derive(Debug, Clone)]
pub enum ModalState {
    About,
    UnsavedChanges,
}

/// High-level application state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkshopState {
    Idle,
    Running,
}

/// Messages for the Workshop
#[derive(Debug, Clone)]
pub enum WorkshopMessage {
    // REPL
    Repl(ReplMessage),

    // File operations
    NewFile,
    OpenFile,
    SaveFile,
    CloseFile,

    // Editor
    EditorAction(text_editor::Action),
    ToggleEditor,

    // Run
    RunFile,

    // Dialogs
    FileDialogOpened(Option<(PathBuf, String)>),
    FileSaved(PathBuf),
    FileSaveError(String),

    // Modal
    ShowAbout,
    ModalClose,
    ModalDiscard,

    // App
    Exit,
}

impl Default for Workshop {
    fn default() -> Self {
        Self::new()
    }
}

impl Workshop {
    /// Create a new Workshop instance
    pub fn new() -> Self {
        Self {
            repl: ReplPanel::new(),
            editor: None,
            show_editor: false,
            modal: None,
            status: "Ready".to_string(),
        }
    }

    /// Get the window title
    pub fn title(&self) -> String {
        if let Some(editor) = &self.editor {
            let name = editor
                .path
                .as_ref()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Untitled".to_string());
            let modified = if editor.modified { " *" } else { "" };
            format!("{}{} - Stratum Shell", name, modified)
        } else {
            "Stratum Shell".to_string()
        }
    }

    /// Handle messages
    pub fn update(&mut self, message: WorkshopMessage) -> Task<WorkshopMessage> {
        match message {
            WorkshopMessage::Repl(msg) => {
                self.repl.update(msg);
            }

            WorkshopMessage::NewFile => {
                if self.editor.as_ref().is_some_and(|e| e.modified) {
                    self.modal = Some(ModalState::UnsavedChanges);
                } else {
                    self.editor = Some(EditorState {
                        path: None,
                        content: text_editor::Content::new(),
                        modified: false,
                    });
                    self.show_editor = true;
                    self.status = "New file".to_string();
                }
            }

            WorkshopMessage::OpenFile => {
                return Task::perform(
                    async {
                        let file = AsyncFileDialog::new()
                            .add_filter("Stratum", &["strat", "st"])
                            .add_filter("All files", &["*"])
                            .set_title("Open File")
                            .pick_file()
                            .await;

                        match file {
                            Some(handle) => {
                                let path = handle.path().to_path_buf();
                                match tokio::fs::read_to_string(&path).await {
                                    Ok(content) => Some((path, content)),
                                    Err(_) => None,
                                }
                            }
                            None => None,
                        }
                    },
                    WorkshopMessage::FileDialogOpened,
                );
            }

            WorkshopMessage::FileDialogOpened(result) => {
                if let Some((path, content)) = result {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    self.editor = Some(EditorState {
                        path: Some(path),
                        content: text_editor::Content::with_text(&content),
                        modified: false,
                    });
                    self.show_editor = true;
                    self.status = format!("Opened {}", name);
                }
            }

            WorkshopMessage::SaveFile => {
                if let Some(editor) = &self.editor {
                    if let Some(path) = &editor.path {
                        let content = editor.content.text();
                        let path_clone = path.clone();
                        return Task::perform(
                            async move {
                                match tokio::fs::write(&path_clone, content).await {
                                    Ok(()) => WorkshopMessage::FileSaved(path_clone),
                                    Err(e) => WorkshopMessage::FileSaveError(e.to_string()),
                                }
                            },
                            |msg| msg,
                        );
                    } else {
                        // Save As for untitled files
                        let content = editor.content.text();
                        return Task::perform(
                            async move {
                                let file = AsyncFileDialog::new()
                                    .add_filter("Stratum", &["strat"])
                                    .set_title("Save As")
                                    .set_file_name("untitled.strat")
                                    .save_file()
                                    .await;

                                if let Some(handle) = file {
                                    let path = handle.path().to_path_buf();
                                    match tokio::fs::write(&path, &content).await {
                                        Ok(()) => WorkshopMessage::FileSaved(path),
                                        Err(e) => WorkshopMessage::FileSaveError(e.to_string()),
                                    }
                                } else {
                                    WorkshopMessage::FileSaveError("Cancelled".to_string())
                                }
                            },
                            |msg| msg,
                        );
                    }
                }
            }

            WorkshopMessage::FileSaved(path) => {
                if let Some(editor) = &mut self.editor {
                    editor.path = Some(path.clone());
                    editor.modified = false;
                }
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                self.status = format!("Saved {}", name);
            }

            WorkshopMessage::FileSaveError(err) => {
                if err != "Cancelled" {
                    self.status = format!("Save failed: {}", err);
                }
            }

            WorkshopMessage::CloseFile => {
                if self.editor.as_ref().is_some_and(|e| e.modified) {
                    self.modal = Some(ModalState::UnsavedChanges);
                } else {
                    self.editor = None;
                    self.show_editor = false;
                    self.status = "Ready".to_string();
                }
            }

            WorkshopMessage::EditorAction(action) => {
                if let Some(editor) = &mut self.editor {
                    let is_edit = action.is_edit();
                    editor.content.perform(action);
                    if is_edit {
                        editor.modified = true;
                    }
                }
            }

            WorkshopMessage::ToggleEditor => {
                if self.editor.is_some() {
                    self.show_editor = !self.show_editor;
                }
            }

            WorkshopMessage::RunFile => {
                if let Some(editor) = &self.editor {
                    let source = editor.content.text();
                    // Execute the file content in the REPL
                    // Split into lines and execute each
                    for line in source.lines() {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() && !trimmed.starts_with("//") {
                            self.repl.update(ReplMessage::InputChanged(line.to_string()));
                            self.repl.update(ReplMessage::Submit);
                        }
                    }
                    self.status = "Executed file".to_string();
                }
            }

            WorkshopMessage::ShowAbout => {
                self.modal = Some(ModalState::About);
            }

            WorkshopMessage::ModalClose => {
                self.modal = None;
            }

            WorkshopMessage::ModalDiscard => {
                // Discard changes and close
                self.modal = None;
                self.editor = None;
                self.show_editor = false;
                self.status = "Ready".to_string();
            }

            WorkshopMessage::Exit => {
                if self.editor.as_ref().is_some_and(|e| e.modified) {
                    self.modal = Some(ModalState::UnsavedChanges);
                } else {
                    std::process::exit(0);
                }
            }
        }

        Task::none()
    }

    /// Render the application
    pub fn view(&self) -> Element<'_, WorkshopMessage> {
        let menu_bar = self.menu_bar();

        // Main content: optional editor + REPL
        let main_content = if self.show_editor && self.editor.is_some() {
            let editor = self.editor.as_ref().unwrap();
            let editor_view = self.editor_view(editor);

            column![
                editor_view,
                rule::horizontal(1),
                self.repl.view().map(WorkshopMessage::Repl),
            ]
            .spacing(0)
        } else {
            column![self.repl.view().map(WorkshopMessage::Repl),]
        };

        let status_bar = self.status_bar();

        let base_content: Element<WorkshopMessage> = container(
            column![menu_bar, rule::horizontal(1), main_content, status_bar]
                .spacing(0)
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into();

        // Render modal if present
        if let Some(modal_state) = &self.modal {
            self.modal_overlay(base_content, modal_state)
        } else {
            base_content
        }
    }

    /// Render the menu bar
    fn menu_bar(&self) -> Element<'_, WorkshopMessage> {
        container(
            row![
                Self::menu_button("New", WorkshopMessage::NewFile),
                Self::menu_button("Open", WorkshopMessage::OpenFile),
                Self::menu_button("Save", WorkshopMessage::SaveFile),
                Self::menu_button("Close", WorkshopMessage::CloseFile),
                text("|").size(12),
                Self::menu_button("Run", WorkshopMessage::RunFile),
                text("|").size(12),
                Self::menu_button("About", WorkshopMessage::ShowAbout),
                Space::new().width(Length::Fill),
            ]
            .spacing(4)
            .padding([6, 8])
            .align_y(iced::Alignment::Center),
        )
        .width(Length::Fill)
        .style(|theme: &Theme| {
            let palette = theme.extended_palette();
            container::Style {
                background: Some(palette.background.weak.color.into()),
                ..Default::default()
            }
        })
        .into()
    }

    /// Create a menu button
    fn menu_button(label: &'static str, message: WorkshopMessage) -> Element<'static, WorkshopMessage> {
        button(text(label).size(12))
            .on_press(message)
            .padding([4, 10])
            .style(button::text)
            .into()
    }

    /// Render the editor view
    fn editor_view<'a>(&self, editor: &'a EditorState) -> Element<'a, WorkshopMessage> {
        let title = editor
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Untitled".to_string());

        let modified = if editor.modified { " *" } else { "" };

        let header = container(
            row![
                text(format!("{}{}", title, modified)).size(12),
                Space::new().width(Length::Fill),
                button(text("x").size(10))
                    .on_press(WorkshopMessage::CloseFile)
                    .padding([2, 6])
                    .style(button::text),
            ]
            .align_y(iced::Alignment::Center),
        )
        .padding([4, 8])
        .width(Length::Fill)
        .style(|theme: &Theme| {
            let palette = theme.extended_palette();
            container::Style {
                background: Some(palette.background.weak.color.into()),
                ..Default::default()
            }
        });

        let editor_widget = text_editor(&editor.content)
            .on_action(WorkshopMessage::EditorAction)
            .font(iced::Font::MONOSPACE)
            .size(13)
            .height(Length::FillPortion(1));

        container(column![header, scrollable(editor_widget).height(Length::FillPortion(1))])
            .width(Length::Fill)
            .height(Length::FillPortion(1))
            .into()
    }

    /// Render the status bar
    fn status_bar(&self) -> Element<'_, WorkshopMessage> {
        container(text(&self.status).size(11))
            .padding([2, 8])
            .width(Length::Fill)
            .style(|theme: &Theme| {
                let palette = theme.extended_palette();
                container::Style {
                    background: Some(palette.background.weak.color.into()),
                    ..Default::default()
                }
            })
            .into()
    }

    /// Render modal overlay
    fn modal_overlay<'a>(
        &self,
        base: Element<'a, WorkshopMessage>,
        modal_state: &ModalState,
    ) -> Element<'a, WorkshopMessage> {
        use iced::widget::{center, mouse_area, opaque, stack};

        let dialog = match modal_state {
            ModalState::About => {
                let content = column![
                    text("Stratum Shell").size(20),
                    text("Version 0.1.0").size(13),
                    Space::new().height(8),
                    text("An interactive environment for the").size(12),
                    text("Stratum programming language.").size(12),
                    Space::new().height(8),
                    text("2024 Horizon Analytic Studios").size(11),
                    Space::new().height(12),
                    button(text("OK").size(12))
                        .on_press(WorkshopMessage::ModalClose)
                        .padding([6, 16])
                        .style(button::primary),
                ]
                .spacing(4)
                .padding(24)
                .align_x(iced::Alignment::Center);

                container(content).style(container::rounded_box)
            }

            ModalState::UnsavedChanges => {
                let content = column![
                    text("Unsaved Changes").size(18),
                    text("You have unsaved changes.").size(13),
                    Space::new().height(12),
                    row![
                        button(text("Save").size(12))
                            .on_press(WorkshopMessage::SaveFile)
                            .padding([6, 12])
                            .style(button::primary),
                        button(text("Discard").size(12))
                            .on_press(WorkshopMessage::ModalDiscard)
                            .padding([6, 12])
                            .style(button::danger),
                        button(text("Cancel").size(12))
                            .on_press(WorkshopMessage::ModalClose)
                            .padding([6, 12])
                            .style(button::secondary),
                    ]
                    .spacing(8),
                ]
                .spacing(8)
                .padding(20);

                container(content).style(container::rounded_box)
            }
        };

        let backdrop = container(center(opaque(dialog)))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme: &Theme| container::Style {
                background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.5).into()),
                ..Default::default()
            });

        stack![
            base,
            opaque(mouse_area(backdrop).on_press(WorkshopMessage::ModalClose))
        ]
        .into()
    }

    /// Keyboard subscription
    pub fn subscription(&self) -> Subscription<WorkshopMessage> {
        keyboard::listen().filter_map(|event| {
            let keyboard::Event::KeyPressed { key, modifiers, .. } = event else {
                return None;
            };

            // Escape closes modals
            if let keyboard::Key::Named(key::Named::Escape) = key {
                return Some(WorkshopMessage::ModalClose);
            }

            // Keyboard shortcuts
            if let keyboard::Key::Character(ref c) = key {
                if modifiers.command() {
                    match c.as_ref() {
                        "n" => return Some(WorkshopMessage::NewFile),
                        "o" => return Some(WorkshopMessage::OpenFile),
                        "s" => return Some(WorkshopMessage::SaveFile),
                        "w" => return Some(WorkshopMessage::CloseFile),
                        "r" => return Some(WorkshopMessage::RunFile),
                        "q" => return Some(WorkshopMessage::Exit),
                        _ => {}
                    }
                }
            }

            // F5 to run
            if let keyboard::Key::Named(key::Named::F5) = key {
                return Some(WorkshopMessage::RunFile);
            }

            None
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workshop_creation() {
        let workshop = Workshop::new();
        assert!(workshop.editor.is_none());
        assert!(!workshop.show_editor);
        assert!(workshop.modal.is_none());
    }

    #[test]
    fn test_new_file() {
        let mut workshop = Workshop::new();
        let _ = workshop.update(WorkshopMessage::NewFile);
        assert!(workshop.editor.is_some());
        assert!(workshop.show_editor);
    }

    #[test]
    fn test_close_file() {
        let mut workshop = Workshop::new();
        let _ = workshop.update(WorkshopMessage::NewFile);
        assert!(workshop.editor.is_some());

        let _ = workshop.update(WorkshopMessage::CloseFile);
        assert!(workshop.editor.is_none());
        assert!(!workshop.show_editor);
    }
}
