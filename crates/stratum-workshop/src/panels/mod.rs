//! Panel implementations for Workshop
//!
//! Each panel is a self-contained component that renders within a pane.

mod data_explorer;
mod debug;
mod editor;
mod file_browser;
mod output;
mod repl;

pub use data_explorer::{DataExplorerAction, DataExplorerMessage, DataExplorerPanel};
pub use debug::{DebugPanel, DebugPanelMessage};
pub use editor::{EditorMessage, EditorPanel, EditorTab, FindState, TabContextMenuState, TabDragState};
pub use file_browser::{FileBrowserMessage, FileBrowserPanel};
pub use output::{OutputMessage, OutputPanel, SourceLocation};
pub use repl::{ReplMessage, ReplPanel};

use iced::widget::{button, column, container, row, text, Column, Space};
use iced::{Alignment, Element, Length, Theme};
use std::path::PathBuf;
use stratum_core::{DebugStackFrame, DebugVariable};

/// Panel identifier for the pane grid
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanelKind {
    FileBrowser,
    Editor,
    Output,
    DataExplorer,
}

impl PanelKind {
    /// Get the display title for this panel
    pub fn title(&self) -> &'static str {
        match self {
            Self::FileBrowser => "Files",
            Self::Editor => "Editor",
            Self::Output => "Output",
            Self::DataExplorer => "Data Explorer",
        }
    }
}

/// Recent files data for welcome screen
pub struct RecentFilesData {
    pub recent_files: Vec<PathBuf>,
    pub recent_folders: Vec<PathBuf>,
}

/// Content stored in each pane
#[derive(Debug)]
pub struct PaneContent {
    pub kind: PanelKind,
}

impl PaneContent {
    pub fn new(kind: PanelKind) -> Self {
        Self { kind }
    }

    /// Render the pane header bar
    pub fn header<'a, Message: 'a>(&self) -> Element<'a, Message> {
        container(text(self.kind.title()).size(14))
            .padding(4)
        .style(|theme: &Theme| {
            let palette = theme.extended_palette();
            container::Style {
                background: Some(palette.background.weak.color.into()),
                ..Default::default()
            }
        })
        .width(Length::Fill)
        .into()
    }

    /// Render the pane content
    #[allow(clippy::too_many_arguments)]
    pub fn content<'a, Message: Clone + 'static>(
        &self,
        editor: &'a EditorPanel,
        file_browser: &'a FileBrowserPanel,
        output: &'a OutputPanel,
        repl: &'a ReplPanel,
        data_explorer: &'a DataExplorerPanel,
        debug_panel: &'a DebugPanel,
        debug_call_stack: &'a [DebugStackFrame],
        debug_locals: &'a [DebugVariable],
        is_debugging: bool,
        recent_data: Option<&RecentFilesData>,
        editor_mapper: impl Fn(EditorMessage) -> Message + 'a,
        file_browser_mapper: impl Fn(FileBrowserMessage) -> Message + 'a,
        output_mapper: impl Fn(OutputMessage) -> Message + 'a,
        repl_mapper: impl Fn(ReplMessage) -> Message + 'a,
        data_explorer_mapper: impl Fn(DataExplorerMessage) -> Message + 'a,
        debug_panel_mapper: impl Fn(DebugPanelMessage) -> Message + 'a,
        welcome_handler: Option<impl Fn(WelcomeAction) -> Message + 'a>,
    ) -> Element<'a, Message> {
        match self.kind {
            PanelKind::FileBrowser => file_browser.view().map(file_browser_mapper),
            PanelKind::Editor => {
                // Show welcome screen if no tabs are open
                if editor.tabs.is_empty() && recent_data.is_some() && welcome_handler.is_some() {
                    let data = recent_data.unwrap();
                    let handler = welcome_handler.unwrap();
                    Self::render_welcome_screen(data, handler)
                } else {
                    editor.view().map(editor_mapper)
                }
            }
            PanelKind::Output => {
                // When debugging, show debug panel alongside output and REPL
                if is_debugging {
                    column![
                        debug_panel.view(debug_call_stack, debug_locals, is_debugging).map(debug_panel_mapper),
                        output.view().map(output_mapper),
                        repl.view().map(repl_mapper)
                    ]
                    .spacing(2)
                    .into()
                } else {
                    column![output.view().map(output_mapper), repl.view().map(repl_mapper)]
                        .spacing(2)
                        .into()
                }
            }
            PanelKind::DataExplorer => data_explorer.view().map(data_explorer_mapper),
        }
    }

    /// Render the welcome screen
    fn render_welcome_screen<'a, Message: Clone + 'a>(
        data: &RecentFilesData,
        handler: impl Fn(WelcomeAction) -> Message + 'a,
    ) -> Element<'a, Message> {
        let title = text("Welcome to Stratum Workshop")
            .size(24);

        let subtitle = text("Get started by creating a new file or opening an existing project")
            .size(14);

        // Quick actions
        let new_file = button(
            row![text("+ New File").size(14)]
                .align_y(Alignment::Center)
                .spacing(8)
        )
        .on_press(handler(WelcomeAction::NewFile))
        .padding([8, 16])
        .style(button::primary);

        let open_file = button(
            row![text("Open File").size(14)]
                .align_y(Alignment::Center)
                .spacing(8)
        )
        .on_press(handler(WelcomeAction::OpenFile))
        .padding([8, 16])
        .style(button::secondary);

        let open_folder = button(
            row![text("Open Folder").size(14)]
                .align_y(Alignment::Center)
                .spacing(8)
        )
        .on_press(handler(WelcomeAction::OpenFolder))
        .padding([8, 16])
        .style(button::secondary);

        let actions_row = row![new_file, open_file, open_folder]
            .spacing(12)
            .align_y(Alignment::Center);

        let mut content = Column::new()
            .push(title)
            .push(Space::new().height(8))
            .push(subtitle)
            .push(Space::new().height(24))
            .push(actions_row)
            .align_x(Alignment::Center)
            .spacing(4);

        // Recent files section
        if !data.recent_files.is_empty() {
            content = content
                .push(Space::new().height(32))
                .push(text("Recent Files").size(16));

            for path in data.recent_files.iter().take(5) {
                let label = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string());
                let path_str = path.display().to_string();
                let path_clone = path.clone();

                let item = button(
                    column![
                        text(label).size(13),
                        text(path_str).size(10),
                    ]
                    .spacing(2)
                )
                .on_press(handler(WelcomeAction::OpenRecentFile(path_clone)))
                .padding([6, 12])
                .width(Length::Fixed(400.0))
                .style(button::text);

                content = content.push(item);
            }
        }

        // Recent folders section
        if !data.recent_folders.is_empty() {
            content = content
                .push(Space::new().height(24))
                .push(text("Recent Folders").size(16));

            for path in data.recent_folders.iter().take(3) {
                let label = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string());
                let path_str = path.display().to_string();
                let path_clone = path.clone();

                let item = button(
                    column![
                        text(format!("[{}]", label)).size(13),
                        text(path_str).size(10),
                    ]
                    .spacing(2)
                )
                .on_press(handler(WelcomeAction::OpenRecentFolder(path_clone)))
                .padding([6, 12])
                .width(Length::Fixed(400.0))
                .style(button::text);

                content = content.push(item);
            }
        }

        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .padding(40)
            .into()
    }
}

/// Actions from the welcome screen
#[derive(Debug, Clone)]
pub enum WelcomeAction {
    NewFile,
    OpenFile,
    OpenFolder,
    OpenRecentFile(PathBuf),
    OpenRecentFolder(PathBuf),
}
