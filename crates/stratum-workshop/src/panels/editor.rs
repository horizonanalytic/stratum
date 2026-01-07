//! Code editor panel
//!
//! Provides syntax-highlighted text editing for Stratum source files.
//! Uses iced's TextEditor widget with a custom Highlighter.

use crate::highlight::{highlight_to_format, HighlightSettings, StratumHighlighter};
use iced::widget::text_editor::{Action, Content};
use iced::widget::{button, checkbox, container, mouse_area, row, stack, text, text_editor, text_input, Column, Row, Space};
use iced::{Color, Element, Font, Length, Theme};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Message type for editor actions
#[derive(Debug, Clone)]
pub enum EditorMessage {
    /// Editor action from text_editor widget
    Edit(Action),
    /// Tab selection
    SelectTab(usize),
    /// Close a tab
    CloseTab(usize),
    /// Start dragging a tab
    TabDragStart(usize),
    /// Drag is moving over a position
    TabDragMove(f32),
    /// End dragging (drop)
    TabDragEnd,
    /// Cancel dragging
    TabDragCancel,
    /// Open context menu on tab
    TabContextMenu(usize, f32, f32), // tab_index, x, y
    /// Close context menu
    CloseContextMenu,
    /// Close other tabs (all except the one specified)
    CloseOtherTabs(usize),
    /// Close all tabs
    CloseAllTabs,
    /// Toggle breakpoint on a line (1-indexed)
    ToggleBreakpoint(usize),
    /// Find bar query changed
    FindQueryChanged(String),
    /// Replace text changed
    ReplaceTextChanged(String),
    /// Go to next match
    FindNext,
    /// Go to previous match
    FindPrevious,
    /// Replace current match
    ReplaceCurrent,
    /// Replace all matches
    ReplaceAll,
    /// Close find bar
    CloseFindBar,
    /// Toggle case sensitivity
    ToggleCaseSensitive,
}

/// State for tab drag-and-drop
#[derive(Debug, Clone)]
pub struct TabDragState {
    /// Index of the tab being dragged
    pub dragging_index: usize,
    /// Current x position of the drag
    pub current_x: f32,
    /// Target drop index (where tab would be inserted)
    pub drop_target: Option<usize>,
}

/// State for tab context menu
#[derive(Debug, Clone)]
pub struct TabContextMenuState {
    /// Index of the tab the menu is for
    pub tab_index: usize,
    /// Position of the menu (x, y)
    pub position: (f32, f32),
}

/// Represents an open file tab
#[derive(Debug)]
pub struct EditorTab {
    pub path: Option<PathBuf>,
    pub content: Content,
    pub modified: bool,
}

impl Clone for EditorTab {
    fn clone(&self) -> Self {
        // Content doesn't implement Clone, so we recreate from text
        let text_content = self.content.text();
        Self {
            path: self.path.clone(),
            content: Content::with_text(&text_content),
            modified: self.modified,
        }
    }
}

impl EditorTab {
    pub fn new_untitled() -> Self {
        Self {
            path: None,
            content: Content::new(),
            modified: false,
        }
    }

    pub fn from_file(path: PathBuf, file_content: String) -> Self {
        Self {
            path: Some(path),
            content: Content::with_text(&file_content),
            modified: false,
        }
    }

    /// Get the display name for the tab
    pub fn name(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Untitled".to_string())
    }

    /// Get the text content
    pub fn text(&self) -> String {
        self.content.text()
    }

    /// Get cursor position (line, column) - 1-indexed for display
    pub fn cursor_position(&self) -> (usize, usize) {
        let cursor = self.content.cursor();
        // position has line and column fields, both 0-indexed
        (cursor.position.line + 1, cursor.position.column + 1)
    }
}

/// State for find/replace bar
#[derive(Debug, Clone, Default)]
pub struct FindState {
    /// Whether find bar is visible
    pub visible: bool,
    /// Whether replace bar is also visible
    pub replace_visible: bool,
    /// Current search query
    pub query: String,
    /// Current replace text
    pub replace_text: String,
    /// Positions of matches (line 0-indexed, start column, end column)
    pub matches: Vec<(usize, usize, usize)>,
    /// Index of current highlighted match
    pub current_match: usize,
    /// Case-sensitive search
    pub case_sensitive: bool,
}

