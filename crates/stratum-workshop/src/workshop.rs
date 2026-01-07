//! Main Workshop application
//!
//! Implements the IDE window with resizable pane layout.

use crate::config::{LayoutConfig, WorkshopConfig};
use crate::debug::{DebugSession, DebugSessionState};
use crate::execution::{build_source_async, execute_source_async, BuildResult, CancellationToken, ExecutionResult};
use crate::panels::{DataExplorerAction, DataExplorerMessage, DataExplorerPanel, DebugPanel, DebugPanelMessage, EditorMessage, EditorPanel, EditorTab, FileBrowserMessage, FileBrowserPanel, OutputMessage, OutputPanel, PaneContent, PanelKind, RecentFilesData, ReplMessage, ReplPanel, SourceLocation, WelcomeAction};
use iced::event::{self, Event};
use iced::keyboard;
use iced::keyboard::key;
use iced::mouse;
use iced::widget::pane_grid::{self, Axis, Configuration, DragEvent, Pane, PaneGrid, ResizeEvent, State};
use iced::widget::{button, center, column, container, mouse_area, opaque, row, rule, stack, text, Column, Space};
use iced::{Color, Element, Length, Subscription, Task, Theme};
use rfd::AsyncFileDialog;
use std::collections::HashMap;
use std::path::PathBuf;
use stratum_core::{DebugStackFrame, DebugVariable};

/// Main application state
pub struct Workshop {
    /// Pane grid state
    panes: State<PaneContent>,
    /// Pane identifiers for each panel type
    pane_ids: HashMap<PanelKind, Pane>,
    /// Focus tracking
    focus: Option<Pane>,
    /// Panel components
    pub editor: EditorPanel,
    pub file_browser: FileBrowserPanel,
    pub output: OutputPanel,
    pub repl: ReplPanel,
    pub data_explorer: DataExplorerPanel,
    /// Configuration
    config: WorkshopConfig,
    /// Current application state
    state: WorkshopState,
    /// Modal dialog state
    modal: Option<ModalState>,
    /// Cancellation token for stopping execution
    cancellation_token: CancellationToken,
    /// Run arguments (passed to main())
    run_args: String,
    /// Debug session
    debug_session: DebugSession,
    /// Debug state (call stack and locals when paused)
    debug_call_stack: Vec<DebugStackFrame>,
    debug_locals: Vec<DebugVariable>,
    /// Debug panel
    debug_panel: DebugPanel,
    /// Currently open menu dropdown
    active_menu: Option<MenuKind>,
}

/// State for modal dialogs
#[derive(Debug, Clone)]
pub enum ModalState {
    /// Confirm closing an unsaved tab
    CloseConfirm {
        tab_index: usize,
        file_name: String,
    },
    /// About dialog
    About,
}

/// Menu bar dropdown menus
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuKind {
    File,
    Edit,
    Run,
    View,
    Help,
}

/// High-level application state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkshopState {
    Idle,
    Running,
    Debugging,
}

/// Messages for Workshop application
#[derive(Debug, Clone)]
pub enum WorkshopMessage {
    // Pane management
    PaneClicked(Pane),
    PaneDragged(DragEvent),
    PaneResized(ResizeEvent),

    // Panel visibility
    ToggleFileBrowser,
    ToggleOutput,

    // File operations
    NewFile,
    OpenFile,
    OpenFolder,
    SaveFile,
    CloseTab(usize),
    CloseCurrentTab,

    // Editor operations
    TabSelected(usize),
    Editor(EditorMessage),

    // File browser operations
    FileBrowser(FileBrowserMessage),

    // REPL operations
    Repl(ReplMessage),

    // Output panel operations
    Output(OutputMessage),

    // Tab navigation
    NextTab,
    PreviousTab,

    // Run operations
    Run,
    Stop,
    Build,
    RunArgsChanged(String),

    // Debug operations
    Debug,
    DebugContinue,
    DebugStepInto,
    DebugStepOver,
    DebugStepOut,
    DebugStop,

    // Internal
    FileOpened(PathBuf, String),
    FolderOpened(PathBuf),
    RunOutput(String),
    RunError(String),
    RunComplete(ExecutionResult),
    BuildComplete(BuildResult),

    // Debug internal
    DebugPaused {
        file: Option<PathBuf>,
        line: u32,
        function_name: String,
        call_stack: Vec<DebugStackFrame>,
        locals: Vec<DebugVariable>,
        reason: String,
    },
    DebugCompleted(Option<String>),
    DebugError(String),

    // Modal dialog
    ModalSave,
    ModalDiscard,
    ModalCancel,

    // Mouse events for tab dragging
    MouseEvent(Event),

    // Menu bar
    MenuOpen(MenuKind),
    MenuClose,

    // File menu actions
    SaveFileAs,
    Exit,
    FileSaved(PathBuf),
    FileSaveError(String),

    // File dialog results
    FileDialogOpened(Option<(PathBuf, String)>),
    FolderDialogOpened(Option<PathBuf>),
    SaveDialogCompleted(Option<PathBuf>),

    // Edit menu actions
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Find,
    Replace,

    // View menu actions
    ToggleRepl,
    ToggleDataExplorer,

    // Data Explorer operations
    DataExplorer(DataExplorerMessage),

    // Help menu actions
    ShowAbout,

    // Recent files
    OpenRecentFile(PathBuf),
    OpenRecentFolder(PathBuf),
    ClearRecentFiles,

    // Welcome screen actions
    Welcome(WelcomeAction),
}

impl Default for Workshop {
    fn default() -> Self {
        Self::new()
    }
}

impl Workshop {
    /// Create a new Workshop instance
    pub fn new() -> Self {
        let config = WorkshopConfig::load();

        // Build initial pane configuration
        let (panes, pane_ids) = Self::create_pane_layout(&config.layout);

        Self {
            panes,
            pane_ids,
            focus: None,
            editor: EditorPanel::new(),
            file_browser: FileBrowserPanel::new(),
            output: OutputPanel::new(),
            repl: ReplPanel::new(),
            data_explorer: DataExplorerPanel::new(),
            config,
            state: WorkshopState::Idle,
            modal: None,
            cancellation_token: CancellationToken::new(),
            run_args: String::new(),
            debug_session: DebugSession::new(),
            debug_call_stack: Vec::new(),
            debug_locals: Vec::new(),
            debug_panel: DebugPanel::new(),
            active_menu: None,
        }
    }

