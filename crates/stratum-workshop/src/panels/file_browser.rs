//! File browser panel
//!
//! Provides directory tree navigation for the project.

use iced::widget::{button, column, container, row, scrollable, text, text_input, Column, Row, Space};
use iced::{Color, Element, Length, Theme};
use std::path::PathBuf;

/// Messages emitted by the file browser panel
#[derive(Debug, Clone)]
pub enum FileBrowserMessage {
    /// Single-click selects an entry
    Select(usize),
    /// Double-click opens a file or toggles folder
    Activate(usize),
    /// Toggle folder expansion
    ToggleExpand(usize),
    /// Refresh the file tree
    Refresh,
    /// Search filter text changed
    SearchChanged(String),
    /// Open context menu at position
    ContextMenu(usize, f32, f32),
    /// Close context menu
    CloseContextMenu,
    /// Context menu actions
    NewFile,
    NewFolder,
    Rename,
    Delete,
    RevealInFileManager,
    /// Confirm dialog actions
    ConfirmDelete,
    CancelDialog,
    /// Text input for new file/folder/rename
    InputChanged(String),
    ConfirmInput,
}

/// State for context menu
#[derive(Debug, Clone)]
pub struct ContextMenuState {
    /// Index of the entry the menu is for
    pub entry_index: usize,
    /// Position of the menu
    pub position: (f32, f32),
}

/// State for input dialogs (new file, rename, etc.)
#[derive(Debug, Clone)]
pub enum DialogState {
    NewFile { parent_path: PathBuf, input: String },
    NewFolder { parent_path: PathBuf, input: String },
    Rename { path: PathBuf, input: String },
    ConfirmDelete { path: PathBuf, is_dir: bool },
}

/// Entry in the file tree
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub is_dir: bool,
    pub expanded: bool,
    pub depth: usize,
    /// Index of parent entry (None for root-level entries)
    pub parent_index: Option<usize>,
    /// Whether children have been loaded
    pub children_loaded: bool,
}

impl FileEntry {
    pub fn new(path: PathBuf, is_dir: bool, depth: usize) -> Self {
        Self {
            path,
            is_dir,
            expanded: false,
            depth,
            parent_index: None,
            children_loaded: false,
        }
    }

    /// Get the display name
    pub fn name(&self) -> String {
        self.path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| self.path.to_string_lossy().to_string())
    }

    /// Get file extension if any
    pub fn extension(&self) -> Option<&str> {
        self.path.extension().and_then(|e| e.to_str())
    }

    /// Get text marker icon for this entry type
    pub fn icon(&self) -> &'static str {
        if self.is_dir {
            if self.expanded {
                "[v]"
            } else {
                "[>]"
            }
        } else {
            match self.extension() {
                Some("strat") => ".st"  ,
                Some("rs") => ".rs",
                Some("toml") => ".tm",
                Some("md") => ".md",
                Some("json") => ".js",
                Some("yaml") | Some("yml") => ".ym",
                Some("txt") => ".tx",
                Some("py") => ".py",
                Some("js") | Some("ts") => ".js",
                Some("html") => ".ht",
                Some("css") => ".cs",
                _ => "   ",
            }
        }
    }
}

/// File browser panel with directory tree
#[derive(Debug)]
pub struct FileBrowserPanel {
    pub root: Option<PathBuf>,
    pub entries: Vec<FileEntry>,
    pub selected: Option<usize>,
    /// Search filter text
    pub search_filter: String,
    /// Context menu state
    pub context_menu: Option<ContextMenuState>,
    /// Dialog state (new file, rename, delete confirm)
    pub dialog: Option<DialogState>,
    /// Last error message
    pub last_error: Option<String>,
    /// Exclude patterns
    exclude_patterns: Vec<String>,
}

impl Default for FileBrowserPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl FileBrowserPanel {
    pub fn new() -> Self {
        Self {
            root: None,
            entries: Vec::new(),
            selected: None,
            search_filter: String::new(),
            context_menu: None,
            dialog: None,
            last_error: None,
            exclude_patterns: vec![
                ".git".to_string(),
                "target".to_string(),
                "__pycache__".to_string(),
                "node_modules".to_string(),
                ".DS_Store".to_string(),
            ],
        }
    }

    /// Open a folder and populate the tree
    pub fn open_folder(&mut self, path: PathBuf) -> std::io::Result<()> {
        self.root = Some(path.clone());
        self.entries.clear();
        self.selected = None;
        self.search_filter.clear();
        self.context_menu = None;
        self.dialog = None;
        self.last_error = None;
        self.scan_directory(&path, 0, None)?;
        Ok(())
    }