/// Editor panel with tabbed file editing
#[derive(Debug)]
pub struct EditorPanel {
    pub tabs: Vec<EditorTab>,
    pub active_tab: usize,
    pub highlight_settings: HighlightSettings,
    /// Tab drag state
    pub drag_state: Option<TabDragState>,
    /// Tab widths for drag calculations (cached)
    tab_positions: Vec<(f32, f32)>, // (start_x, width) for each tab
    /// Context menu state
    pub context_menu: Option<TabContextMenuState>,
    /// Breakpoints per file (file path -> set of line numbers, 1-indexed)
    pub breakpoints: HashMap<Option<PathBuf>, HashSet<usize>>,
    /// Current debug line (when paused in debugger, 1-indexed)
    pub debug_line: Option<usize>,
    /// Find/replace state
    pub find_state: FindState,
}

impl Default for EditorPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorPanel {
    pub fn new() -> Self {
        Self {
            tabs: vec![EditorTab::new_untitled()],
            active_tab: 0,
            highlight_settings: HighlightSettings::default(),
            drag_state: None,
            tab_positions: Vec::new(),
            context_menu: None,
            breakpoints: HashMap::new(),
            debug_line: None,
            find_state: FindState::default(),
        }
    }

    /// Toggle the find bar visibility
    pub fn toggle_find_bar(&mut self) {
        self.find_state.visible = !self.find_state.visible;
        if !self.find_state.visible {
            self.find_state.replace_visible = false;
        }
    }

    /// Toggle the replace bar visibility (also shows find bar)
    pub fn toggle_replace_bar(&mut self) {
        if self.find_state.replace_visible {
            self.find_state.replace_visible = false;
        } else {
            self.find_state.visible = true;
            self.find_state.replace_visible = true;
        }
    }

    /// Update find query and search for matches
    pub fn set_find_query(&mut self, query: String) {
        self.find_state.query = query.clone();
        self.find_state.matches.clear();
        self.find_state.current_match = 0;

        if query.is_empty() {
            return;
        }

        if let Some(tab) = self.active() {
            let text = tab.text();
            let search_query = if self.find_state.case_sensitive {
                query.clone()
            } else {
                query.to_lowercase()
            };

            for (line_idx, line) in text.lines().enumerate() {
                let search_line = if self.find_state.case_sensitive {
                    line.to_string()
                } else {
                    line.to_lowercase()
                };

                let mut start = 0;
                while let Some(pos) = search_line[start..].find(&search_query) {
                    let col = start + pos;
                    self.find_state.matches.push((line_idx, col, col + query.len()));
                    start = col + 1;
                }
            }
        }
    }

    /// Go to next match
    pub fn find_next(&mut self) {
        if !self.find_state.matches.is_empty() {
            self.find_state.current_match = (self.find_state.current_match + 1) % self.find_state.matches.len();
        }
    }

    /// Go to previous match
    pub fn find_previous(&mut self) {
        if !self.find_state.matches.is_empty() {
            if self.find_state.current_match == 0 {
                self.find_state.current_match = self.find_state.matches.len() - 1;
            } else {
                self.find_state.current_match -= 1;
            }
        }
    }

    /// Replace current match
    pub fn replace_current(&mut self) {
        if self.find_state.matches.is_empty() || self.find_state.replace_text.is_empty() {
            return;
        }

        // Clone values before mutable borrow
        let replace_text = self.find_state.replace_text.clone();
        let current_match = self.find_state.current_match;

        // Get the match position
        if let Some(&(line, start_col, end_col)) = self.find_state.matches.get(current_match) {
            if let Some(tab) = self.active_mut() {
                let text = tab.text();
                let lines: Vec<&str> = text.lines().collect();

                if line < lines.len() {
                    // Calculate the absolute character position
                    let mut abs_pos = 0;
                    for (i, l) in lines.iter().enumerate() {
                        if i == line {
                            abs_pos += start_col;
                            break;
                        }
                        abs_pos += l.len() + 1; // +1 for newline
                    }

                    // Build new text
                    let new_text = format!(
                        "{}{}{}",
                        &text[..abs_pos],
                        &replace_text,
                        &text[abs_pos + (end_col - start_col)..]
                    );

                    // Update content
                    tab.content = Content::with_text(&new_text);
                    tab.modified = true;
                }
            }

            // Re-search to update matches
            let query = self.find_state.query.clone();
            self.set_find_query(query);
        }
    }