    /// Create the pane layout based on visibility settings
    ///
    /// Layout structure:
    /// ```text
    /// ┌────────────┬────────────────────────────┬──────────────┐
    /// │            │     Editor Area            │              │
    /// │   File     ├────────────────────────────┤    Data      │
    /// │  Browser   │     Output / REPL          │   Explorer   │
    /// └────────────┴────────────────────────────┴──────────────┘
    /// ```
    fn create_pane_layout(layout: &LayoutConfig) -> (State<PaneContent>, HashMap<PanelKind, Pane>) {
        let pane_ids = HashMap::new();

        // Build the main area (editor + output)
        let main_area = if layout.visibility.output {
            Configuration::Split {
                axis: Axis::Horizontal,
                ratio: 1.0 - layout.output_ratio,
                a: Box::new(Configuration::Pane(PaneContent::new(PanelKind::Editor))),
                b: Box::new(Configuration::Pane(PaneContent::new(PanelKind::Output))),
            }
        } else {
            Configuration::Pane(PaneContent::new(PanelKind::Editor))
        };

        // Add data explorer on the right if visible
        let main_with_explorer = if layout.visibility.data_explorer {
            Configuration::Split {
                axis: Axis::Vertical,
                ratio: 1.0 - layout.data_explorer_ratio,
                a: Box::new(main_area),
                b: Box::new(Configuration::Pane(PaneContent::new(PanelKind::DataExplorer))),
            }
        } else {
            main_area
        };

        // Add file browser on the left if visible
        let config = if layout.visibility.file_browser {
            Configuration::Split {
                axis: Axis::Vertical,
                ratio: layout.file_browser_ratio,
                a: Box::new(Configuration::Pane(PaneContent::new(PanelKind::FileBrowser))),
                b: Box::new(main_with_explorer),
            }
        } else {
            main_with_explorer
        };

        let state = State::with_configuration(config);

        // Map pane IDs to panel kinds
        let mut pane_ids = pane_ids;
        for (pane, content) in state.iter() {
            pane_ids.insert(content.kind, *pane);
        }

        (state, pane_ids)
    }

    /// Rebuild the pane layout (used when toggling panels)
    fn rebuild_layout(&mut self) {
        let (panes, pane_ids) = Self::create_pane_layout(&self.config.layout);
        self.panes = panes;
        self.pane_ids = pane_ids;
    }

    /// Refresh the Data Explorer with current REPL globals
    fn refresh_data_explorer(&mut self) {
        // Get globals from the REPL's VM
        let globals = self.repl.get_globals();
        self.data_explorer.update_from_globals(&globals);
    }

    /// Get the title for the window
    pub fn title(&self) -> String {
        let file_name = self
            .editor
            .active()
            .and_then(|tab| tab.path.as_ref())
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Untitled".to_string());

        format!("{} - Stratum Workshop", file_name)
    }