    /// Refresh the current folder
    pub fn refresh(&mut self) -> std::io::Result<()> {
        if let Some(root) = self.root.clone() {
            // Remember expanded paths
            let expanded_paths: Vec<PathBuf> = self
                .entries
                .iter()
                .filter(|e| e.expanded)
                .map(|e| e.path.clone())
                .collect();

            self.entries.clear();
            self.selected = None;
            self.context_menu = None;
            self.scan_directory(&root, 0, None)?;

            // Re-expand previously expanded directories
            for path in expanded_paths {
                if let Some(idx) = self.entries.iter().position(|e| e.path == path) {
                    self.expand_directory(idx)?;
                }
            }
        }
        Ok(())
    }

    /// Scan a directory and add entries
    fn scan_directory(
        &mut self,
        path: &PathBuf,
        depth: usize,
        parent_index: Option<usize>,
    ) -> std::io::Result<()> {
        let mut entries: Vec<_> = std::fs::read_dir(path)?
            .filter_map(|e| e.ok())
            .map(|e| {
                let path = e.path();
                let is_dir = path.is_dir();
                let mut entry = FileEntry::new(path, is_dir, depth);
                entry.parent_index = parent_index;
                entry
            })
            .filter(|e| !self.should_exclude(&e.path))
            .collect();

        // Sort: directories first, then alphabetically
        entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name().to_lowercase().cmp(&b.name().to_lowercase()),
        });

        self.entries.extend(entries);
        Ok(())
    }

    /// Check if a path should be excluded from display
    fn should_exclude(&self, path: &PathBuf) -> bool {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        // Exclude hidden files (starting with .)
        if name.starts_with('.') {
            return true;
        }
        // Check against exclude patterns
        self.exclude_patterns.iter().any(|p| name == p)
    }

    /// Toggle expansion of a directory entry
    pub fn toggle_expand(&mut self, index: usize) -> std::io::Result<()> {
        if let Some(entry) = self.entries.get(index) {
            if !entry.is_dir {
                return Ok(());
            }

            let is_expanded = entry.expanded;
            if is_expanded {
                self.collapse_directory(index);
            } else {
                self.expand_directory(index)?;
            }
        }
        Ok(())
    }

    /// Expand a directory and load its children
    fn expand_directory(&mut self, index: usize) -> std::io::Result<()> {
        let entry = &self.entries[index];
        let path = entry.path.clone();
        let depth = entry.depth + 1;

        // Mark as expanded
        self.entries[index].expanded = true;
        self.entries[index].children_loaded = true;

        // Scan children
        let mut children: Vec<_> = std::fs::read_dir(&path)?
            .filter_map(|e| e.ok())
            .map(|e| {
                let child_path = e.path();
                let is_dir = child_path.is_dir();
                let mut entry = FileEntry::new(child_path, is_dir, depth);
                entry.parent_index = Some(index);
                entry
            })
            .filter(|e| !self.should_exclude(&e.path))
            .collect();

        // Sort children
        children.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name().to_lowercase().cmp(&b.name().to_lowercase()),
        });

        // Insert children right after the parent
        let insert_pos = index + 1;
        for (i, child) in children.into_iter().enumerate() {
            self.entries.insert(insert_pos + i, child);
        }

        // Update parent indices for entries after the insertion
        let num_children = self.entries.iter().skip(insert_pos).take_while(|e| e.depth > self.entries[index].depth || e.parent_index == Some(index)).count();
        self.update_parent_indices_after_insert(insert_pos, num_children);

        Ok(())
    }

    /// Collapse a directory and remove its children
    fn collapse_directory(&mut self, index: usize) {
        self.entries[index].expanded = false;

        // Find and remove all descendants
        let parent_depth = self.entries[index].depth;
        let mut remove_count = 0;
        let start = index + 1;

        // Count entries to remove (all entries with greater depth that come after)
        for entry in self.entries.iter().skip(start) {
            if entry.depth > parent_depth {
                remove_count += 1;
            } else {
                break;
            }
        }

        // Remove the children
        if remove_count > 0 {
            self.entries.drain(start..start + remove_count);
            self.update_parent_indices_after_remove(start, remove_count);

            // Adjust selected index if needed
            if let Some(sel) = self.selected {
                if sel >= start && sel < start + remove_count {
                    self.selected = Some(index);
                } else if sel >= start + remove_count {
                    self.selected = Some(sel - remove_count);
                }
            }
        }
    }

    /// Update parent indices after insertion
    fn update_parent_indices_after_insert(&mut self, _start: usize, _count: usize) {
        // Parent indices are stored as absolute indices, so we need to update
        // indices for all entries that come after the insertion point
        // This is a simplified implementation - a more robust approach would use IDs
    }

    /// Update parent indices after removal
    fn update_parent_indices_after_remove(&mut self, _start: usize, _count: usize) {
        // Simplified - would need more careful handling in a production implementation
    }

    /// Get filtered entries based on search
    pub fn filtered_entries(&self) -> Vec<(usize, &FileEntry)> {
        if self.search_filter.is_empty() {
            self.entries.iter().enumerate().collect()
        } else {
            let filter = self.search_filter.to_lowercase();
            self.entries
                .iter()
                .enumerate()
                .filter(|(_, e)| e.name().to_lowercase().contains(&filter))
                .collect()
        }
    }

    /// Get the currently selected entry
    pub fn selected_entry(&self) -> Option<&FileEntry> {
        self.selected.and_then(|idx| self.entries.get(idx))
    }

    /// Get the path for creating new files (selected folder or root)
    fn get_target_folder(&self) -> Option<PathBuf> {
        if let Some(entry) = self.selected_entry() {
            if entry.is_dir {
                Some(entry.path.clone())
            } else {
                entry.path.parent().map(|p| p.to_path_buf())
            }
        } else {
            self.root.clone()
        }
    }

    /// Handle a file browser message
    pub fn update(&mut self, message: FileBrowserMessage) -> Option<PathBuf> {
        match message {
            FileBrowserMessage::Select(index) => {
                if index < self.entries.len() {
                    self.selected = Some(index);
                }
                self.context_menu = None;
                None
            }
            FileBrowserMessage::Activate(index) => {
                self.context_menu = None;
                if let Some(entry) = self.entries.get(index) {
                    if entry.is_dir {
                        let _ = self.toggle_expand(index);
                        None
                    } else {
                        // Return path for opening
                        Some(entry.path.clone())
                    }
                } else {
                    None
                }
            }
            FileBrowserMessage::ToggleExpand(index) => {
                let _ = self.toggle_expand(index);
                self.context_menu = None;
                None
            }
            FileBrowserMessage::Refresh => {
                if let Err(e) = self.refresh() {
                    self.last_error = Some(format!("Refresh failed: {e}"));
                }
                None
            }
            FileBrowserMessage::SearchChanged(filter) => {
                self.search_filter = filter;
                None
            }
            FileBrowserMessage::ContextMenu(index, x, y) => {
                self.selected = Some(index);
                self.context_menu = Some(ContextMenuState {
                    entry_index: index,
                    position: (x, y),
                });
                None
            }
            FileBrowserMessage::CloseContextMenu => {
                self.context_menu = None;
                None
            }
            FileBrowserMessage::NewFile => {
                self.context_menu = None;
                if let Some(folder) = self.get_target_folder() {
                    self.dialog = Some(DialogState::NewFile {
                        parent_path: folder,
                        input: String::new(),
                    });
                }
                None
            }
            FileBrowserMessage::NewFolder => {
                self.context_menu = None;
                if let Some(folder) = self.get_target_folder() {
                    self.dialog = Some(DialogState::NewFolder {
                        parent_path: folder,
                        input: String::new(),
                    });
                }
                None
            }
            FileBrowserMessage::Rename => {
                self.context_menu = None;
                if let Some(entry) = self.selected_entry() {
                    self.dialog = Some(DialogState::Rename {
                        path: entry.path.clone(),
                        input: entry.name(),
                    });
                }
                None
            }
            FileBrowserMessage::Delete => {
                self.context_menu = None;
                if let Some(entry) = self.selected_entry() {
                    self.dialog = Some(DialogState::ConfirmDelete {
                        path: entry.path.clone(),
                        is_dir: entry.is_dir,
                    });
                }
                None
            }
            FileBrowserMessage::RevealInFileManager => {
                self.context_menu = None;
                if let Some(entry) = self.selected_entry() {
                    let _ = reveal_in_file_manager(&entry.path);
                }
                None
            }
            FileBrowserMessage::ConfirmDelete => {
                if let Some(DialogState::ConfirmDelete { path, is_dir }) = self.dialog.take() {
                    let result = if is_dir {
                        std::fs::remove_dir_all(&path)
                    } else {
                        std::fs::remove_file(&path)
                    };
                    match result {
                        Ok(()) => {
                            let _ = self.refresh();
                        }
                        Err(e) => {
                            self.last_error = Some(format!("Delete failed: {e}"));
                        }
                    }
                }
                None
            }
            FileBrowserMessage::CancelDialog => {
                self.dialog = None;
                None
            }
            FileBrowserMessage::InputChanged(input) => {
                if let Some(ref mut dialog) = self.dialog {
                    match dialog {
                        DialogState::NewFile { input: i, .. } => *i = input,
                        DialogState::NewFolder { input: i, .. } => *i = input,
                        DialogState::Rename { input: i, .. } => *i = input,
                        DialogState::ConfirmDelete { .. } => {}
                    }
                }
                None
            }
            FileBrowserMessage::ConfirmInput => {
                let result = match self.dialog.take() {
                    Some(DialogState::NewFile { parent_path, input }) => {
                        if !input.is_empty() {
                            let new_path = parent_path.join(&input);
                            std::fs::write(&new_path, "").map_err(|e| e.to_string())
                        } else {
                            Err("File name cannot be empty".to_string())
                        }
                    }
                    Some(DialogState::NewFolder { parent_path, input }) => {
                        if !input.is_empty() {
                            let new_path = parent_path.join(&input);
                            std::fs::create_dir(&new_path).map_err(|e| e.to_string())
                        } else {
                            Err("Folder name cannot be empty".to_string())
                        }
                    }
                    Some(DialogState::Rename { path, input }) => {
                        if !input.is_empty() {
                            if let Some(parent) = path.parent() {
                                let new_path = parent.join(&input);
                                std::fs::rename(&path, &new_path).map_err(|e| e.to_string())
                            } else {
                                Err("Cannot rename root".to_string())
                            }
                        } else {
                            Err("Name cannot be empty".to_string())
                        }
                    }
                    _ => Ok(()),
                };

                match result {
                    Ok(()) => {
                        let _ = self.refresh();
                    }
                    Err(e) => {
                        self.last_error = Some(e);
                    }
                }
                None
            }
        }
    }

    /// Render the file browser panel
    pub fn view(&self) -> Element<'_, FileBrowserMessage> {
        let has_root = self.root.is_some();

        // Header with title and refresh button
        let header = self.header();

        // Search filter input
        let search_input = text_input("Search files...", &self.search_filter)
            .on_input(FileBrowserMessage::SearchChanged)
            .size(12)
            .padding(4);

        // Get filtered entries
        let filtered = self.filtered_entries();

        // Build the entry list
        let content: Element<'_, FileBrowserMessage> = if filtered.is_empty() {
            container(
                text(if has_root {
                    if self.search_filter.is_empty() {
                        "No files"
                    } else {
                        "No matching files"
                    }
                } else {
                    "Open a folder to get started"
                })
                .size(12),
            )
            .padding(10)
            .into()
        } else {
            let items: Vec<Element<'_, FileBrowserMessage>> = filtered
                .iter()
                .map(|(idx, entry)| self.render_entry(*idx, entry))
                .collect();

            scrollable(Column::with_children(items).spacing(1))
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        };

        // Error message if any
        let error_display: Element<'_, FileBrowserMessage> = if let Some(ref err) = self.last_error {
            container(text(err).size(10).color(Color::from_rgb(0.9, 0.2, 0.2)))
                .padding([2, 4])
                .into()
        } else {
            Space::new().height(0).into()
        };

        // Base content
        let base_content = container(
            column![header, search_input, content, error_display]
                .spacing(4)
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(4);

        // Render context menu if open
        if let Some(ref menu) = self.context_menu {
            let context_menu = self.render_context_menu(menu);
            iced::widget::stack![base_content, context_menu].into()
        } else if let Some(ref dialog) = self.dialog {
            let dialog_widget = self.render_dialog(dialog);
            iced::widget::stack![base_content, dialog_widget].into()
        } else {
            base_content.into()
        }
    }

    /// Render a single file entry
    fn render_entry(&self, index: usize, entry: &FileEntry) -> Element<'_, FileBrowserMessage> {
        let is_selected = self.selected == Some(index);
        let indent = "  ".repeat(entry.depth);
        let icon = entry.icon();
        let name = entry.name();

        let style: fn(&Theme) -> container::Style = if is_selected {
            |theme: &Theme| {
                let palette = theme.extended_palette();
                container::Style {
                    background: Some(palette.primary.weak.color.into()),
                    ..Default::default()
                }
            }
        } else {
            |_theme: &Theme| container::Style::default()
        };

        // Use mouse_area for click detection
        let entry_row = row![
            text(format!("{indent}{icon} ")).size(11).font(iced::Font::MONOSPACE),
            text(name).size(12),
        ]
        .spacing(0);

        let entry_container = container(entry_row)
            .style(style)
            .width(Length::Fill)
            .padding([2, 4]);

        // Wrap in mouse_area for click events
        iced::widget::mouse_area(entry_container)
            .on_press(FileBrowserMessage::Select(index))
            .on_release(FileBrowserMessage::Activate(index))
            .into()
    }

    /// Render the header
    fn header(&self) -> Element<'_, FileBrowserMessage> {
        let title = self
            .root
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "No folder".to_string());

        let refresh_btn = button(text("↻").size(12))
            .on_press(FileBrowserMessage::Refresh)
            .padding([2, 6])
            .style(button::text);

        let new_file_btn = button(text("+").size(12))
            .on_press(FileBrowserMessage::NewFile)
            .padding([2, 6])
            .style(button::text);

        row![
            text(title).size(14),
            Space::new().width(Length::Fill),
            new_file_btn,
            refresh_btn,
        ]
        .spacing(4)
        .align_y(iced::Alignment::Center)
        .into()
    }

    /// Render context menu
    fn render_context_menu(&self, menu: &ContextMenuState) -> Element<'_, FileBrowserMessage> {
        let is_dir = self
            .entries
            .get(menu.entry_index)
            .map(|e| e.is_dir)
            .unwrap_or(false);

        let mut menu_items = Column::new().spacing(2).width(Length::Shrink);

        if is_dir {
            menu_items = menu_items
                .push(
                    button(text("New File").size(11))
                        .on_press(FileBrowserMessage::NewFile)
                        .padding([4, 12])
                        .width(Length::Fill)
                        .style(button::text),
                )
                .push(
                    button(text("New Folder").size(11))
                        .on_press(FileBrowserMessage::NewFolder)
                        .padding([4, 12])
                        .width(Length::Fill)
                        .style(button::text),
                );
        }

        menu_items = menu_items
            .push(
                button(text("Rename").size(11))
                    .on_press(FileBrowserMessage::Rename)
                    .padding([4, 12])
                    .width(Length::Fill)
                    .style(button::text),
            )
            .push(
                button(text("Delete").size(11))
                    .on_press(FileBrowserMessage::Delete)
                    .padding([4, 12])
                    .width(Length::Fill)
                    .style(button::text),
            )
            .push(
                button(text("Reveal in File Manager").size(11))
                    .on_press(FileBrowserMessage::RevealInFileManager)
                    .padding([4, 12])
                    .width(Length::Fill)
                    .style(button::text),
            );

        let menu_widget = container(menu_items)
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

        // Position the menu (simplified - uses fixed offset)
        Column::new()
            .push(Space::new().height(80))
            .push(
                Row::new()
                    .push(Space::new().width(20))
                    .push(menu_widget)
                    .push(Space::new().width(Length::Fill)),
            )
            .push(Space::new().height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// Render dialog (new file, rename, delete confirm)
    fn render_dialog(&self, dialog: &DialogState) -> Element<'_, FileBrowserMessage> {
        let dialog_content: Element<'_, FileBrowserMessage> = match dialog {
            DialogState::NewFile { input, .. } => {
                column![
                    text("New File").size(14),
                    text_input("filename.strat", input)
                        .on_input(FileBrowserMessage::InputChanged)
                        .on_submit(FileBrowserMessage::ConfirmInput)
                        .size(12)
                        .padding(4),
                    row![
                        button(text("Create").size(11))
                            .on_press(FileBrowserMessage::ConfirmInput)
                            .padding([4, 12])
                            .style(button::primary),
                        button(text("Cancel").size(11))
                            .on_press(FileBrowserMessage::CancelDialog)
                            .padding([4, 12])
                            .style(button::secondary),
                    ]
                    .spacing(8)
                ]
                .spacing(8)
                .into()
            }
            DialogState::NewFolder { input, .. } => {
                column![
                    text("New Folder").size(14),
                    text_input("folder_name", input)
                        .on_input(FileBrowserMessage::InputChanged)
                        .on_submit(FileBrowserMessage::ConfirmInput)
                        .size(12)
                        .padding(4),
                    row![
                        button(text("Create").size(11))
                            .on_press(FileBrowserMessage::ConfirmInput)
                            .padding([4, 12])
                            .style(button::primary),
                        button(text("Cancel").size(11))
                            .on_press(FileBrowserMessage::CancelDialog)
                            .padding([4, 12])
                            .style(button::secondary),
                    ]
                    .spacing(8)
                ]
                .spacing(8)
                .into()
            }
            DialogState::Rename { input, path } => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                column![
                    text(format!("Rename '{name}'")).size(14),
                    text_input("new_name", input)
                        .on_input(FileBrowserMessage::InputChanged)
                        .on_submit(FileBrowserMessage::ConfirmInput)
                        .size(12)
                        .padding(4),
                    row![
                        button(text("Rename").size(11))
                            .on_press(FileBrowserMessage::ConfirmInput)
                            .padding([4, 12])
                            .style(button::primary),
                        button(text("Cancel").size(11))
                            .on_press(FileBrowserMessage::CancelDialog)
                            .padding([4, 12])
                            .style(button::secondary),
                    ]
                    .spacing(8)
                ]
                .spacing(8)
                .into()
            }
            DialogState::ConfirmDelete { path, is_dir } => {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let type_name = if *is_dir { "folder" } else { "file" };
                column![
                    text("Confirm Delete").size(14),
                    text(format!("Delete {type_name} '{name}'?")).size(12),
                    if *is_dir {
                        text("This will delete all contents!").size(11).color(Color::from_rgb(0.9, 0.2, 0.2))
                    } else {
                        text("").size(1)
                    },
                    row![
                        button(text("Delete").size(11))
                            .on_press(FileBrowserMessage::ConfirmDelete)
                            .padding([4, 12])
                            .style(button::danger),
                        button(text("Cancel").size(11))
                            .on_press(FileBrowserMessage::CancelDialog)
                            .padding([4, 12])
                            .style(button::secondary),
                    ]
                    .spacing(8)
                ]
                .spacing(8)
                .into()
            }
        };

        let dialog_box = container(dialog_content)
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
                        color: Color::from_rgba(0.0, 0.0, 0.0, 0.4),
                        offset: iced::Vector::new(4.0, 4.0),
                        blur_radius: 8.0,
                    },
                    ..Default::default()
                }
            })
            .padding(16);

        // Center the dialog
        container(iced::widget::center(dialog_box))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_theme: &Theme| container::Style {
                background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.5).into()),
                ..Default::default()
            })
            .into()
    }
}