    /// Replace all matches
    pub fn replace_all(&mut self) {
        if self.find_state.matches.is_empty() || self.find_state.replace_text.is_empty() {
            return;
        }

        // Clone values before mutable borrow
        let query = self.find_state.query.clone();
        let replacement = self.find_state.replace_text.clone();
        let case_sensitive = self.find_state.case_sensitive;

        if let Some(tab) = self.active_mut() {
            let text = tab.text();

            let new_text = if case_sensitive {
                text.replace(&query, &replacement)
            } else {
                // Case-insensitive replace
                let mut result = text.clone();
                let lower_text = text.to_lowercase();
                let lower_query = query.to_lowercase();
                let mut offset: i64 = 0;

                let mut start = 0;
                while let Some(pos) = lower_text[start..].find(&lower_query) {
                    let abs_pos = (start + pos) as i64 + offset;
                    result = format!(
                        "{}{}{}",
                        &result[..abs_pos as usize],
                        &replacement,
                        &result[(abs_pos as usize + query.len())..]
                    );
                    offset += replacement.len() as i64 - query.len() as i64;
                    start = start + pos + 1;
                }
                result
            };

            tab.content = Content::with_text(&new_text);
            tab.modified = true;
        }

        // Re-search to clear matches
        self.set_find_query(query);
    }

    /// Get the currently active tab
    pub fn active(&self) -> Option<&EditorTab> {
        self.tabs.get(self.active_tab)
    }

    /// Get the currently active tab mutably
    pub fn active_mut(&mut self) -> Option<&mut EditorTab> {
        self.tabs.get_mut(self.active_tab)
    }

    /// Open a new tab with the given file
    pub fn open_file(&mut self, path: PathBuf) -> std::io::Result<()> {
        // Check if already open
        if let Some(idx) = self.tabs.iter().position(|t| t.path.as_ref() == Some(&path)) {
            self.active_tab = idx;
            return Ok(());
        }

        let file_content = std::fs::read_to_string(&path)?;
        self.tabs.push(EditorTab::from_file(path, file_content));
        self.active_tab = self.tabs.len() - 1;
        Ok(())
    }

    /// Close the tab at the given index
    pub fn close_tab(&mut self, index: usize) {
        if self.tabs.len() > 1 {
            self.tabs.remove(index);
            if self.active_tab >= self.tabs.len() {
                self.active_tab = self.tabs.len() - 1;
            }
        }
    }

    /// Create a new untitled tab
    pub fn new_tab(&mut self) {
        self.tabs.push(EditorTab::new_untitled());
        self.active_tab = self.tabs.len() - 1;
    }