    /// Jump to a source location in the editor
    fn jump_to_source(&mut self, location: &SourceLocation) {
        // First, try to find an already open tab with this file
        for (i, tab) in self.editor.tabs.iter().enumerate() {
            if let Some(path) = &tab.path {
                if path.file_name().map(|n| n.to_string_lossy()) == Some(location.file.clone().into()) {
                    // Found the tab, switch to it
                    self.editor.active_tab = i;
                    // TODO: Move cursor to the specific line/column
                    // For now, we just switch to the tab
                    return;
                }
            }
        }

        // If not found, try to open the file from the current folder
        if let Some(root) = &self.file_browser.root {
            let file_path = root.join(&location.file);
            if file_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&file_path) {
                    self.editor.tabs.push(EditorTab::from_file(file_path.clone(), content));
                    self.editor.active_tab = self.editor.tabs.len() - 1;
                    // TODO: Move cursor to the specific line/column
                }
            }
        }
    }

    /// Handle messages
    pub fn update(&mut self, message: WorkshopMessage) -> Task<WorkshopMessage> {
        match message {
            WorkshopMessage::PaneClicked(pane) => {
                self.focus = Some(pane);
            }

            WorkshopMessage::PaneDragged(DragEvent::Dropped { pane, target }) => {
                self.panes.drop(pane, target);
            }

            WorkshopMessage::PaneDragged(_) => {}

            WorkshopMessage::PaneResized(ResizeEvent { split, ratio }) => {
                self.panes.resize(split, ratio);

                // Update config with new ratios
                // This is a simplification - in practice we'd need to track which split is which
                if let Some(pane) = self.pane_ids.get(&PanelKind::FileBrowser) {
                    if self.panes.adjacent(*pane, pane_grid::Direction::Right).is_some() {
                        self.config.layout.file_browser_ratio = ratio;
                    }
                }
            }

            WorkshopMessage::ToggleFileBrowser => {
                self.config.layout.visibility.file_browser = !self.config.layout.visibility.file_browser;
                self.rebuild_layout();
                let _ = self.config.save();
            }

            WorkshopMessage::ToggleOutput => {
                self.config.layout.visibility.output = !self.config.layout.visibility.output;
                self.rebuild_layout();
                let _ = self.config.save();
            }

            WorkshopMessage::NewFile => {
                self.editor.new_tab();
            }

            WorkshopMessage::OpenFile => {
                self.active_menu = None;
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

            WorkshopMessage::OpenFolder => {
                self.active_menu = None;
                return Task::perform(
                    async {
                        let folder = AsyncFileDialog::new()
                            .set_title("Open Folder")
                            .pick_folder()
                            .await;

                        folder.map(|handle| handle.path().to_path_buf())
                    },
                    WorkshopMessage::FolderDialogOpened,
                );
            }

            WorkshopMessage::SaveFile => {
                if let Some(tab) = self.editor.active() {
                    if let Some(path) = &tab.path {
                        // File has a path - save directly
                        let content = tab.text();
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
                        // No path - show Save As dialog
                        let content = tab.text();
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
                                    WorkshopMessage::FileSaveError("Save cancelled".to_string())
                                }
                            },
                            |msg| msg,
                        );
                    }
                }
            }

            WorkshopMessage::CloseTab(index) => {
                // Check if tab has unsaved changes
                if let Some(tab) = self.editor.tabs.get(index) {
                    if tab.modified {
                        // Show confirmation dialog
                        self.modal = Some(ModalState::CloseConfirm {
                            tab_index: index,
                            file_name: tab.name(),
                        });
                    } else {
                        self.editor.close_tab(index);
                    }
                }
            }

            WorkshopMessage::CloseCurrentTab => {
                let index = self.editor.active_tab;
                // Check if tab has unsaved changes
                if let Some(tab) = self.editor.tabs.get(index) {
                    if tab.modified {
                        // Show confirmation dialog
                        self.modal = Some(ModalState::CloseConfirm {
                            tab_index: index,
                            file_name: tab.name(),
                        });
                    } else {
                        self.editor.close_tab(index);
                    }
                }
            }

            WorkshopMessage::TabSelected(index) => {
                if index < self.editor.tabs.len() {
                    self.editor.active_tab = index;
                }
            }

            WorkshopMessage::NextTab => {
                if !self.editor.tabs.is_empty() {
                    self.editor.active_tab = (self.editor.active_tab + 1) % self.editor.tabs.len();
                }
            }

            WorkshopMessage::PreviousTab => {
                if !self.editor.tabs.is_empty() {
                    self.editor.active_tab = if self.editor.active_tab == 0 {
                        self.editor.tabs.len() - 1
                    } else {
                        self.editor.active_tab - 1
                    };
                }
            }

            WorkshopMessage::Editor(msg) => {
                self.editor.update(msg);
            }

            WorkshopMessage::FileBrowser(msg) => {
                // Handle file browser message and check if a file should be opened
                if let Some(path) = self.file_browser.update(msg) {
                    // File was activated - open it in the editor
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        self.editor.tabs.push(EditorTab::from_file(path.clone(), content));
                        self.editor.active_tab = self.editor.tabs.len() - 1;
                        self.config.add_recent_file(path);
                        let _ = self.config.save();
                    }
                }
            }

            WorkshopMessage::Repl(msg) => {
                // Check if this is a submit that might define new variables
                let is_submit = matches!(msg, ReplMessage::Submit);
                self.repl.update(msg);

                // Refresh Data Explorer after REPL evaluation
                if is_submit {
                    self.refresh_data_explorer();
                }
            }

            WorkshopMessage::DataExplorer(msg) => {
                if let Some(action) = self.data_explorer.update(msg) {
                    match action {
                        DataExplorerAction::RequestRefresh => {
                            self.refresh_data_explorer();
                        }
                        DataExplorerAction::PrintInRepl(path) => {
                            // Send the variable name to REPL for printing
                            self.repl.update(ReplMessage::InputChanged(path));
                            self.repl.update(ReplMessage::Submit);
                            self.refresh_data_explorer();
                        }
                        DataExplorerAction::CopyToClipboard(_path) => {
                            // TODO: Implement clipboard integration
                            self.output.info("Copy to clipboard not yet implemented".to_string());
                        }
                    }
                }
            }

            WorkshopMessage::Output(msg) => {
                // Handle output panel messages
                if let Some(location) = self.output.update(msg) {
                    // User clicked on an error - jump to source location
                    self.jump_to_source(&location);
                }
            }

            WorkshopMessage::FileOpened(path, content) => {
                self.editor.tabs.push(EditorTab::from_file(path.clone(), content));
                self.editor.active_tab = self.editor.tabs.len() - 1;
                self.config.add_recent_file(path);
                let _ = self.config.save();
            }

            WorkshopMessage::FolderOpened(path) => {
                let _ = self.file_browser.open_folder(path.clone());
                self.config.add_recent_folder(path);
                let _ = self.config.save();
            }

            WorkshopMessage::Run => {
                // Get the current file content
                if let Some(tab) = self.editor.active() {
                    let source = tab.text();
                    let file_path = tab.path.clone();
                    let args = self.run_args.clone();

                    // Create a new cancellation token for this run
                    self.cancellation_token = CancellationToken::new();
                    let token = self.cancellation_token.clone();

                    self.state = WorkshopState::Running;
                    self.output.clear();
                    if args.is_empty() {
                        self.output.info("Running...".to_string());
                    } else {
                        self.output.info(format!("Running with args: {args}"));
                    }

                    // Execute asynchronously
                    return Task::perform(
                        execute_source_async(source, file_path, args, token),
                        WorkshopMessage::RunComplete,
                    );
                } else {
                    self.output.info("No file open to run.".to_string());
                }
            }

            WorkshopMessage::Stop => {
                // Signal cancellation
                self.cancellation_token.cancel();
                self.state = WorkshopState::Idle;
                self.output.info("Execution stopped.".to_string());
            }

            WorkshopMessage::Build => {
                if let Some(tab) = self.editor.active() {
                    // Need a file path to build - unsaved files can't be built
                    if let Some(file_path) = tab.path.clone() {
                        let source = tab.text();

                        self.output.clear();
                        self.output.info("Building...".to_string());

                        // Build asynchronously (release mode = false for now)
                        return Task::perform(
                            build_source_async(source, file_path, false),
                            WorkshopMessage::BuildComplete,
                        );
                    } else {
                        self.output.clear();
                        self.output.stderr("Cannot build: File must be saved first.".to_string());
                    }
                } else {
                    self.output.clear();
                    self.output.stderr("Cannot build: No file open.".to_string());
                }
            }

            WorkshopMessage::RunArgsChanged(args) => {
                self.run_args = args;
            }

            WorkshopMessage::RunOutput(text) => {
                self.output.stdout(text);
            }

            WorkshopMessage::RunError(text) => {
                self.output.stderr(text);
            }

            WorkshopMessage::RunComplete(result) => {
                self.state = WorkshopState::Idle;

                // Display captured stdout
                for line in &result.stdout {
                    self.output.stdout(line.clone());
                }

                // Display return value if any
                if let Some(return_value) = &result.return_value {
                    self.output.info(format!("=> {return_value}"));
                }

                // Display errors if any
                for error in &result.errors {
                    self.output.stderr(error.clone());
                }

                // Show final status
                if result.success {
                    self.output.success("Program exited successfully.".to_string());
                } else {
                    self.output.stderr("Program exited with errors.".to_string());
                }
            }

            WorkshopMessage::BuildComplete(result) => {
                // Display build log messages
                for message in &result.messages {
                    self.output.info(message.clone());
                }

                // Display errors if any
                for error in &result.errors {
                    self.output.stderr(error.clone());
                }

                // Show final status
                if result.success {
                    if let Some(output_path) = &result.output_path {
                        self.output.success(format!("Build successful: {}", output_path.display()));
                    } else {
                        self.output.success("Build successful.".to_string());
                    }
                } else {
                    self.output.stderr("Build failed.".to_string());
                }
            }

            // Debug operations
            WorkshopMessage::Debug => {
                if let Some(tab) = self.editor.active() {
                    let source = tab.text();
                    let file_path = tab.path.clone();

                    // Collect breakpoints from editor
                    let breakpoints: Vec<(u32, Option<PathBuf>)> = self.editor.get_breakpoints()
                        .iter()
                        .map(|&line| (line as u32, file_path.clone()))
                        .collect();

                    self.state = WorkshopState::Debugging;
                    self.output.clear();
                    self.output.info("Starting debug session...".to_string());

                    // Start debug session
                    let result = self.debug_session.start(&source, file_path.clone(), &breakpoints);
                    return self.handle_debug_result(result);
                } else {
                    self.output.info("No file open to debug.".to_string());
                }
            }

            WorkshopMessage::DebugContinue => {
                if self.state == WorkshopState::Debugging {
                    let result = self.debug_session.continue_execution();
                    return self.handle_debug_result(result);
                }
            }

            WorkshopMessage::DebugStepInto => {
                if self.state == WorkshopState::Debugging {
                    let result = self.debug_session.step_into();
                    return self.handle_debug_result(result);
                }
            }

            WorkshopMessage::DebugStepOver => {
                if self.state == WorkshopState::Debugging {
                    let result = self.debug_session.step_over();
                    return self.handle_debug_result(result);
                }
            }

            WorkshopMessage::DebugStepOut => {
                if self.state == WorkshopState::Debugging {
                    let result = self.debug_session.step_out();
                    return self.handle_debug_result(result);
                }
            }

            WorkshopMessage::DebugStop => {
                self.debug_session.stop();
                self.state = WorkshopState::Idle;
                self.editor.set_debug_line(None);
                self.debug_call_stack.clear();
                self.debug_locals.clear();
                self.output.info("Debug session stopped.".to_string());
            }

            WorkshopMessage::DebugPaused { file, line, function_name, call_stack, locals, reason } => {
                self.output.info(format!("Paused at {} line {} ({})", function_name, line, reason));
                self.editor.set_debug_line(Some(line as usize));
                self.debug_call_stack = call_stack;
                self.debug_locals = locals;
            }

            WorkshopMessage::DebugCompleted(result) => {
                self.state = WorkshopState::Idle;
                self.editor.set_debug_line(None);
                self.debug_call_stack.clear();
                self.debug_locals.clear();
                if let Some(value) = result {
                    self.output.info(format!("=> {value}"));
                }
                self.output.success("Debug session completed.".to_string());
            }

            WorkshopMessage::DebugError(error) => {
                self.state = WorkshopState::Idle;
                self.editor.set_debug_line(None);
                self.debug_call_stack.clear();
                self.debug_locals.clear();
                self.output.stderr(format!("Debug error: {error}"));
            }

            WorkshopMessage::ModalSave => {
                // Save and close - for now just close since save isn't implemented
                if let Some(ModalState::CloseConfirm { tab_index, .. }) = self.modal.take() {
                    // TODO: Actually save the file first
                    self.editor.close_tab(tab_index);
                }
            }

            WorkshopMessage::ModalDiscard => {
                // Discard changes and close
                if let Some(ModalState::CloseConfirm { tab_index, .. }) = self.modal.take() {
                    self.editor.close_tab(tab_index);
                }
            }

            WorkshopMessage::ModalCancel => {
                // Close modal if open
                self.modal = None;
                // Close menu if open
                self.active_menu = None;
                // Close find bar if open
                if self.editor.find_state.visible {
                    self.editor.update(EditorMessage::CloseFindBar);
                }
                // Also cancel any active drag
                if self.editor.drag_state.is_some() {
                    self.editor.update(EditorMessage::TabDragCancel);
                }
            }

            WorkshopMessage::MouseEvent(event) => {
                // Handle mouse events for tab dragging
                if self.editor.drag_state.is_some() {
                    match event {
                        Event::Mouse(mouse::Event::CursorMoved { position }) => {
                            self.editor.update(EditorMessage::TabDragMove(position.x));
                        }
                        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                            self.editor.update(EditorMessage::TabDragEnd);
                        }
                        _ => {}
                    }
                }

                // Handle click anywhere to close context menu
                if self.editor.context_menu.is_some() {
                    if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
                        self.editor.update(EditorMessage::CloseContextMenu);
                    }
                }

                // Also close file browser context menu on click elsewhere
                if self.file_browser.context_menu.is_some() {
                    if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
                        self.file_browser.update(FileBrowserMessage::CloseContextMenu);
                    }
                }

                // Close menu dropdown on click elsewhere
                if self.active_menu.is_some() {
                    if let Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) = event {
                        self.active_menu = None;
                    }
                }
            }

            // Menu bar
            WorkshopMessage::MenuOpen(menu) => {
                // Toggle: if same menu clicked, close it; otherwise open new one
                if self.active_menu == Some(menu) {
                    self.active_menu = None;
                } else {
                    self.active_menu = Some(menu);
                }
            }

            WorkshopMessage::MenuClose => {
                self.active_menu = None;
            }

            // File menu actions
            WorkshopMessage::SaveFileAs => {
                self.active_menu = None;
                if let Some(tab) = self.editor.active() {
                    let content = tab.text();
                    let default_name = tab.path
                        .as_ref()
                        .and_then(|p| p.file_name())
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "untitled.strat".to_string());

                    return Task::perform(
                        async move {
                            let file = AsyncFileDialog::new()
                                .add_filter("Stratum", &["strat"])
                                .add_filter("All files", &["*"])
                                .set_title("Save As")
                                .set_file_name(&default_name)
                                .save_file()
                                .await;

                            if let Some(handle) = file {
                                let path = handle.path().to_path_buf();
                                match tokio::fs::write(&path, &content).await {
                                    Ok(()) => WorkshopMessage::FileSaved(path),
                                    Err(e) => WorkshopMessage::FileSaveError(e.to_string()),
                                }
                            } else {
                                WorkshopMessage::FileSaveError("Save cancelled".to_string())
                            }
                        },
                        |msg| msg,
                    );
                }
            }

            WorkshopMessage::FileSaved(path) => {
                if let Some(tab) = self.editor.active_mut() {
                    tab.modified = false;
                    tab.path = Some(path.clone());
                }
                self.output.success(format!("File saved: {}", path.display()));
            }

            WorkshopMessage::FileSaveError(error) => {
                self.output.stderr(format!("Failed to save file: {error}"));
            }

            WorkshopMessage::Exit => {
                self.active_menu = None;
                // TODO: Check for unsaved changes before exiting
                std::process::exit(0);
            }

            // Edit menu actions
            // Note: iced's text_editor doesn't have built-in Undo/Redo/Cut/Copy
            // These are placeholder handlers - full implementation requires custom editor state
            WorkshopMessage::Undo => {
                self.active_menu = None;
                // TODO: Implement undo when custom editor history is added
                self.output.info("Undo not yet implemented for text editor".to_string());
            }

            WorkshopMessage::Redo => {
                self.active_menu = None;
                // TODO: Implement redo when custom editor history is added
                self.output.info("Redo not yet implemented for text editor".to_string());
            }

            WorkshopMessage::Cut => {
                self.active_menu = None;
                // TODO: Implement cut with clipboard
                self.output.info("Cut not yet implemented for text editor".to_string());
            }

            WorkshopMessage::Copy => {
                self.active_menu = None;
                // TODO: Implement copy with clipboard
                self.output.info("Copy not yet implemented for text editor".to_string());
            }

            WorkshopMessage::Paste => {
                self.active_menu = None;
                // TODO: Implement paste with clipboard
                self.output.info("Paste not yet implemented for text editor".to_string());
            }

            WorkshopMessage::Find => {
                self.active_menu = None;
                self.editor.toggle_find_bar();
            }

            WorkshopMessage::Replace => {
                self.active_menu = None;
                self.editor.toggle_replace_bar();
            }

            // View menu actions
            WorkshopMessage::ToggleRepl => {
                self.active_menu = None;
                self.config.layout.visibility.repl = !self.config.layout.visibility.repl;
                self.rebuild_layout();
                let _ = self.config.save();
            }

            WorkshopMessage::ToggleDataExplorer => {
                self.active_menu = None;
                self.config.layout.visibility.data_explorer = !self.config.layout.visibility.data_explorer;
                self.rebuild_layout();
                // Refresh data explorer when showing it
                if self.config.layout.visibility.data_explorer {
                    self.refresh_data_explorer();
                }
                let _ = self.config.save();
            }

            // Help menu actions
            WorkshopMessage::ShowAbout => {
                self.active_menu = None;
                self.modal = Some(ModalState::About);
            }

            // Recent files
            WorkshopMessage::OpenRecentFile(path) => {
                self.active_menu = None;
                if path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        self.editor.tabs.push(EditorTab::from_file(path.clone(), content));
                        self.editor.active_tab = self.editor.tabs.len() - 1;
                        self.config.add_recent_file(path);
                        let _ = self.config.save();
                    }
                } else {
                    self.output.stderr(format!("File not found: {}", path.display()));
                }
            }

            WorkshopMessage::OpenRecentFolder(path) => {
                self.active_menu = None;
                if path.exists() && path.is_dir() {
                    let _ = self.file_browser.open_folder(path.clone());
                    self.config.add_recent_folder(path);
                    let _ = self.config.save();
                } else {
                    self.output.stderr(format!("Folder not found: {}", path.display()));
                }
            }

            WorkshopMessage::ClearRecentFiles => {
                self.active_menu = None;
                self.config.recent_files.clear();
                self.config.recent_folders.clear();
                let _ = self.config.save();
            }

            // Welcome screen actions
            WorkshopMessage::Welcome(action) => {
                match action {
                    WelcomeAction::NewFile => {
                        self.editor.new_tab();
                    }
                    WelcomeAction::OpenFile => {
                        // Reuse the OpenFile handler
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
                    WelcomeAction::OpenFolder => {
                        // Reuse the OpenFolder handler
                        return Task::perform(
                            async {
                                let folder = AsyncFileDialog::new()
                                    .set_title("Open Folder")
                                    .pick_folder()
                                    .await;

                                folder.map(|handle| handle.path().to_path_buf())
                            },
                            WorkshopMessage::FolderDialogOpened,
                        );
                    }
                    WelcomeAction::OpenRecentFile(path) => {
                        if path.exists() {
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                self.editor.tabs.push(EditorTab::from_file(path.clone(), content));
                                self.editor.active_tab = self.editor.tabs.len() - 1;
                                self.config.add_recent_file(path);
                                let _ = self.config.save();
                            }
                        } else {
                            self.output.stderr(format!("File not found: {}", path.display()));
                        }
                    }
                    WelcomeAction::OpenRecentFolder(path) => {
                        if path.exists() && path.is_dir() {
                            let _ = self.file_browser.open_folder(path.clone());
                            self.config.add_recent_folder(path);
                            let _ = self.config.save();
                        } else {
                            self.output.stderr(format!("Folder not found: {}", path.display()));
                        }
                    }
                }
            }

            // File dialog results
            WorkshopMessage::FileDialogOpened(result) => {
                if let Some((path, content)) = result {
                    self.editor.tabs.push(EditorTab::from_file(path.clone(), content));
                    self.editor.active_tab = self.editor.tabs.len() - 1;
                    self.config.add_recent_file(path);
                    let _ = self.config.save();
                }
            }

            WorkshopMessage::FolderDialogOpened(result) => {
                if let Some(path) = result {
                    let _ = self.file_browser.open_folder(path.clone());
                    self.config.add_recent_folder(path);
                    let _ = self.config.save();
                }
            }

            WorkshopMessage::SaveDialogCompleted(result) => {
                if let Some(path) = result {
                    if let Some(tab) = self.editor.active_mut() {
                        tab.path = Some(path.clone());
                        tab.modified = false;
                    }
                    self.output.success(format!("File saved: {}", path.display()));
                }
            }
        }

        Task::none()
    }

    /// Render the application
    pub fn view(&self) -> Element<WorkshopMessage> {
        let menu_bar = self.menu_bar();
        let toolbar = self.toolbar();

        // Prepare recent files data for welcome screen
        let recent_data = RecentFilesData {
            recent_files: self.config.recent_files.clone(),
            recent_folders: self.config.recent_folders.clone(),
        };

        let pane_grid: Element<WorkshopMessage> = PaneGrid::new(&self.panes, |_pane, content, _is_maximized| {
            let header = content.header();
            let body = content.content(
                &self.editor,
                &self.file_browser,
                &self.output,
                &self.repl,
                &self.data_explorer,
                Some(&recent_data),
                WorkshopMessage::Editor,
                WorkshopMessage::FileBrowser,
                WorkshopMessage::Output,
                WorkshopMessage::Repl,
                WorkshopMessage::DataExplorer,
                Some(WorkshopMessage::Welcome),
            );

            pane_grid::Content::new(column![header, body])
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(2)
        .on_click(WorkshopMessage::PaneClicked)
        .on_drag(WorkshopMessage::PaneDragged)
        .on_resize(6, WorkshopMessage::PaneResized)
        .into();

        let base_content: Element<WorkshopMessage> = container(
            column![menu_bar, toolbar, rule::horizontal(1), pane_grid]
                .spacing(0)
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into();

        // Render modal if present
        if let Some(modal_state) = &self.modal {
            let dialog = self.render_modal(modal_state);
            Self::modal_overlay(base_content, dialog)
        } else {
            base_content
        }
    }

    /// Render modal dialog content
    fn render_modal(&self, modal_state: &ModalState) -> Element<WorkshopMessage> {
        match modal_state {
            ModalState::CloseConfirm { file_name, .. } => {
                let dialog_content = column![
                    text("Unsaved Changes").size(18),
                    text(format!("\"{}\" has unsaved changes.", file_name)).size(14),
                    text("What would you like to do?").size(14),
                    row![
                        button(text("Save").size(12))
                            .on_press(WorkshopMessage::ModalSave)
                            .style(button::primary)
                            .padding([6, 12]),
                        button(text("Discard").size(12))
                            .on_press(WorkshopMessage::ModalDiscard)
                            .style(button::danger)
                            .padding([6, 12]),
                        button(text("Cancel").size(12))
                            .on_press(WorkshopMessage::ModalCancel)
                            .style(button::secondary)
                            .padding([6, 12]),
                    ]
                    .spacing(8)
                ]
                .spacing(12)
                .padding(20);

                container(dialog_content)
                    .style(container::rounded_box)
                    .into()
            }
            ModalState::About => {
                let dialog_content = column![
                    text("Stratum Workshop").size(20),
                    text("Version 0.1.0").size(14),
                    text("").size(8),
                    text("A lightweight IDE for the Stratum programming language.").size(12),
                    text("Built with Rust and iced.").size(12),
                    text("").size(8),
                    text("© 2024 Horizon Analytic Studios, LLC").size(11),
                    text("").size(12),
                    button(text("OK").size(12))
                        .on_press(WorkshopMessage::ModalCancel)
                        .style(button::primary)
                        .padding([6, 16]),
                ]
                .spacing(4)
                .padding(24)
                .align_x(iced::Alignment::Center);

                container(dialog_content)
                    .style(container::rounded_box)
                    .into()
            }
        }
    }

    /// Create modal overlay with backdrop
    fn modal_overlay<'a>(
        base: Element<'a, WorkshopMessage>,
        dialog: Element<'a, WorkshopMessage>,
    ) -> Element<'a, WorkshopMessage> {
        let backdrop = container(center(opaque(dialog)))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme: &Theme| container::Style {
                background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.5).into()),
                ..Default::default()
            });

        stack![
            base,
            opaque(mouse_area(backdrop).on_press(WorkshopMessage::ModalCancel))
        ]
        .into()
    }

    /// Render the menu bar with dropdown menus
    fn menu_bar(&self) -> Element<WorkshopMessage> {
        // Menu title buttons
        let file_title = Self::menu_title("File", MenuKind::File, self.active_menu);
        let edit_title = Self::menu_title("Edit", MenuKind::Edit, self.active_menu);
        let run_title = Self::menu_title("Run", MenuKind::Run, self.active_menu);
        let view_title = Self::menu_title("View", MenuKind::View, self.active_menu);
        let help_title = Self::menu_title("Help", MenuKind::Help, self.active_menu);

        let menu_titles = row![file_title, edit_title, run_title, view_title, help_title]
            .spacing(1)
            .align_y(iced::Alignment::Center);

        let menu_bar_content = container(menu_titles)
            .padding([2, 4])
            .width(Length::Fill)
            .style(|theme: &Theme| {
                let palette = theme.extended_palette();
                container::Style {
                    background: Some(palette.background.weak.color.into()),
                    ..Default::default()
                }
            });

        // If a menu is open, render it as overlay
        if let Some(menu_kind) = self.active_menu {
            let dropdown = self.render_dropdown(menu_kind);
            stack![menu_bar_content, dropdown].into()
        } else {
            menu_bar_content.into()
        }
    }

    /// Create a menu title button
    fn menu_title(label: &'static str, kind: MenuKind, active: Option<MenuKind>) -> Element<'static, WorkshopMessage> {
        let is_active = active == Some(kind);
        let style = if is_active { button::primary } else { button::text };

        button(text(label).size(12))
            .on_press(WorkshopMessage::MenuOpen(kind))
            .padding([4, 10])
            .style(style)
            .into()
    }

    /// Render a dropdown menu based on the menu kind
    fn render_dropdown(&self, kind: MenuKind) -> Element<WorkshopMessage> {
        let (items, offset) = match kind {
            MenuKind::File => (self.file_menu_items(), 0.0),
            MenuKind::Edit => (self.edit_menu_items(), 50.0),
            MenuKind::Run => (self.run_menu_items(), 100.0),
            MenuKind::View => (self.view_menu_items(), 140.0),
            MenuKind::Help => (self.help_menu_items(), 190.0),
        };

        let menu_content = container(items)
            .style(|theme: &Theme| {
                let palette = theme.extended_palette();
                container::Style {
                    background: Some(palette.background.strong.color.into()),
                    border: iced::Border {
                        color: palette.background.weak.color,
                        width: 1.0,
                        radius: 4.0.into(),
                    },
                    shadow: iced::Shadow {
                        color: Color::from_rgba(0.0, 0.0, 0.0, 0.3),
                        offset: iced::Vector::new(2.0, 2.0),
                        blur_radius: 4.0,
                    },
                    ..Default::default()
                }
            })
            .padding(4);

        // Position the dropdown below the menu bar
        column![
            Space::new().height(24.0),
            row![
                Space::new().width(offset),
                menu_content,
                Space::new().width(Length::Fill),
            ]
        ]
        .into()
    }

    /// File menu items
    fn file_menu_items(&self) -> Column<'static, WorkshopMessage> {
        let mut col = Column::new()
            .push(Self::menu_item("New", "Ctrl+N", WorkshopMessage::NewFile))
            .push(Self::menu_item("Open...", "Ctrl+O", WorkshopMessage::OpenFile))
            .push(Self::menu_item("Open Folder...", "", WorkshopMessage::OpenFolder))
            .push(Self::menu_separator());

        // Add recent files submenu header if there are recent files
        if !self.config.recent_files.is_empty() || !self.config.recent_folders.is_empty() {
            col = col.push(Self::recent_files_header());

            // Add recent files (up to 5)
            for path in self.config.recent_files.iter().take(5) {
                let label = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string());
                col = col.push(Self::recent_item(label, WorkshopMessage::OpenRecentFile(path.clone())));
            }

            // Add separator between files and folders if both exist
            if !self.config.recent_files.is_empty() && !self.config.recent_folders.is_empty() {
                col = col.push(Self::menu_separator());
            }

            // Add recent folders (up to 3)
            for path in self.config.recent_folders.iter().take(3) {
                let label = path
                    .file_name()
                    .map(|n| format!("[{}]", n.to_string_lossy()))
                    .unwrap_or_else(|| format!("[{}]", path.display()));
                col = col.push(Self::recent_item(label, WorkshopMessage::OpenRecentFolder(path.clone())));
            }

            // Clear recent option
            col = col.push(Self::menu_separator());
            col = col.push(Self::menu_item("Clear Recent", "", WorkshopMessage::ClearRecentFiles));
            col = col.push(Self::menu_separator());
        }

        col.push(Self::menu_item("Save", "Ctrl+S", WorkshopMessage::SaveFile))
            .push(Self::menu_item("Save As...", "Ctrl+Shift+S", WorkshopMessage::SaveFileAs))
            .push(Self::menu_separator())
            .push(Self::menu_item("Close", "Ctrl+W", WorkshopMessage::CloseCurrentTab))
            .push(Self::menu_separator())
            .push(Self::menu_item("Exit", "Ctrl+Q", WorkshopMessage::Exit))
            .spacing(2)
            .width(Length::Fixed(220.0))
    }

    /// Recent files section header
    fn recent_files_header() -> Element<'static, WorkshopMessage> {
        container(text("Recent").size(11))
            .padding([4, 8])
            .width(Length::Fill)
            .style(|theme: &Theme| {
                let palette = theme.extended_palette();
                container::Style {
                    text_color: Some(palette.primary.base.text),
                    ..Default::default()
                }
            })
            .into()
    }

    /// Create a recent file/folder menu item
    fn recent_item(label: String, message: WorkshopMessage) -> Element<'static, WorkshopMessage> {
        let label_text = text(format!("  {}", label)).size(11);

        button(label_text)
            .on_press(message)
            .padding([3, 8])
            .width(Length::Fill)
            .style(button::text)
            .into()
    }

    /// Edit menu items
    fn edit_menu_items(&self) -> Column<'static, WorkshopMessage> {
        Column::new()
            .push(Self::menu_item("Undo", "Ctrl+Z", WorkshopMessage::Undo))
            .push(Self::menu_item("Redo", "Ctrl+Y", WorkshopMessage::Redo))
            .push(Self::menu_separator())
            .push(Self::menu_item("Cut", "Ctrl+X", WorkshopMessage::Cut))
            .push(Self::menu_item("Copy", "Ctrl+C", WorkshopMessage::Copy))
            .push(Self::menu_item("Paste", "Ctrl+V", WorkshopMessage::Paste))
            .push(Self::menu_separator())
            .push(Self::menu_item("Find", "Ctrl+F", WorkshopMessage::Find))
            .push(Self::menu_item("Replace", "Ctrl+H", WorkshopMessage::Replace))
            .spacing(2)
            .width(Length::Fixed(180.0))
    }

    /// Run menu items
    fn run_menu_items(&self) -> Column<'static, WorkshopMessage> {
        let (run_label, run_msg): (&'static str, _) = if self.state == WorkshopState::Running {
            ("Stop", WorkshopMessage::Stop)
        } else {
            ("Run", WorkshopMessage::Run)
        };

        let (debug_label, debug_msg): (&'static str, _) = if self.state == WorkshopState::Debugging {
            ("Stop Debug", WorkshopMessage::DebugStop)
        } else {
            ("Debug", WorkshopMessage::Debug)
        };

        Column::new()
            .push(Self::menu_item(run_label, "F5", run_msg))
            .push(Self::menu_item(debug_label, "Ctrl+F5", debug_msg))
            .push(Self::menu_item("Build", "Ctrl+B", WorkshopMessage::Build))
            .spacing(2)
            .width(Length::Fixed(180.0))
    }

    /// View menu items
    fn view_menu_items(&self) -> Column<'static, WorkshopMessage> {
        // Use static strings for menu labels based on state
        let file_label = if self.config.layout.visibility.file_browser {
            "✓ File Browser"
        } else {
            "   File Browser"
        };
        let output_label = if self.config.layout.visibility.output {
            "✓ Output Panel"
        } else {
            "   Output Panel"
        };
        let repl_label = if self.config.layout.visibility.repl {
            "✓ REPL Panel"
        } else {
            "   REPL Panel"
        };
        let data_explorer_label = if self.config.layout.visibility.data_explorer {
            "✓ Data Explorer"
        } else {
            "   Data Explorer"
        };

        Column::new()
            .push(Self::menu_item(file_label, "", WorkshopMessage::ToggleFileBrowser))
            .push(Self::menu_item(output_label, "", WorkshopMessage::ToggleOutput))
            .push(Self::menu_item(repl_label, "", WorkshopMessage::ToggleRepl))
            .push(Self::menu_item(data_explorer_label, "", WorkshopMessage::ToggleDataExplorer))
            .spacing(2)
            .width(Length::Fixed(180.0))
    }

    /// Help menu items
    fn help_menu_items(&self) -> Column<'static, WorkshopMessage> {
        Column::new()
            .push(Self::menu_item("About", "", WorkshopMessage::ShowAbout))
            .spacing(2)
            .width(Length::Fixed(180.0))
    }

    /// Create a menu item with label and shortcut
    fn menu_item(label: &'static str, shortcut: &'static str, message: WorkshopMessage) -> Element<'static, WorkshopMessage> {
        let label_text = text(label).size(12);
        let shortcut_text = text(shortcut).size(10);

        let content = row![
            label_text,
            Space::new().width(Length::Fill),
            shortcut_text,
        ]
        .align_y(iced::Alignment::Center)
        .width(Length::Fill);

        button(content)
            .on_press(message)
            .padding([4, 8])
            .width(Length::Fill)
            .style(button::text)
            .into()
    }

    /// Create a menu separator
    fn menu_separator() -> Element<'static, WorkshopMessage> {
        container(rule::horizontal(1))
            .padding([2, 4])
            .width(Length::Fill)
            .into()
    }

    /// Render the toolbar
    fn toolbar(&self) -> Element<WorkshopMessage> {
        use iced::widget::text_input;

        let is_debugging = self.state == WorkshopState::Debugging;
        let is_paused = is_debugging && self.debug_session.is_paused();

        // Run/Stop button
        let run_button = if self.state == WorkshopState::Running {
            button(text("Stop").size(12))
                .on_press(WorkshopMessage::Stop)
                .style(button::danger)
        } else {
            button(text("Run").size(12))
                .on_press(WorkshopMessage::Run)
                .style(button::success)
        };

        // Debug button (only visible when not debugging)
        let debug_button = if !is_debugging {
            button(text("Debug").size(12))
                .on_press(WorkshopMessage::Debug)
                .style(button::primary)
        } else {
            button(text("Stop Debug").size(12))
                .on_press(WorkshopMessage::DebugStop)
                .style(button::danger)
        };

        // Debug controls (only visible when debugging and paused)
        let debug_controls = if is_paused {
            row![
                button(text("Continue").size(11))
                    .on_press(WorkshopMessage::DebugContinue)
                    .style(button::success),
                button(text("Step Over").size(11))
                    .on_press(WorkshopMessage::DebugStepOver)
                    .style(button::secondary),
                button(text("Step Into").size(11))
                    .on_press(WorkshopMessage::DebugStepInto)
                    .style(button::secondary),
                button(text("Step Out").size(11))
                    .on_press(WorkshopMessage::DebugStepOut)
                    .style(button::secondary),
            ]
            .spacing(4)
        } else {
            row![]
        };

        let build_button = button(text("Build").size(12))
            .on_press(WorkshopMessage::Build)
            .style(button::secondary);

        let args_label = text("Args:").size(12);
        let args_input = text_input("arguments...", &self.run_args)
            .on_input(WorkshopMessage::RunArgsChanged)
            .size(12)
            .width(Length::Fixed(150.0))
            .padding(4);

        container(
            row![run_button, debug_button, debug_controls, build_button, args_label, args_input]
                .spacing(8)
                .padding(4)
                .align_y(iced::Alignment::Center),
        )
        .width(Length::Fill)
        .into()
    }

    /// Handle debug result and return appropriate Task
    fn handle_debug_result(&mut self, result: crate::debug::DebugResult) -> Task<WorkshopMessage> {
        use crate::debug::DebugResult;
        match result {
            DebugResult::Started => {
                Task::none()
            }
            DebugResult::Paused { file, line, function_name, call_stack, locals, reason } => {
                Task::done(WorkshopMessage::DebugPaused {
                    file,
                    line,
                    function_name,
                    call_stack,
                    locals,
                    reason,
                })
            }
            DebugResult::Completed(value) => {
                Task::done(WorkshopMessage::DebugCompleted(value))
            }
            DebugResult::Error(error) => {
                Task::done(WorkshopMessage::DebugError(error))
            }
        }
    }

    /// Subscription for global shortcuts and drag events
    pub fn subscription(&self) -> Subscription<WorkshopMessage> {
        let keyboard_sub = keyboard::listen().filter_map(|event| {
            let keyboard::Event::KeyPressed { key, modifiers, .. } = event else {
                return None;
            };

            // Escape to close modal, menu, or cancel drag
            if let keyboard::Key::Named(key::Named::Escape) = key {
                return Some(WorkshopMessage::ModalCancel);
            }

            // F5 to run (Ctrl+F5 for debug)
            if let keyboard::Key::Named(key::Named::F5) = key {
                if modifiers.command() {
                    return Some(WorkshopMessage::Debug);
                } else {
                    return Some(WorkshopMessage::Run);
                }
            }

            // Ctrl+Tab / Cmd+Tab for next tab, Ctrl+Shift+Tab / Cmd+Shift+Tab for previous
            if let keyboard::Key::Named(key::Named::Tab) = key {
                if modifiers.command() {
                    if modifiers.shift() {
                        return Some(WorkshopMessage::PreviousTab);
                    } else {
                        return Some(WorkshopMessage::NextTab);
                    }
                }
            }

            // Character-based shortcuts
            if let keyboard::Key::Character(ref c) = key {
                if modifiers.command() {
                    match c.as_ref() {
                        // File operations
                        "n" => return Some(WorkshopMessage::NewFile),
                        "o" => return Some(WorkshopMessage::OpenFile),
                        "s" => {
                            if modifiers.shift() {
                                return Some(WorkshopMessage::SaveFileAs);
                            } else {
                                return Some(WorkshopMessage::SaveFile);
                            }
                        }
                        "w" => return Some(WorkshopMessage::CloseCurrentTab),
                        "q" => return Some(WorkshopMessage::Exit),

                        // Edit operations
                        "z" => return Some(WorkshopMessage::Undo),
                        "y" => return Some(WorkshopMessage::Redo),
                        "x" => return Some(WorkshopMessage::Cut),
                        "c" => return Some(WorkshopMessage::Copy),
                        "v" => return Some(WorkshopMessage::Paste),
                        "f" => return Some(WorkshopMessage::Find),
                        "h" => return Some(WorkshopMessage::Replace),

                        // Build
                        "b" => return Some(WorkshopMessage::Build),

                        _ => {}
                    }
                }
            }

            // Alt+Up / Alt+Down for REPL history navigation
            if modifiers.alt() {
                match key {
                    keyboard::Key::Named(key::Named::ArrowUp) => {
                        return Some(WorkshopMessage::Repl(ReplMessage::HistoryUp));
                    }
                    keyboard::Key::Named(key::Named::ArrowDown) => {
                        return Some(WorkshopMessage::Repl(ReplMessage::HistoryDown));
                    }
                    _ => {}
                }
            }

            None
        });

        // Listen for mouse events when dragging, context menu, or dropdown menu is open
        if self.editor.drag_state.is_some()
            || self.editor.context_menu.is_some()
            || self.file_browser.context_menu.is_some()
            || self.active_menu.is_some()
        {
            let mouse_sub = event::listen().map(WorkshopMessage::MouseEvent);
            Subscription::batch([keyboard_sub, mouse_sub])
        } else {
            keyboard_sub
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workshop_creation() {
        let workshop = Workshop::new();
        assert_eq!(workshop.state, WorkshopState::Idle);
        assert!(!workshop.pane_ids.is_empty());
    }

    #[test]
    fn test_toggle_visibility() {
        let mut workshop = Workshop::new();
        let initial = workshop.config.layout.visibility.file_browser;

        workshop.update(WorkshopMessage::ToggleFileBrowser);
        assert_ne!(workshop.config.layout.visibility.file_browser, initial);

        workshop.update(WorkshopMessage::ToggleFileBrowser);
        assert_eq!(workshop.config.layout.visibility.file_browser, initial);
    }

    #[test]
    fn test_new_file() {
        let mut workshop = Workshop::new();
        let initial_count = workshop.editor.tabs.len();

        workshop.update(WorkshopMessage::NewFile);
        assert_eq!(workshop.editor.tabs.len(), initial_count + 1);
    }

    #[test]
    fn test_close_unmodified_tab_directly() {
        let mut workshop = Workshop::new();
        workshop.update(WorkshopMessage::NewFile); // Creates second tab
        assert_eq!(workshop.editor.tabs.len(), 2);

        // Close unmodified tab - should close directly
        workshop.update(WorkshopMessage::CloseTab(1));
        assert_eq!(workshop.editor.tabs.len(), 1);
        assert!(workshop.modal.is_none());
    }

    #[test]
    fn test_close_modified_tab_shows_modal() {
        use iced::widget::text_editor::Action;

        let mut workshop = Workshop::new();

        // Mark tab as modified by simulating an edit
        workshop.editor.update(EditorMessage::Edit(Action::Edit(
            iced::widget::text_editor::Edit::Insert('x'),
        )));
        assert!(workshop.editor.tabs[0].modified);

        // Try to close - should show modal
        workshop.update(WorkshopMessage::CloseTab(0));
        assert!(workshop.modal.is_some());
        assert_eq!(workshop.editor.tabs.len(), 1); // Tab still exists
    }

    #[test]
    fn test_modal_discard_closes_tab() {
        use iced::widget::text_editor::Action;

        let mut workshop = Workshop::new();
        workshop.update(WorkshopMessage::NewFile); // Second tab

        // Mark first tab as modified
        workshop.editor.active_tab = 0;
        workshop.editor.update(EditorMessage::Edit(Action::Edit(
            iced::widget::text_editor::Edit::Insert('x'),
        )));

        // Try to close - shows modal
        workshop.update(WorkshopMessage::CloseTab(0));
        assert!(workshop.modal.is_some());
        assert_eq!(workshop.editor.tabs.len(), 2);

        // Discard changes
        workshop.update(WorkshopMessage::ModalDiscard);
        assert!(workshop.modal.is_none());
        assert_eq!(workshop.editor.tabs.len(), 1);
    }

    #[test]
    fn test_modal_cancel_keeps_tab() {
        use iced::widget::text_editor::Action;

        let mut workshop = Workshop::new();

        // Mark tab as modified
        workshop.editor.update(EditorMessage::Edit(Action::Edit(
            iced::widget::text_editor::Edit::Insert('x'),
        )));

        // Try to close - shows modal
        workshop.update(WorkshopMessage::CloseTab(0));
        assert!(workshop.modal.is_some());

        // Cancel - modal closes but tab remains
        workshop.update(WorkshopMessage::ModalCancel);
        assert!(workshop.modal.is_none());
        assert_eq!(workshop.editor.tabs.len(), 1);
    }

    #[test]
    fn test_tab_navigation() {
        let mut workshop = Workshop::new();
        workshop.update(WorkshopMessage::NewFile);
        workshop.update(WorkshopMessage::NewFile);
        assert_eq!(workshop.editor.tabs.len(), 3);
        assert_eq!(workshop.editor.active_tab, 2);

        // Next tab wraps around
        workshop.update(WorkshopMessage::NextTab);
        assert_eq!(workshop.editor.active_tab, 0);

        // Previous tab wraps around
        workshop.update(WorkshopMessage::PreviousTab);
        assert_eq!(workshop.editor.active_tab, 2);
    }
}