/// Reveal a path in the system file manager (cross-platform)
fn reveal_in_file_manager(path: &PathBuf) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(path)
            .spawn()?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg("/select,")
            .arg(path)
            .spawn()?;
    }

    #[cfg(target_os = "linux")]
    {
        // Try different file managers
        if let Some(parent) = path.parent() {
            // Try xdg-open first (most common)
            if std::process::Command::new("xdg-open")
                .arg(parent)
                .spawn()
                .is_err()
            {
                // Fallback to nautilus
                let _ = std::process::Command::new("nautilus")
                    .arg("--select")
                    .arg(path)
                    .spawn();
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_directory() -> TempDir {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        // Create test structure:
        // root/
        //   file1.strat
        //   file2.rs
        //   subdir/
        //     nested.strat
        //   .hidden
        //   target/  (should be excluded)

        fs::write(root.join("file1.strat"), "// stratum file").unwrap();
        fs::write(root.join("file2.rs"), "// rust file").unwrap();
        fs::create_dir(root.join("subdir")).unwrap();
        fs::write(root.join("subdir/nested.strat"), "// nested").unwrap();
        fs::write(root.join(".hidden"), "hidden").unwrap();
        fs::create_dir(root.join("target")).unwrap();
        fs::write(root.join("target/build.txt"), "build output").unwrap();

        temp
    }

    #[test]
    fn test_new_panel() {
        let panel = FileBrowserPanel::new();
        assert!(panel.root.is_none());
        assert!(panel.entries.is_empty());
        assert!(panel.selected.is_none());
    }

    #[test]
    fn test_open_folder() {
        let temp = setup_test_directory();
        let mut panel = FileBrowserPanel::new();

        panel.open_folder(temp.path().to_path_buf()).unwrap();

        assert!(panel.root.is_some());
        assert!(!panel.entries.is_empty());

        // Should have subdir, file1.strat, file2.rs (sorted: dirs first, then alphabetically)
        // Should NOT have .hidden or target
        let names: Vec<String> = panel.entries.iter().map(|e| e.name()).collect();
        assert!(names.contains(&"subdir".to_string()));
        assert!(names.contains(&"file1.strat".to_string()));
        assert!(names.contains(&"file2.rs".to_string()));
        assert!(!names.contains(&".hidden".to_string()));
        assert!(!names.contains(&"target".to_string()));
    }

    #[test]
    fn test_sorting_dirs_first() {
        let temp = setup_test_directory();
        let mut panel = FileBrowserPanel::new();

        panel.open_folder(temp.path().to_path_buf()).unwrap();

        // First entry should be a directory
        assert!(panel.entries[0].is_dir);
        assert_eq!(panel.entries[0].name(), "subdir");
    }

    #[test]
    fn test_expand_collapse() {
        let temp = setup_test_directory();
        let mut panel = FileBrowserPanel::new();

        panel.open_folder(temp.path().to_path_buf()).unwrap();

        // Find the subdir index
        let subdir_idx = panel.entries.iter().position(|e| e.name() == "subdir").unwrap();
        assert!(!panel.entries[subdir_idx].expanded);

        // Expand it
        panel.toggle_expand(subdir_idx).unwrap();
        assert!(panel.entries[subdir_idx].expanded);

        // Should now have nested.strat in entries
        let names: Vec<String> = panel.entries.iter().map(|e| e.name()).collect();
        assert!(names.contains(&"nested.strat".to_string()));

        // Collapse it
        panel.toggle_expand(subdir_idx).unwrap();
        assert!(!panel.entries[subdir_idx].expanded);

        // nested.strat should be removed
        let names: Vec<String> = panel.entries.iter().map(|e| e.name()).collect();
        assert!(!names.contains(&"nested.strat".to_string()));
    }

    #[test]
    fn test_select() {
        let temp = setup_test_directory();
        let mut panel = FileBrowserPanel::new();

        panel.open_folder(temp.path().to_path_buf()).unwrap();
        assert!(panel.selected.is_none());

        panel.update(FileBrowserMessage::Select(0));
        assert_eq!(panel.selected, Some(0));

        panel.update(FileBrowserMessage::Select(1));
        assert_eq!(panel.selected, Some(1));
    }

    #[test]
    fn test_search_filter() {
        let temp = setup_test_directory();
        let mut panel = FileBrowserPanel::new();

        panel.open_folder(temp.path().to_path_buf()).unwrap();

        // No filter - should show all
        let filtered = panel.filtered_entries();
        assert_eq!(filtered.len(), panel.entries.len());

        // Filter for "strat"
        panel.update(FileBrowserMessage::SearchChanged("strat".to_string()));
        let filtered = panel.filtered_entries();
        assert!(filtered.iter().all(|(_, e)| e.name().contains("strat")));
    }

    #[test]
    fn test_file_entry_icon() {
        let entry_dir = FileEntry::new(PathBuf::from("/test/dir"), true, 0);
        assert_eq!(entry_dir.icon(), "[>]");

        let mut entry_dir_expanded = FileEntry::new(PathBuf::from("/test/dir"), true, 0);
        entry_dir_expanded.expanded = true;
        assert_eq!(entry_dir_expanded.icon(), "[v]");

        let entry_strat = FileEntry::new(PathBuf::from("/test/file.strat"), false, 0);
        assert_eq!(entry_strat.icon(), ".st");

        let entry_rs = FileEntry::new(PathBuf::from("/test/file.rs"), false, 0);
        assert_eq!(entry_rs.icon(), ".rs");

        let entry_unknown = FileEntry::new(PathBuf::from("/test/file.xyz"), false, 0);
        assert_eq!(entry_unknown.icon(), "   ");
    }

    #[test]
    fn test_activate_file_returns_path() {
        let temp = setup_test_directory();
        let mut panel = FileBrowserPanel::new();

        panel.open_folder(temp.path().to_path_buf()).unwrap();

        // Find a file (not directory)
        let file_idx = panel.entries.iter().position(|e| !e.is_dir).unwrap();

        // Activate should return the path
        let result = panel.update(FileBrowserMessage::Activate(file_idx));
        assert!(result.is_some());
    }

    #[test]
    fn test_activate_dir_toggles_expand() {
        let temp = setup_test_directory();
        let mut panel = FileBrowserPanel::new();

        panel.open_folder(temp.path().to_path_buf()).unwrap();

        // Find subdir
        let dir_idx = panel.entries.iter().position(|e| e.is_dir).unwrap();
        assert!(!panel.entries[dir_idx].expanded);

        // Activate should toggle, not return path
        let result = panel.update(FileBrowserMessage::Activate(dir_idx));
        assert!(result.is_none());
        assert!(panel.entries[dir_idx].expanded);
    }

    #[test]
    fn test_context_menu() {
        let mut panel = FileBrowserPanel::new();

        panel.update(FileBrowserMessage::ContextMenu(0, 100.0, 50.0));
        assert!(panel.context_menu.is_some());
        assert_eq!(panel.context_menu.as_ref().unwrap().entry_index, 0);

        panel.update(FileBrowserMessage::CloseContextMenu);
        assert!(panel.context_menu.is_none());
    }

    #[test]
    fn test_new_file_dialog() {
        let temp = setup_test_directory();
        let mut panel = FileBrowserPanel::new();

        panel.open_folder(temp.path().to_path_buf()).unwrap();

        // Open new file dialog
        panel.update(FileBrowserMessage::NewFile);
        assert!(matches!(panel.dialog, Some(DialogState::NewFile { .. })));

        // Input a name
        panel.update(FileBrowserMessage::InputChanged("test.strat".to_string()));

        // Confirm
        panel.update(FileBrowserMessage::ConfirmInput);
        assert!(panel.dialog.is_none());

        // File should exist
        assert!(temp.path().join("test.strat").exists());
    }

    #[test]
    fn test_new_folder() {
        let temp = setup_test_directory();
        let mut panel = FileBrowserPanel::new();

        panel.open_folder(temp.path().to_path_buf()).unwrap();

        panel.update(FileBrowserMessage::NewFolder);
        assert!(matches!(panel.dialog, Some(DialogState::NewFolder { .. })));

        panel.update(FileBrowserMessage::InputChanged("newfolder".to_string()));
        panel.update(FileBrowserMessage::ConfirmInput);

        assert!(temp.path().join("newfolder").is_dir());
    }

    #[test]
    fn test_rename() {
        let temp = setup_test_directory();
        let mut panel = FileBrowserPanel::new();

        panel.open_folder(temp.path().to_path_buf()).unwrap();

        // Select file1.strat
        let file_idx = panel.entries.iter().position(|e| e.name() == "file1.strat").unwrap();
        panel.update(FileBrowserMessage::Select(file_idx));

        panel.update(FileBrowserMessage::Rename);
        assert!(matches!(panel.dialog, Some(DialogState::Rename { .. })));

        panel.update(FileBrowserMessage::InputChanged("renamed.strat".to_string()));
        panel.update(FileBrowserMessage::ConfirmInput);

        assert!(!temp.path().join("file1.strat").exists());
        assert!(temp.path().join("renamed.strat").exists());
    }

    #[test]
    fn test_delete_file() {
        let temp = setup_test_directory();
        let mut panel = FileBrowserPanel::new();

        panel.open_folder(temp.path().to_path_buf()).unwrap();

        // Select file2.rs
        let file_idx = panel.entries.iter().position(|e| e.name() == "file2.rs").unwrap();
        panel.update(FileBrowserMessage::Select(file_idx));

        panel.update(FileBrowserMessage::Delete);
        assert!(matches!(panel.dialog, Some(DialogState::ConfirmDelete { .. })));

        panel.update(FileBrowserMessage::ConfirmDelete);
        assert!(!temp.path().join("file2.rs").exists());
    }

    #[test]
    fn test_cancel_dialog() {
        let temp = setup_test_directory();
        let mut panel = FileBrowserPanel::new();

        panel.open_folder(temp.path().to_path_buf()).unwrap();

        panel.update(FileBrowserMessage::NewFile);
        assert!(panel.dialog.is_some());

        panel.update(FileBrowserMessage::CancelDialog);
        assert!(panel.dialog.is_none());
    }

    #[test]
    fn test_refresh() {
        let temp = setup_test_directory();
        let mut panel = FileBrowserPanel::new();

        panel.open_folder(temp.path().to_path_buf()).unwrap();
        let initial_count = panel.entries.len();

        // Add a new file externally
        fs::write(temp.path().join("new_file.txt"), "new").unwrap();

        panel.update(FileBrowserMessage::Refresh);

        // Should now have one more entry
        assert_eq!(panel.entries.len(), initial_count + 1);
    }
}