    /// Handle editor message
    pub fn update(&mut self, message: EditorMessage) {
        match message {
            EditorMessage::Edit(action) => {
                if let Some(tab) = self.active_mut() {
                    let is_edit = action.is_edit();
                    tab.content.perform(action);
                    if is_edit {
                        tab.modified = true;
                    }
                }
            }
            EditorMessage::SelectTab(index) => {
                if index < self.tabs.len() && self.drag_state.is_none() {
                    self.active_tab = index;
                }
            }
            EditorMessage::CloseTab(index) => {
                self.close_tab(index);
            }
            EditorMessage::TabDragStart(index) => {
                if index < self.tabs.len() {
                    self.drag_state = Some(TabDragState {
                        dragging_index: index,
                        current_x: 0.0,
                        drop_target: None,
                    });
                }
            }
            EditorMessage::TabDragMove(x) => {
                if let Some(ref mut drag) = self.drag_state {
                    drag.current_x = x;
                    let dragging_index = drag.dragging_index;
                    // Calculate drop target based on position
                    let target = Self::calculate_drop_index_static(&self.tab_positions, x, dragging_index, self.tabs.len());
                    drag.drop_target = target;
                }
            }
            EditorMessage::TabDragEnd => {
                if let Some(drag) = self.drag_state.take() {
                    if let Some(target) = drag.drop_target {
                        self.move_tab(drag.dragging_index, target);
                    }
                }
            }
            EditorMessage::TabDragCancel => {
                self.drag_state = None;
            }
            EditorMessage::TabContextMenu(index, x, y) => {
                self.context_menu = Some(TabContextMenuState {
                    tab_index: index,
                    position: (x, y),
                });
            }
            EditorMessage::CloseContextMenu => {
                self.context_menu = None;
            }
            EditorMessage::CloseOtherTabs(keep_index) => {
                // Close all tabs except the specified one
                // Note: This collects indices to close first to avoid modification during iteration
                self.context_menu = None;
                if keep_index < self.tabs.len() {
                    let kept_tab = self.tabs.remove(keep_index);
                    self.tabs.clear();
                    self.tabs.push(kept_tab);
                    self.active_tab = 0;
                }
            }
            EditorMessage::CloseAllTabs => {
                // Close all tabs and create a new untitled one
                self.context_menu = None;
                self.tabs.clear();
                self.tabs.push(EditorTab::new_untitled());
                self.active_tab = 0;
            }
            EditorMessage::ToggleBreakpoint(line) => {
                // Toggle breakpoint for the active file at the given line
                if let Some(tab) = self.active() {
                    let file_key = tab.path.clone();
                    let lines = self.breakpoints.entry(file_key).or_default();
                    if lines.contains(&line) {
                        lines.remove(&line);
                    } else {
                        lines.insert(line);
                    }
                }
            }
            EditorMessage::FindQueryChanged(query) => {
                self.set_find_query(query);
            }
            EditorMessage::ReplaceTextChanged(text) => {
                self.find_state.replace_text = text;
            }
            EditorMessage::FindNext => {
                self.find_next();
            }
            EditorMessage::FindPrevious => {
                self.find_previous();
            }
            EditorMessage::ReplaceCurrent => {
                self.replace_current();
            }
            EditorMessage::ReplaceAll => {
                self.replace_all();
            }
            EditorMessage::CloseFindBar => {
                self.find_state.visible = false;
                self.find_state.replace_visible = false;
            }
            EditorMessage::ToggleCaseSensitive => {
                self.find_state.case_sensitive = !self.find_state.case_sensitive;
                // Re-search with new case sensitivity
                let query = self.find_state.query.clone();
                self.set_find_query(query);
            }
        }
    }

    /// Check if a breakpoint exists at the given line for the active file
    pub fn has_breakpoint(&self, line: usize) -> bool {
        if let Some(tab) = self.active() {
            self.breakpoints
                .get(&tab.path)
                .map(|lines| lines.contains(&line))
                .unwrap_or(false)
        } else {
            false
        }
    }

    /// Get all breakpoints for the active file
    pub fn get_breakpoints(&self) -> Vec<usize> {
        if let Some(tab) = self.active() {
            self.breakpoints
                .get(&tab.path)
                .map(|lines| lines.iter().copied().collect())
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    /// Set the current debug line (when paused)
    pub fn set_debug_line(&mut self, line: Option<usize>) {
        self.debug_line = line;
    }

    /// Clear all breakpoints for the active file
    pub fn clear_breakpoints(&mut self) {
        if let Some(path) = self.active().and_then(|tab| tab.path.clone()) {
            self.breakpoints.remove(&Some(path));
        } else {
            // For untitled files
            self.breakpoints.remove(&None);
        }
    }

    /// Calculate the drop index based on current x position (static to avoid borrow issues)
    fn calculate_drop_index_static(
        tab_positions: &[(f32, f32)],
        x: f32,
        dragging_index: usize,
        tab_count: usize,
    ) -> Option<usize> {
        if tab_positions.is_empty() || tab_count == 0 {
            return None;
        }

        // Find which tab the position is over
        for (i, (start, width)) in tab_positions.iter().enumerate() {
            let mid = start + width / 2.0;
            if x < mid {
                // Before this tab
                if i == dragging_index || i == dragging_index + 1 {
                    return None; // No change needed
                }
                return Some(if i > dragging_index { i - 1 } else { i });
            }
        }

        // Past the last tab
        let last_idx = tab_count - 1;
        if dragging_index == last_idx {
            return None;
        }
        Some(last_idx)
    }

    /// Move a tab from one index to another
    fn move_tab(&mut self, from: usize, to: usize) {
        if from == to || from >= self.tabs.len() || to >= self.tabs.len() {
            return;
        }

        let tab = self.tabs.remove(from);
        self.tabs.insert(to, tab);

        // Update active tab index if needed
        if self.active_tab == from {
            self.active_tab = to;
        } else if from < self.active_tab && to >= self.active_tab {
            self.active_tab -= 1;
        } else if from > self.active_tab && to <= self.active_tab {
            self.active_tab += 1;
        }
    }

    /// Update cached tab positions (called during rendering)
    pub fn update_tab_positions(&mut self, positions: Vec<(f32, f32)>) {
        self.tab_positions = positions;
    }

    /// Render the editor panel
    pub fn view(&self) -> Element<'_, EditorMessage> {
        let tab_bar = self.tab_bar();
        let status_bar = self.status_bar();

        let editor_content: Element<'_, EditorMessage> = if let Some(tab) = self.active() {
            let editor = text_editor(&tab.content)
                .placeholder("// Start typing or open a file...")
                .on_action(EditorMessage::Edit)
                .highlight_with::<StratumHighlighter>(
                    self.highlight_settings.clone(),
                    highlight_to_format,
                )
                .font(Font::MONOSPACE)
                .size(14)
                .padding(10);

            container(editor)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            container(text("No file open").size(14))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into()
        };

        let line_numbers = self.line_numbers();

        let editor_with_lines: Element<'_, EditorMessage> = row![
            line_numbers,
            editor_content,
        ]
        .spacing(0)
        .width(Length::Fill)
        .height(Length::Fill)
        .into();

        // Build content with optional find bar
        let mut content = Column::new()
            .push(tab_bar)
            .width(Length::Fill)
            .height(Length::Fill);

        // Add find bar if visible
        if self.find_state.visible {
            content = content.push(self.render_find_bar());
        }

        content = content.push(editor_with_lines).push(status_bar);

        let base_content: Element<'_, EditorMessage> = container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        // Render context menu if open
        if let Some(ref menu) = self.context_menu {
            let context_menu = self.render_context_menu(menu);
            stack![base_content, context_menu].into()
        } else {
            base_content
        }
    }

    /// Render the find/replace bar
    fn render_find_bar(&self) -> Element<'_, EditorMessage> {
        let match_count = self.find_state.matches.len();
        let current = if match_count > 0 {
            self.find_state.current_match + 1
        } else {
            0
        };

        let find_input = text_input("Find...", &self.find_state.query)
            .on_input(EditorMessage::FindQueryChanged)
            .on_submit(EditorMessage::FindNext)
            .size(12)
            .width(Length::Fixed(200.0))
            .padding(4);

        let match_label = text(format!("{}/{}", current, match_count)).size(11);

        let prev_button = button(text("<").size(11))
            .on_press(EditorMessage::FindPrevious)
            .padding([2, 6])
            .style(button::secondary);

        let next_button = button(text(">").size(11))
            .on_press(EditorMessage::FindNext)
            .padding([2, 6])
            .style(button::secondary);

        let case_toggle = checkbox(self.find_state.case_sensitive)
            .label("Aa")
            .on_toggle(|_| EditorMessage::ToggleCaseSensitive)
            .size(14)
            .spacing(4);

        let close_button = button(text("×").size(14))
            .on_press(EditorMessage::CloseFindBar)
            .padding([0, 6])
            .style(button::text);

        let mut find_row = Row::new()
            .push(find_input)
            .push(match_label)
            .push(prev_button)
            .push(next_button)
            .push(case_toggle)
            .spacing(8)
            .align_y(iced::Alignment::Center);

        // Add replace controls if visible
        if self.find_state.replace_visible {
            let replace_input = text_input("Replace...", &self.find_state.replace_text)
                .on_input(EditorMessage::ReplaceTextChanged)
                .size(12)
                .width(Length::Fixed(200.0))
                .padding(4);

            let replace_button = button(text("Replace").size(11))
                .on_press(EditorMessage::ReplaceCurrent)
                .padding([2, 8])
                .style(button::secondary);

            let replace_all_button = button(text("Replace All").size(11))
                .on_press(EditorMessage::ReplaceAll)
                .padding([2, 8])
                .style(button::secondary);

            find_row = find_row
                .push(Space::new().width(16))
                .push(replace_input)
                .push(replace_button)
                .push(replace_all_button);
        }

        find_row = find_row.push(Space::new().width(Length::Fill)).push(close_button);

        container(find_row)
            .padding([4, 8])
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

    /// Render the context menu overlay
    fn render_context_menu(&self, menu: &TabContextMenuState) -> Element<'_, EditorMessage> {
        let menu_content = Column::new()
            .push(
                button(text("Close").size(12))
                    .on_press(EditorMessage::CloseTab(menu.tab_index))
                    .padding([4, 12])
                    .width(Length::Fill)
                    .style(button::text),
            )
            .push(
                button(text("Close Others").size(12))
                    .on_press(EditorMessage::CloseOtherTabs(menu.tab_index))
                    .padding([4, 12])
                    .width(Length::Fill)
                    .style(button::text),
            )
            .push(
                button(text("Close All").size(12))
                    .on_press(EditorMessage::CloseAllTabs)
                    .padding([4, 12])
                    .width(Length::Fill)
                    .style(button::text),
            )
            .spacing(2)
            .width(Length::Shrink);

        // Position the menu - simplified to show at fixed position near top-left
        // A proper implementation would use absolute positioning
        let menu_widget = container(menu_content)
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

        // Use row/column with spacers to approximate positioning
        let (_x, _y) = menu.position;

        // Simplified: show menu at a fixed position (top area of editor)
        Column::new()
            .push(Space::new().height(40)) // Vertical offset
            .push(
                Row::new()
                    .push(Space::new().width(100)) // Horizontal offset
                    .push(menu_widget)
                    .push(Space::new().width(Length::Fill)),
            )
            .push(Space::new().height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// Render line numbers with breakpoint indicators
    fn line_numbers(&self) -> Element<'_, EditorMessage> {
        let line_count = self
            .active()
            .map(|tab| tab.content.line_count())
            .unwrap_or(1);

        let current_line = self
            .active()
            .map(|tab| tab.content.cursor().position.line)
            .unwrap_or(0);

        // Get current file's breakpoints
        let breakpoints: HashSet<usize> = self.active()
            .and_then(|tab| self.breakpoints.get(&tab.path))
            .cloned()
            .unwrap_or_default();

        let debug_line = self.debug_line;

        let lines: Vec<Element<'_, EditorMessage>> = (1..=line_count)
            .map(|n| {
                let is_current = n - 1 == current_line;
                let has_breakpoint = breakpoints.contains(&n);
                let is_debug_line = debug_line == Some(n);

                // Breakpoint indicator (red dot if set, empty if not)
                let bp_indicator = if has_breakpoint {
                    text("●").size(12).color(Color::from_rgb(0.9, 0.2, 0.2))
                } else {
                    text(" ").size(12)
                };

                // Debug arrow indicator (shows current execution line)
                let debug_indicator = if is_debug_line {
                    text("→").size(12).color(Color::from_rgb(1.0, 0.8, 0.0))
                } else {
                    text(" ").size(12)
                };

                // Line number with appropriate color
                let line_color = if is_debug_line {
                    Color::from_rgb(1.0, 0.8, 0.0) // Yellow for debug line
                } else if is_current {
                    Color::from_rgb(0.85, 0.85, 0.85) // Bright for cursor line
                } else {
                    Color::from_rgb(0.5, 0.5, 0.5) // Dim for other lines
                };

                let line_num = text(format!("{n:>4}"))
                    .size(14)
                    .font(Font::MONOSPACE)
                    .color(line_color);

                // Make the entire line row clickable for toggling breakpoints
                let line_row = row![bp_indicator, debug_indicator, line_num]
                    .spacing(2);

                // Wrap in a mouse_area to make it clickable
                mouse_area(line_row)
                    .on_press(EditorMessage::ToggleBreakpoint(n))
                    .into()
            })
            .collect();

        container(
            Column::with_children(lines)
                .spacing(0)
                .padding(10),
        )
        .style(|theme: &Theme| {
            let palette = theme.extended_palette();
            container::Style {
                background: Some(palette.background.weak.color.into()),
                ..Default::default()
            }
        })
        .into()
    }

    /// Render the tab bar
    fn tab_bar(&self) -> Element<'_, EditorMessage> {
        let is_dragging = self.drag_state.is_some();
        let dragging_idx = self.drag_state.as_ref().map(|d| d.dragging_index);
        let drop_target = self.drag_state.as_ref().and_then(|d| d.drop_target);

        let tabs: Vec<Element<'_, EditorMessage>> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(idx, tab)| {
                let name = if tab.modified {
                    format!("{}*", tab.name())
                } else {
                    tab.name()
                };
                let is_active = idx == self.active_tab;
                let is_being_dragged = dragging_idx == Some(idx);
                let is_drop_target = drop_target == Some(idx);

                // Style based on state
                let style: fn(&Theme) -> container::Style = if is_being_dragged {
                    |theme: &Theme| {
                        let palette = theme.extended_palette();
                        container::Style {
                            background: Some(Color::from_rgba(0.3, 0.5, 0.8, 0.5).into()),
                            border: iced::Border {
                                color: palette.primary.strong.color,
                                width: 2.0,
                                radius: 4.0.into(),
                            },
                            ..Default::default()
                        }
                    }
                } else if is_drop_target {
                    |theme: &Theme| {
                        let palette = theme.extended_palette();
                        container::Style {
                            background: Some(palette.background.weak.color.into()),
                            border: iced::Border {
                                color: palette.primary.base.color,
                                width: 2.0,
                                radius: 4.0.into(),
                            },
                            ..Default::default()
                        }
                    }
                } else if is_active {
                    container::rounded_box
                } else {
                    container::bordered_box
                };

                // Tab name button - starts drag on press
                let name_button = mouse_area(
                    iced::widget::button(text(name.clone()).size(12))
                        .on_press(EditorMessage::SelectTab(idx))
                        .padding([2, 4])
                        .style(iced::widget::button::text),
                )
                .on_press(EditorMessage::TabDragStart(idx));

                // Estimate position for context menu (simplified - actual position
                // would need layout calculation)
                let estimated_x = (idx as f32) * 120.0 + 100.0;
                let estimated_y = 60.0;

                let tab_content: Element<'_, EditorMessage> = row![
                    name_button,
                    // Menu button (opens context menu)
                    iced::widget::button(text("⋮").size(12))
                        .on_press(EditorMessage::TabContextMenu(idx, estimated_x, estimated_y))
                        .padding([2, 2])
                        .style(iced::widget::button::text),
                    iced::widget::button(text("×").size(12))
                        .on_press(EditorMessage::CloseTab(idx))
                        .padding([2, 4])
                        .style(iced::widget::button::text),
                ]
                .spacing(2)
                .into();

                container(tab_content)
                    .padding([4, 4])
                    .style(style)
                    .into()
            })
            .collect();

        // Wrap entire tab bar in mouse_area to detect drag cancel on escape
        let tab_row = Row::with_children(tabs).spacing(2).padding(2);

        let tab_bar_element: Element<'_, EditorMessage> = if is_dragging {
            // During drag, show cursor change
            container(tab_row)
                .width(Length::Fill)
                .style(|theme: &Theme| {
                    let palette = theme.extended_palette();
                    container::Style {
                        background: Some(palette.background.base.color.into()),
                        ..Default::default()
                    }
                })
                .into()
        } else {
            container(tab_row).width(Length::Fill).into()
        };

        tab_bar_element
    }

    /// Render the status bar showing cursor position
    fn status_bar(&self) -> Element<'_, EditorMessage> {
        let position_text = if let Some(tab) = self.active() {
            let (line, col) = tab.cursor_position();
            format!("Ln {line}, Col {col}")
        } else {
            String::new()
        };

        let file_type = "Stratum";

        container(
            row![
                text(file_type).size(11),
                Space::new().width(Length::Fill),
                text(position_text).size(11),
            ]
            .padding([2, 8])
            .align_y(iced::Alignment::Center),
        )
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_editor_panel() {
        let panel = EditorPanel::new();
        assert_eq!(panel.tabs.len(), 1);
        assert_eq!(panel.active_tab, 0);
    }

    #[test]
    fn test_new_tab() {
        let mut panel = EditorPanel::new();
        panel.new_tab();
        assert_eq!(panel.tabs.len(), 2);
        assert_eq!(panel.active_tab, 1);
    }

    #[test]
    fn test_close_tab() {
        let mut panel = EditorPanel::new();
        panel.new_tab();
        panel.new_tab();
        assert_eq!(panel.tabs.len(), 3);

        panel.close_tab(1);
        assert_eq!(panel.tabs.len(), 2);
    }

    #[test]
    fn test_tab_name_untitled() {
        let tab = EditorTab::new_untitled();
        assert_eq!(tab.name(), "Untitled");
    }

    #[test]
    fn test_tab_from_content() {
        let tab = EditorTab::from_file(
            PathBuf::from("/test/file.strat"),
            "let x = 42".to_string(),
        );
        assert_eq!(tab.name(), "file.strat");
        assert_eq!(tab.text(), "let x = 42");
        assert!(!tab.modified);
    }

    #[test]
    fn test_move_tab_forward() {
        let mut panel = EditorPanel::new();
        // Create 3 tabs: [0, 1, 2]
        panel.tabs[0].path = Some(PathBuf::from("tab0.strat"));
        panel.new_tab();
        panel.tabs[1].path = Some(PathBuf::from("tab1.strat"));
        panel.new_tab();
        panel.tabs[2].path = Some(PathBuf::from("tab2.strat"));

        panel.active_tab = 0;
        // Move tab 0 to position 2
        panel.move_tab(0, 2);

        assert_eq!(panel.tabs[0].name(), "tab1.strat");
        assert_eq!(panel.tabs[1].name(), "tab2.strat");
        assert_eq!(panel.tabs[2].name(), "tab0.strat");
        assert_eq!(panel.active_tab, 2); // Active tab moved
    }

    #[test]
    fn test_move_tab_backward() {
        let mut panel = EditorPanel::new();
        // Create 3 tabs
        panel.tabs[0].path = Some(PathBuf::from("tab0.strat"));
        panel.new_tab();
        panel.tabs[1].path = Some(PathBuf::from("tab1.strat"));
        panel.new_tab();
        panel.tabs[2].path = Some(PathBuf::from("tab2.strat"));

        panel.active_tab = 2;
        // Move tab 2 to position 0
        panel.move_tab(2, 0);

        assert_eq!(panel.tabs[0].name(), "tab2.strat");
        assert_eq!(panel.tabs[1].name(), "tab0.strat");
        assert_eq!(panel.tabs[2].name(), "tab1.strat");
        assert_eq!(panel.active_tab, 0); // Active tab moved
    }

    #[test]
    fn test_drag_state() {
        let mut panel = EditorPanel::new();
        panel.new_tab();

        // Start drag
        panel.update(EditorMessage::TabDragStart(0));
        assert!(panel.drag_state.is_some());
        assert_eq!(panel.drag_state.as_ref().unwrap().dragging_index, 0);

        // Cancel drag
        panel.update(EditorMessage::TabDragCancel);
        assert!(panel.drag_state.is_none());
    }

    #[test]
    fn test_context_menu_open_close() {
        let mut panel = EditorPanel::new();

        // Open context menu
        panel.update(EditorMessage::TabContextMenu(0, 100.0, 50.0));
        assert!(panel.context_menu.is_some());
        assert_eq!(panel.context_menu.as_ref().unwrap().tab_index, 0);

        // Close context menu
        panel.update(EditorMessage::CloseContextMenu);
        assert!(panel.context_menu.is_none());
    }

    #[test]
    fn test_close_other_tabs() {
        let mut panel = EditorPanel::new();
        panel.tabs[0].path = Some(PathBuf::from("tab0.strat"));
        panel.new_tab();
        panel.tabs[1].path = Some(PathBuf::from("tab1.strat"));
        panel.new_tab();
        panel.tabs[2].path = Some(PathBuf::from("tab2.strat"));

        // Close all except tab 1
        panel.update(EditorMessage::CloseOtherTabs(1));

        assert_eq!(panel.tabs.len(), 1);
        assert_eq!(panel.tabs[0].name(), "tab1.strat");
        assert_eq!(panel.active_tab, 0);
    }

    #[test]
    fn test_close_all_tabs() {
        let mut panel = EditorPanel::new();
        panel.new_tab();
        panel.new_tab();
        assert_eq!(panel.tabs.len(), 3);

        // Close all tabs
        panel.update(EditorMessage::CloseAllTabs);

        // Should have one untitled tab
        assert_eq!(panel.tabs.len(), 1);
        assert_eq!(panel.tabs[0].name(), "Untitled");
        assert_eq!(panel.active_tab, 0);
    }
}
