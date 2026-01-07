//! Custom canvas-based code editor widget
//!
//! A high-performance code editor with syntax highlighting, current line highlighting,
//! bracket matching, and other advanced features not available in iced's built-in text_editor.

use crate::highlight::{highlight_to_format, HighlightKind, HighlightSettings, StratumHighlighter};
use iced::advanced::text::highlighter::Highlighter;
use iced::keyboard::{self, Key};
use iced::mouse::{self, Cursor};
use iced::widget::canvas::{self, Cache, Canvas, Geometry, Path, Text};
use iced::{Color, Element, Event, Font, Length, Point, Rectangle, Renderer, Size, Theme, Vector};
use ropey::Rope;
use std::ops::Range;
use std::time::Instant;

/// Character dimensions for monospace font rendering
const CHAR_WIDTH: f32 = 8.4;
const LINE_HEIGHT: f32 = 20.0;
const GUTTER_PADDING: f32 = 8.0;
const EDITOR_PADDING: f32 = 10.0;
/// Standard indentation (4 spaces)
const INDENT: &str = "    ";
const INDENT_SIZE: usize = 4;
/// Scrollbar dimensions
const SCROLLBAR_WIDTH: f32 = 12.0;
const SCROLLBAR_MIN_THUMB_SIZE: f32 = 30.0;
/// Scroll margin - how close to the edge before scrolling kicks in
const SCROLL_MARGIN_LINES: usize = 2;
const SCROLL_MARGIN_CHARS: usize = 4;

/// Cursor position in the text buffer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Position {
    /// Line index (0-indexed)
    pub line: usize,
    /// Column index (0-indexed, in characters)
    pub column: usize,
}

impl Position {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

/// Text selection range
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    /// Start of selection (anchor)
    pub start: Position,
    /// End of selection (cursor)
    pub end: Position,
}

impl Selection {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    /// Returns true if selection is empty (cursor only)
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Returns the selection normalized so start <= end
    pub fn normalized(&self) -> Self {
        if self.start.line > self.end.line
            || (self.start.line == self.end.line && self.start.column > self.end.column)
        {
            Self {
                start: self.end,
                end: self.start,
            }
        } else {
            *self
        }
    }
}

/// Messages for the code editor
#[derive(Debug, Clone)]
pub enum CodeEditorMessage {
    /// Text input (character typed)
    Input(char),
    /// Backspace key pressed
    Backspace,
    /// Delete key pressed
    Delete,
    /// Enter/Return key pressed
    Enter,
    /// Tab key pressed
    Tab,
    /// Cursor movement
    MoveCursor(CursorMovement),
    /// Cursor movement with selection (shift held)
    MoveCursorSelect(CursorMovement),
    /// Mouse click at position
    Click(Point),
    /// Mouse drag (selection)
    Drag(Point),
    /// Mouse released
    Release,
    /// Double-click at position (select word)
    DoubleClick(Point),
    /// Triple-click at position (select line)
    TripleClick(Point),
    /// Set selection
    Select(Selection),
    /// Content changed (for external updates)
    ContentChanged(String),
    /// Scroll by offset
    Scroll(Vector<f32>),
    /// Start dragging vertical scrollbar
    StartDragVScrollbar(Point),
    /// Start dragging horizontal scrollbar
    StartDragHScrollbar(Point),
    /// Drag scrollbar to new position
    DragScrollbar(Point),
    /// Release scrollbar
    ReleaseScrollbar,
    /// Click on scrollbar track (page scroll)
    ClickVScrollbarTrack(f32),
    /// Click on horizontal scrollbar track
    ClickHScrollbarTrack(f32),
    /// Toggle cursor blink state
    CursorBlink,
    /// Editor gained focus
    Focus,
    /// Editor lost focus
    Blur,
    /// Viewport size changed
    ViewportResized(Size),
    /// Undo last edit
    Undo,
    /// Redo last undone edit
    Redo,
}

/// Cursor movement types
#[derive(Debug, Clone, Copy)]
pub enum CursorMovement {
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    WordLeft,
    WordRight,
    DocumentStart,
    DocumentEnd,
    /// Jump to matching bracket (Ctrl+])
    MatchingBracket,
}

/// Bracket pair for matching
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BracketPair {
    /// Position of the opening bracket
    pub open: Position,
    /// Position of the closing bracket
    pub close: Position,
}

/// Time window for grouping rapid consecutive edits (in milliseconds)
const EDIT_GROUP_TIMEOUT_MS: u128 = 500;

/// Represents a single edit operation for undo/redo
#[derive(Debug, Clone)]
pub struct EditOperation {
    /// The kind of edit operation
    pub kind: EditKind,
    /// Cursor position before the operation
    pub cursor_before: Position,
    /// Cursor position after the operation
    pub cursor_after: Position,
    /// Selection before the operation (if any)
    pub selection_before: Option<Selection>,
}

/// The kind of edit operation
#[derive(Debug, Clone)]
pub enum EditKind {
    /// Insert text at a position
    Insert {
        /// Position where text was inserted
        position: Position,
        /// The inserted text
        text: String,
    },
    /// Delete text at a position
    Delete {
        /// Position where text was deleted
        position: Position,
        /// The deleted text (for restoring on undo)
        text: String,
    },
    /// Replace text (delete + insert as atomic operation)
    Replace {
        /// Position where replacement occurred
        position: Position,
        /// Old text that was replaced
        old_text: String,
        /// New text that replaced it
        new_text: String,
    },
}

/// A group of related edit operations that should be undone/redone together
#[derive(Debug, Clone)]
pub struct EditGroup {
    /// The operations in this group (in order of execution)
    pub operations: Vec<EditOperation>,
    /// When this group was created
    pub timestamp: Instant,
}

/// State for the code editor
#[derive(Debug)]
pub struct CodeEditorState {
    /// Text content as rope for efficient operations
    rope: Rope,
    /// Current cursor position
    cursor: Position,
    /// Selection anchor (start position when shift-selecting or dragging)
    selection_anchor: Option<Position>,
    /// Current selection (None = no selection, just cursor)
    selection: Option<Selection>,
    /// Scroll offset (in pixels)
    scroll_offset: Vector<f32>,
    /// Last known viewport size (for scroll calculations)
    viewport_size: Size,
    /// Cached line highlight data
    highlight_cache: Vec<Vec<(Range<usize>, HighlightKind)>>,
    /// Lines that need re-highlighting
    dirty_lines: Vec<usize>,
    /// Highlight settings
    highlight_settings: HighlightSettings,
    /// Is the editor focused?
    focused: bool,
    /// Cursor blink state
    cursor_visible: bool,
    /// Canvas cache for redraw optimization
    cache: Cache,
    /// Is mouse currently being dragged (for text selection)?
    dragging: bool,
    /// Is vertical scrollbar being dragged?
    dragging_v_scrollbar: bool,
    /// Is horizontal scrollbar being dragged?
    dragging_h_scrollbar: bool,
    /// Mouse position when scrollbar drag started
    scrollbar_drag_start: Option<Point>,
    /// Scroll offset when scrollbar drag started
    scroll_offset_drag_start: Vector<f32>,
    /// Last click time for double/triple click detection
    last_click_time: Option<Instant>,
    /// Last click position for double/triple click detection
    last_click_pos: Option<Position>,
    /// Click count (1=single, 2=double, 3=triple)
    click_count: u8,
    /// Matching bracket pair (if cursor is adjacent to a bracket)
    matching_bracket: Option<BracketPair>,
    /// Undo stack (history of edit operations)
    undo_stack: Vec<EditGroup>,
    /// Redo stack (undone operations that can be redone)
    redo_stack: Vec<EditGroup>,
    /// Timestamp of last edit (for grouping rapid edits)
    last_edit_time: Option<Instant>,
}

impl Default for CodeEditorState {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeEditorState {
    /// Create a new empty editor state
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
            cursor: Position::default(),
            selection_anchor: None,
            selection: None,
            scroll_offset: Vector::new(0.0, 0.0),
            viewport_size: Size::new(800.0, 600.0), // Default, will be updated on first draw
            highlight_cache: Vec::new(),
            dirty_lines: Vec::new(),
            highlight_settings: HighlightSettings::default(),
            focused: false,
            cursor_visible: true,
            cache: Cache::new(),
            dragging: false,
            dragging_v_scrollbar: false,
            dragging_h_scrollbar: false,
            scrollbar_drag_start: None,
            scroll_offset_drag_start: Vector::new(0.0, 0.0),
            last_click_time: None,
            last_click_pos: None,
            click_count: 0,
            matching_bracket: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            last_edit_time: None,
        }
    }

    /// Create editor state with initial content
    pub fn with_text(text: &str) -> Self {
        let rope = Rope::from_str(text);
        let line_count = rope.len_lines();
        Self {
            rope,
            cursor: Position::default(),
            selection_anchor: None,
            selection: None,
            scroll_offset: Vector::new(0.0, 0.0),
            viewport_size: Size::new(800.0, 600.0),
            highlight_cache: vec![Vec::new(); line_count],
            dirty_lines: (0..line_count).collect(),
            highlight_settings: HighlightSettings::default(),
            focused: false,
            cursor_visible: true,
            cache: Cache::new(),
            dragging: false,
            dragging_v_scrollbar: false,
            dragging_h_scrollbar: false,
            scrollbar_drag_start: None,
            scroll_offset_drag_start: Vector::new(0.0, 0.0),
            last_click_time: None,
            last_click_pos: None,
            click_count: 0,
            matching_bracket: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            last_edit_time: None,
        }
    }

    /// Get the text content as a string
    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    /// Set the text content
    pub fn set_text(&mut self, text: &str) {
        self.rope = Rope::from_str(text);
        let line_count = self.rope.len_lines();
        self.highlight_cache = vec![Vec::new(); line_count];
        self.dirty_lines = (0..line_count).collect();
        self.cursor = Position::default();
        self.selection_anchor = None;
        self.selection = None;
        self.dragging = false;
        self.cache.clear();
    }

    /// Set focus state
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
        if focused {
            self.cursor_visible = true;
        }
        self.cache.clear();
    }

    /// Toggle cursor blink visibility
    pub fn toggle_cursor_blink(&mut self) {
        if self.focused {
            self.cursor_visible = !self.cursor_visible;
            self.cache.clear();
        }
    }

    /// Delete the current selection if any, returns true if something was deleted
    pub fn delete_selection(&mut self) -> bool {
        if let Some(sel) = self.selection.take() {
            let cursor_before = self.cursor;
            let selection_before = Some(sel);

            let normalized = sel.normalized();
            let start_idx = self.position_to_char_index(normalized.start);
            let end_idx = self.position_to_char_index(normalized.end);

            if start_idx < end_idx {
                // Get the text being deleted before removing it
                let deleted_text = self.get_text_range(start_idx, end_idx);

                self.rope.remove(start_idx..end_idx);
                self.cursor = normalized.start;
                self.selection_anchor = None;

                // Rebuild highlight cache
                let line_count = self.rope.len_lines();
                self.highlight_cache = vec![Vec::new(); line_count];
                self.dirty_lines = (0..line_count).collect();
                self.cache.clear();

                // Record the operation
                if !deleted_text.is_empty() {
                    self.record_edit(EditOperation {
                        kind: EditKind::Delete {
                            position: normalized.start,
                            text: deleted_text,
                        },
                        cursor_before,
                        cursor_after: self.cursor,
                        selection_before,
                    });
                }

                return true;
            }
        }
        false
    }

    /// Select the word at the given position
    pub fn select_word_at(&mut self, pos: Position) {
        let line = match self.line(pos.line) {
            Some(l) => l,
            None => return,
        };

        let chars: Vec<char> = line.chars().collect();
        let line_len = self.line_length(pos.line);
        let col = pos.column.min(line_len);

        if col >= chars.len() || chars.is_empty() {
            return;
        }

        // Find word boundaries
        let is_word_char = |c: char| c.is_alphanumeric() || c == '_';
        let current = chars.get(col).copied().unwrap_or(' ');

        if !is_word_char(current) {
            // Not on a word character, just position cursor
            self.move_cursor_to(pos);
            return;
        }

        // Find start of word
        let mut start = col;
        while start > 0 && is_word_char(chars[start - 1]) {
            start -= 1;
        }

        // Find end of word
        let mut end = col;
        while end < chars.len() && is_word_char(chars[end]) {
            end += 1;
        }

        self.selection = Some(Selection::new(
            Position::new(pos.line, start),
            Position::new(pos.line, end),
        ));
        self.cursor = Position::new(pos.line, end);
        self.cache.clear();
    }

    /// Select the entire line at the given position
    pub fn select_line_at(&mut self, pos: Position) {
        let line_len = self.line_length(pos.line);
        let is_last_line = pos.line >= self.rope.len_lines().saturating_sub(1);

        let start = Position::new(pos.line, 0);
        let end = if is_last_line {
            Position::new(pos.line, line_len)
        } else {
            Position::new(pos.line + 1, 0)
        };

        self.selection = Some(Selection::new(start, end));
        self.cursor = end;
        self.cache.clear();
    }

    /// Start a selection from current cursor position
    fn start_selection(&mut self) {
        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        }
    }

    /// Update selection based on anchor and current cursor
    fn update_selection_from_anchor(&mut self) {
        if let Some(anchor) = self.selection_anchor {
            self.selection = Some(Selection::new(anchor, self.cursor));
            self.cache.clear();
        }
    }

    /// Clear selection and anchor
    fn clear_selection(&mut self) {
        self.selection = None;
        self.selection_anchor = None;
    }

    /// Record an edit operation for undo/redo
    fn record_edit(&mut self, operation: EditOperation) {
        let now = Instant::now();

        // Check if we should append to the last group (grouping rapid consecutive edits)
        let should_group = self.last_edit_time.map_or(false, |last| {
            now.duration_since(last).as_millis() < EDIT_GROUP_TIMEOUT_MS
        }) && !self.undo_stack.is_empty()
            && self.can_group_with_last(&operation);

        if should_group {
            // Append to existing group
            if let Some(group) = self.undo_stack.last_mut() {
                group.operations.push(operation);
            }
        } else {
            // Create new group
            self.undo_stack.push(EditGroup {
                operations: vec![operation],
                timestamp: now,
            });
        }

        // Clear redo stack on new edit
        self.redo_stack.clear();
        self.last_edit_time = Some(now);
    }

    /// Check if an operation can be grouped with the last edit group
    fn can_group_with_last(&self, operation: &EditOperation) -> bool {
        if let Some(group) = self.undo_stack.last() {
            if let Some(last_op) = group.operations.last() {
                // Group consecutive character inserts
                if let (EditKind::Insert { text: last_text, .. }, EditKind::Insert { text: new_text, .. }) =
                    (&last_op.kind, &operation.kind)
                {
                    // Only group single character inserts (typing)
                    return last_text.len() == 1 && new_text.len() == 1;
                }
                // Group consecutive single-character deletes (backspace/delete)
                if let (EditKind::Delete { text: last_text, .. }, EditKind::Delete { text: new_text, .. }) =
                    (&last_op.kind, &operation.kind)
                {
                    return last_text.len() == 1 && new_text.len() == 1;
                }
            }
        }
        false
    }

    /// Undo the last edit group
    pub fn undo(&mut self) -> bool {
        if let Some(group) = self.undo_stack.pop() {
            // Apply operations in reverse order
            for op in group.operations.iter().rev() {
                self.apply_reverse_operation(op);
            }

            // Restore cursor to the position before the first operation
            if let Some(first_op) = group.operations.first() {
                self.cursor = first_op.cursor_before;
                self.selection = first_op.selection_before;
                self.ensure_cursor_visible();
            }

            // Push to redo stack
            self.redo_stack.push(group);
            self.cache.clear();
            return true;
        }
        false
    }

    /// Redo the last undone edit group
    pub fn redo(&mut self) -> bool {
        if let Some(group) = self.redo_stack.pop() {
            // Apply operations in forward order
            for op in &group.operations {
                self.apply_forward_operation(op);
            }

            // Restore cursor to the position after the last operation
            if let Some(last_op) = group.operations.last() {
                self.cursor = last_op.cursor_after;
                self.selection = None;
                self.ensure_cursor_visible();
            }

            // Push back to undo stack
            self.undo_stack.push(group);
            self.cache.clear();
            return true;
        }
        false
    }

    /// Apply an operation in reverse (for undo)
    fn apply_reverse_operation(&mut self, op: &EditOperation) {
        match &op.kind {
            EditKind::Insert { position, text } => {
                // Undo insert by deleting the text
                let start_idx = self.position_to_char_index(*position);
                let end_idx = start_idx + text.chars().count();
                if end_idx <= self.rope.len_chars() {
                    self.rope.remove(start_idx..end_idx);
                    // Update highlight cache
                    self.rebuild_highlight_cache();
                }
            }
            EditKind::Delete { position, text } => {
                // Undo delete by inserting the text back
                let char_idx = self.position_to_char_index(*position);
                for (i, ch) in text.chars().enumerate() {
                    self.rope.insert_char(char_idx + i, ch);
                }
                self.rebuild_highlight_cache();
            }
            EditKind::Replace { position, old_text, new_text } => {
                // Undo replace by replacing new with old
                let start_idx = self.position_to_char_index(*position);
                let end_idx = start_idx + new_text.chars().count();
                if end_idx <= self.rope.len_chars() {
                    self.rope.remove(start_idx..end_idx);
                }
                for (i, ch) in old_text.chars().enumerate() {
                    self.rope.insert_char(start_idx + i, ch);
                }
                self.rebuild_highlight_cache();
            }
        }
    }

    /// Apply an operation forward (for redo)
    fn apply_forward_operation(&mut self, op: &EditOperation) {
        match &op.kind {
            EditKind::Insert { position, text } => {
                let char_idx = self.position_to_char_index(*position);
                for (i, ch) in text.chars().enumerate() {
                    self.rope.insert_char(char_idx + i, ch);
                }
                self.rebuild_highlight_cache();
            }
            EditKind::Delete { position, text } => {
                let start_idx = self.position_to_char_index(*position);
                let end_idx = start_idx + text.chars().count();
                if end_idx <= self.rope.len_chars() {
                    self.rope.remove(start_idx..end_idx);
                    self.rebuild_highlight_cache();
                }
            }
            EditKind::Replace { position, old_text, new_text } => {
                let start_idx = self.position_to_char_index(*position);
                let end_idx = start_idx + old_text.chars().count();
                if end_idx <= self.rope.len_chars() {
                    self.rope.remove(start_idx..end_idx);
                }
                for (i, ch) in new_text.chars().enumerate() {
                    self.rope.insert_char(start_idx + i, ch);
                }
                self.rebuild_highlight_cache();
            }
        }
    }

    /// Rebuild the highlight cache after an undo/redo operation
    fn rebuild_highlight_cache(&mut self) {
        let line_count = self.rope.len_lines();
        self.highlight_cache = vec![Vec::new(); line_count];
        self.dirty_lines = (0..line_count).collect();
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Get the number of lines
    pub fn line_count(&self) -> usize {
        self.rope.len_lines()
    }

    /// Get the content of a specific line
    pub fn line(&self, index: usize) -> Option<String> {
        if index < self.rope.len_lines() {
            Some(self.rope.line(index).to_string())
        } else {
            None
        }
    }

    /// Get cursor position (1-indexed for display)
    pub fn cursor_position_display(&self) -> (usize, usize) {
        (self.cursor.line + 1, self.cursor.column + 1)
    }

    /// Move cursor to a position, clamping to valid range
    pub fn move_cursor_to(&mut self, pos: Position) {
        let line = pos.line.min(self.rope.len_lines().saturating_sub(1));
        let line_len = self.line_length(line);
        let column = pos.column.min(line_len);
        self.cursor = Position::new(line, column);
        self.update_matching_bracket();
        self.cache.clear();
    }

    /// Get the length of a line (excluding newline)
    fn line_length(&self, line_index: usize) -> usize {
        if line_index >= self.rope.len_lines() {
            return 0;
        }
        let line = self.rope.line(line_index);
        let len = line.len_chars();
        // Exclude trailing newline if present
        if len > 0 && line.char(len - 1) == '\n' {
            len - 1
        } else {
            len
        }
    }

    /// Get the leading whitespace (indentation) of a line
    fn get_line_indentation(&self, line_index: usize) -> String {
        if line_index >= self.rope.len_lines() {
            return String::new();
        }
        let line = self.rope.line(line_index).to_string();
        line.chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect()
    }

    /// Check if a line ends with an opening brace (ignoring trailing whitespace/comments)
    fn line_ends_with_open_brace(&self, line_index: usize) -> bool {
        if line_index >= self.rope.len_lines() {
            return false;
        }
        let line = self.rope.line(line_index).to_string();
        let trimmed = line.trim_end();

        // Remove trailing comment if present
        let code = if let Some(comment_pos) = trimmed.find("//") {
            trimmed[..comment_pos].trim_end()
        } else {
            trimmed
        };

        code.ends_with('{')
    }

    /// Get the character at a position, if valid
    fn char_at(&self, pos: Position) -> Option<char> {
        if pos.line >= self.rope.len_lines() {
            return None;
        }
        let line = self.rope.line(pos.line);
        let line_len = self.line_length(pos.line);
        if pos.column >= line_len {
            return None;
        }
        Some(line.char(pos.column))
    }

    /// Check if a character is a bracket
    fn is_bracket(ch: char) -> bool {
        matches!(ch, '(' | ')' | '[' | ']' | '{' | '}')
    }

    /// Get the matching bracket for a given bracket character
    fn matching_bracket_char(ch: char) -> Option<char> {
        match ch {
            '(' => Some(')'),
            ')' => Some('('),
            '[' => Some(']'),
            ']' => Some('['),
            '{' => Some('}'),
            '}' => Some('{'),
            _ => None,
        }
    }

    /// Check if a bracket is an opening bracket
    fn is_opening_bracket(ch: char) -> bool {
        matches!(ch, '(' | '[' | '{')
    }

    /// Find the matching bracket for the bracket at the given position
    /// Returns the position of the matching bracket if found
    fn find_matching_bracket(&self, pos: Position) -> Option<Position> {
        let ch = self.char_at(pos)?;
        if !Self::is_bracket(ch) {
            return None;
        }

        let target = Self::matching_bracket_char(ch)?;
        let is_forward = Self::is_opening_bracket(ch);

        // Stack to track nested brackets
        let mut depth = 1;

        if is_forward {
            // Search forward for matching closing bracket
            let mut current_line = pos.line;
            let mut current_col = pos.column + 1;

            while current_line < self.rope.len_lines() {
                let line = self.rope.line(current_line).to_string();
                let chars: Vec<char> = line.chars().collect();
                let line_len = self.line_length(current_line);

                while current_col < line_len {
                    let c = chars[current_col];
                    if c == ch {
                        depth += 1;
                    } else if c == target {
                        depth -= 1;
                        if depth == 0 {
                            return Some(Position::new(current_line, current_col));
                        }
                    }
                    current_col += 1;
                }

                current_line += 1;
                current_col = 0;
            }
        } else {
            // Search backward for matching opening bracket
            let mut current_line = pos.line;
            let mut current_col = pos.column as isize - 1;

            loop {
                if current_col >= 0 {
                    let line = self.rope.line(current_line).to_string();
                    let chars: Vec<char> = line.chars().collect();

                    while current_col >= 0 {
                        let c = chars[current_col as usize];
                        if c == ch {
                            depth += 1;
                        } else if c == target {
                            depth -= 1;
                            if depth == 0 {
                                return Some(Position::new(current_line, current_col as usize));
                            }
                        }
                        current_col -= 1;
                    }
                }

                if current_line == 0 {
                    break;
                }
                current_line -= 1;
                current_col = self.line_length(current_line) as isize - 1;
            }
        }

        None
    }

    /// Update the matching bracket state based on current cursor position
    fn update_matching_bracket(&mut self) {
        self.matching_bracket = None;

        // Check character at cursor position
        if let Some(ch) = self.char_at(self.cursor) {
            if Self::is_bracket(ch) {
                if let Some(matching_pos) = self.find_matching_bracket(self.cursor) {
                    let (open, close) = if Self::is_opening_bracket(ch) {
                        (self.cursor, matching_pos)
                    } else {
                        (matching_pos, self.cursor)
                    };
                    self.matching_bracket = Some(BracketPair { open, close });
                    return;
                }
            }
        }

        // Check character before cursor position (common case: cursor is after the bracket)
        if self.cursor.column > 0 {
            let before_pos = Position::new(self.cursor.line, self.cursor.column - 1);
            if let Some(ch) = self.char_at(before_pos) {
                if Self::is_bracket(ch) {
                    if let Some(matching_pos) = self.find_matching_bracket(before_pos) {
                        let (open, close) = if Self::is_opening_bracket(ch) {
                            (before_pos, matching_pos)
                        } else {
                            (matching_pos, before_pos)
                        };
                        self.matching_bracket = Some(BracketPair { open, close });
                    }
                }
            }
        }
    }

    /// Get the position to jump to for matching bracket
    fn get_matching_bracket_jump_position(&self) -> Option<Position> {
        // First check at cursor
        if let Some(ch) = self.char_at(self.cursor) {
            if Self::is_bracket(ch) {
                return self.find_matching_bracket(self.cursor);
            }
        }
        // Then check before cursor
        if self.cursor.column > 0 {
            let before_pos = Position::new(self.cursor.line, self.cursor.column - 1);
            if let Some(ch) = self.char_at(before_pos) {
                if Self::is_bracket(ch) {
                    return self.find_matching_bracket(before_pos);
                }
            }
        }
        None
    }

    /// Check if the current position is at the start of a line (only whitespace before cursor)
    fn is_at_line_start_whitespace(&self) -> bool {
        if let Some(line) = self.line(self.cursor.line) {
            let before_cursor: String = line.chars().take(self.cursor.column).collect();
            before_cursor.chars().all(|c| c == ' ' || c == '\t')
        } else {
            true
        }
    }

    /// Insert a newline with auto-indentation
    fn insert_newline_with_indent(&mut self) {
        // Get current line's indentation
        let base_indent = self.get_line_indentation(self.cursor.line);

        // Check if line ends with opening brace (for extra indent)
        let add_indent = self.line_ends_with_open_brace(self.cursor.line);

        // Insert newline
        self.insert_char('\n');

        // Insert indentation
        self.insert_str(&base_indent);
        if add_indent {
            self.insert_str(INDENT);
        }
    }

    /// Handle auto-dedent when closing brace is typed
    /// Returns true if dedent was performed
    fn handle_close_brace_dedent(&mut self) -> bool {
        // Only dedent if cursor is at start of line (only whitespace before)
        if !self.is_at_line_start_whitespace() {
            return false;
        }

        // Get current indentation
        let current_indent = self.get_line_indentation(self.cursor.line);
        if current_indent.is_empty() {
            return false;
        }

        // Calculate new indentation (one level less)
        let space_count: usize = current_indent
            .chars()
            .map(|c| if c == '\t' { INDENT_SIZE } else { 1 })
            .sum();
        let new_space_count = space_count.saturating_sub(INDENT_SIZE);
        let new_indent: String = " ".repeat(new_space_count);

        // Replace current line's indentation
        let line_start_idx = self.rope.line_to_char(self.cursor.line);
        let old_indent_len = current_indent.len();

        // Remove old indentation
        self.rope.remove(line_start_idx..line_start_idx + old_indent_len);

        // Insert new indentation
        for ch in new_indent.chars().rev() {
            self.rope.insert_char(line_start_idx, ch);
        }

        // Update cursor position
        self.cursor.column = new_indent.len();

        // Mark line as dirty
        self.dirty_lines.push(self.cursor.line);
        self.cache.clear();

        true
    }

    /// Insert a character at the cursor position
    pub fn insert_char(&mut self, ch: char) {
        self.insert_char_with_undo(ch, true);
    }

    /// Insert a character with optional undo recording
    fn insert_char_with_undo(&mut self, ch: char, record_undo: bool) {
        let cursor_before = self.cursor;
        let selection_before = self.selection;
        let insert_position = self.cursor;

        let char_idx = self.position_to_char_index(self.cursor);
        self.rope.insert_char(char_idx, ch);

        // Update cursor
        if ch == '\n' {
            self.cursor.line += 1;
            self.cursor.column = 0;
            // Insert new line in highlight cache
            self.highlight_cache
                .insert(self.cursor.line, Vec::new());
            self.dirty_lines.push(self.cursor.line.saturating_sub(1));
            self.dirty_lines.push(self.cursor.line);
        } else {
            self.cursor.column += 1;
            self.dirty_lines.push(self.cursor.line);
        }
        self.cache.clear();

        // Record the operation for undo
        if record_undo {
            self.record_edit(EditOperation {
                kind: EditKind::Insert {
                    position: insert_position,
                    text: ch.to_string(),
                },
                cursor_before,
                cursor_after: self.cursor,
                selection_before,
            });
        }
    }

    /// Insert a string at the cursor position
    pub fn insert_str(&mut self, s: &str) {
        for ch in s.chars() {
            self.insert_char(ch);
        }
    }

    /// Delete the character before the cursor (backspace)
    pub fn delete_backward(&mut self) {
        let cursor_before = self.cursor;
        let selection_before = self.selection;

        if self.cursor.column > 0 {
            let char_idx = self.position_to_char_index(self.cursor);
            let deleted_char = self.get_char_at_index(char_idx - 1);
            let delete_position = Position::new(self.cursor.line, self.cursor.column - 1);

            self.rope.remove(char_idx - 1..char_idx);
            self.cursor.column -= 1;
            self.dirty_lines.push(self.cursor.line);

            // Record the operation
            if let Some(ch) = deleted_char {
                self.record_edit(EditOperation {
                    kind: EditKind::Delete {
                        position: delete_position,
                        text: ch.to_string(),
                    },
                    cursor_before,
                    cursor_after: self.cursor,
                    selection_before,
                });
            }
        } else if self.cursor.line > 0 {
            // Merge with previous line (delete newline)
            let prev_line_len = self.line_length(self.cursor.line - 1);
            let char_idx = self.position_to_char_index(self.cursor);
            let delete_position = Position::new(self.cursor.line - 1, prev_line_len);

            self.rope.remove(char_idx - 1..char_idx);
            self.cursor.line -= 1;
            self.cursor.column = prev_line_len;
            // Remove line from highlight cache
            if self.cursor.line + 1 < self.highlight_cache.len() {
                self.highlight_cache.remove(self.cursor.line + 1);
            }
            self.dirty_lines.push(self.cursor.line);

            // Record the operation (deleted newline)
            self.record_edit(EditOperation {
                kind: EditKind::Delete {
                    position: delete_position,
                    text: "\n".to_string(),
                },
                cursor_before,
                cursor_after: self.cursor,
                selection_before,
            });
        }
        self.cache.clear();
    }

    /// Delete the character after the cursor
    pub fn delete_forward(&mut self) {
        let cursor_before = self.cursor;
        let selection_before = self.selection;

        let char_idx = self.position_to_char_index(self.cursor);
        if char_idx < self.rope.len_chars() {
            let deleted_char = self.get_char_at_index(char_idx);

            self.rope.remove(char_idx..char_idx + 1);
            self.dirty_lines.push(self.cursor.line);
            self.cache.clear();

            // Record the operation
            if let Some(ch) = deleted_char {
                self.record_edit(EditOperation {
                    kind: EditKind::Delete {
                        position: self.cursor,
                        text: ch.to_string(),
                    },
                    cursor_before,
                    cursor_after: self.cursor,
                    selection_before,
                });
            }
        }
    }

    /// Convert a Position to a character index in the rope
    fn position_to_char_index(&self, pos: Position) -> usize {
        if pos.line >= self.rope.len_lines() {
            return self.rope.len_chars();
        }
        let line_start = self.rope.line_to_char(pos.line);
        let line_len = self.line_length(pos.line);
        line_start + pos.column.min(line_len)
    }

    /// Get text in a character index range
    fn get_text_range(&self, start: usize, end: usize) -> String {
        if start >= end || end > self.rope.len_chars() {
            return String::new();
        }
        self.rope.slice(start..end).to_string()
    }

    /// Get the character at a position (for recording deletions)
    fn get_char_at_index(&self, idx: usize) -> Option<char> {
        if idx < self.rope.len_chars() {
            Some(self.rope.char(idx))
        } else {
            None
        }
    }

    /// Move cursor based on movement type
    pub fn move_cursor(&mut self, movement: CursorMovement) {
        match movement {
            CursorMovement::Left => {
                if self.cursor.column > 0 {
                    self.cursor.column -= 1;
                } else if self.cursor.line > 0 {
                    self.cursor.line -= 1;
                    self.cursor.column = self.line_length(self.cursor.line);
                }
            }
            CursorMovement::Right => {
                let line_len = self.line_length(self.cursor.line);
                if self.cursor.column < line_len {
                    self.cursor.column += 1;
                } else if self.cursor.line < self.rope.len_lines() - 1 {
                    self.cursor.line += 1;
                    self.cursor.column = 0;
                }
            }
            CursorMovement::Up => {
                if self.cursor.line > 0 {
                    self.cursor.line -= 1;
                    let line_len = self.line_length(self.cursor.line);
                    self.cursor.column = self.cursor.column.min(line_len);
                }
            }
            CursorMovement::Down => {
                if self.cursor.line < self.rope.len_lines() - 1 {
                    self.cursor.line += 1;
                    let line_len = self.line_length(self.cursor.line);
                    self.cursor.column = self.cursor.column.min(line_len);
                }
            }
            CursorMovement::Home => {
                self.cursor.column = 0;
            }
            CursorMovement::End => {
                self.cursor.column = self.line_length(self.cursor.line);
            }
            CursorMovement::PageUp => {
                let lines_per_page = 20; // Approximate
                self.cursor.line = self.cursor.line.saturating_sub(lines_per_page);
                let line_len = self.line_length(self.cursor.line);
                self.cursor.column = self.cursor.column.min(line_len);
            }
            CursorMovement::PageDown => {
                let lines_per_page = 20;
                self.cursor.line = (self.cursor.line + lines_per_page)
                    .min(self.rope.len_lines().saturating_sub(1));
                let line_len = self.line_length(self.cursor.line);
                self.cursor.column = self.cursor.column.min(line_len);
            }
            CursorMovement::WordLeft => {
                // Move to start of previous word
                if self.cursor.column > 0 {
                    if let Some(line) = self.line(self.cursor.line) {
                        let chars: Vec<char> = line.chars().collect();
                        let mut col = self.cursor.column.saturating_sub(1);
                        // Skip whitespace
                        while col > 0 && chars.get(col).map_or(false, |c| c.is_whitespace()) {
                            col -= 1;
                        }
                        // Skip word characters
                        while col > 0 && chars.get(col - 1).map_or(false, |c| !c.is_whitespace()) {
                            col -= 1;
                        }
                        self.cursor.column = col;
                    }
                } else if self.cursor.line > 0 {
                    self.cursor.line -= 1;
                    self.cursor.column = self.line_length(self.cursor.line);
                }
            }
            CursorMovement::WordRight => {
                if let Some(line) = self.line(self.cursor.line) {
                    let chars: Vec<char> = line.chars().collect();
                    let line_len = self.line_length(self.cursor.line);
                    let mut col = self.cursor.column;
                    // Skip current word
                    while col < line_len && chars.get(col).map_or(false, |c| !c.is_whitespace()) {
                        col += 1;
                    }
                    // Skip whitespace
                    while col < line_len && chars.get(col).map_or(false, |c| c.is_whitespace()) {
                        col += 1;
                    }
                    if col > self.cursor.column {
                        self.cursor.column = col;
                    } else if self.cursor.line < self.rope.len_lines() - 1 {
                        self.cursor.line += 1;
                        self.cursor.column = 0;
                    }
                }
            }
            CursorMovement::DocumentStart => {
                self.cursor = Position::default();
            }
            CursorMovement::DocumentEnd => {
                let last_line = self.rope.len_lines().saturating_sub(1);
                self.cursor.line = last_line;
                self.cursor.column = self.line_length(last_line);
            }
            CursorMovement::MatchingBracket => {
                if let Some(pos) = self.get_matching_bracket_jump_position() {
                    self.cursor = pos;
                }
            }
        }
        self.update_matching_bracket();
        self.cache.clear();
    }

    /// Convert screen point to text position
    pub fn point_to_position(&self, point: Point, gutter_width: f32) -> Position {
        let text_x = point.x - gutter_width - EDITOR_PADDING;
        let text_y = point.y + self.scroll_offset.y - EDITOR_PADDING;

        let line = ((text_y / LINE_HEIGHT).floor() as usize)
            .min(self.rope.len_lines().saturating_sub(1));
        let column = ((text_x / CHAR_WIDTH).round() as usize).min(self.line_length(line));

        Position::new(line, column)
    }

    /// Get gutter width based on line count
    pub fn gutter_width(&self) -> f32 {
        let digits = (self.rope.len_lines() as f32).log10().floor() as usize + 1;
        (digits.max(3) as f32) * CHAR_WIDTH + GUTTER_PADDING * 2.0
    }

    /// Calculate the maximum line width in the document (in pixels)
    fn max_line_width(&self) -> f32 {
        let mut max_len = 0usize;
        for i in 0..self.rope.len_lines() {
            let len = self.line_length(i);
            if len > max_len {
                max_len = len;
            }
        }
        (max_len as f32) * CHAR_WIDTH
    }

    /// Calculate total content height (in pixels)
    fn content_height(&self) -> f32 {
        (self.rope.len_lines() as f32) * LINE_HEIGHT
    }

    /// Get the text area width (viewport minus gutter and scrollbar)
    fn text_area_width(&self) -> f32 {
        let gutter = self.gutter_width();
        (self.viewport_size.width - gutter - SCROLLBAR_WIDTH - EDITOR_PADDING * 2.0).max(0.0)
    }

    /// Get the text area height (viewport minus scrollbar)
    fn text_area_height(&self) -> f32 {
        (self.viewport_size.height - SCROLLBAR_WIDTH - EDITOR_PADDING * 2.0).max(0.0)
    }

    /// Calculate maximum scroll offsets
    fn max_scroll(&self) -> Vector<f32> {
        let content_w = self.max_line_width();
        let content_h = self.content_height();
        let area_w = self.text_area_width();
        let area_h = self.text_area_height();

        Vector::new(
            (content_w - area_w).max(0.0),
            (content_h - area_h).max(0.0),
        )
    }

    /// Clamp scroll offset to valid bounds
    fn clamp_scroll(&mut self) {
        let max = self.max_scroll();
        self.scroll_offset.x = self.scroll_offset.x.clamp(0.0, max.x);
        self.scroll_offset.y = self.scroll_offset.y.clamp(0.0, max.y);
    }

    /// Update viewport size from bounds
    pub fn set_viewport_size(&mut self, size: Size) {
        if self.viewport_size != size {
            self.viewport_size = size;
            self.clamp_scroll();
            self.cache.clear();
        }
    }

    /// Ensure the cursor is visible by scrolling if necessary
    pub fn ensure_cursor_visible(&mut self) {
        let cursor_x = (self.cursor.column as f32) * CHAR_WIDTH;
        let cursor_y = (self.cursor.line as f32) * LINE_HEIGHT;

        let text_area_w = self.text_area_width();
        let text_area_h = self.text_area_height();

        // Vertical scrolling
        let margin_y = (SCROLL_MARGIN_LINES as f32) * LINE_HEIGHT;
        let visible_top = self.scroll_offset.y + margin_y;
        let visible_bottom = self.scroll_offset.y + text_area_h - margin_y - LINE_HEIGHT;

        if cursor_y < visible_top {
            self.scroll_offset.y = (cursor_y - margin_y).max(0.0);
        } else if cursor_y > visible_bottom {
            self.scroll_offset.y = cursor_y - text_area_h + margin_y + LINE_HEIGHT;
        }

        // Horizontal scrolling
        let margin_x = (SCROLL_MARGIN_CHARS as f32) * CHAR_WIDTH;
        let visible_left = self.scroll_offset.x + margin_x;
        let visible_right = self.scroll_offset.x + text_area_w - margin_x - CHAR_WIDTH;

        if cursor_x < visible_left {
            self.scroll_offset.x = (cursor_x - margin_x).max(0.0);
        } else if cursor_x > visible_right {
            self.scroll_offset.x = cursor_x - text_area_w + margin_x + CHAR_WIDTH;
        }

        self.clamp_scroll();
        self.cache.clear();
    }

    /// Check if vertical scrollbar is needed
    pub fn needs_vertical_scrollbar(&self) -> bool {
        self.content_height() > self.text_area_height()
    }

    /// Check if horizontal scrollbar is needed
    pub fn needs_horizontal_scrollbar(&self) -> bool {
        self.max_line_width() > self.text_area_width()
    }

    /// Calculate vertical scrollbar geometry
    /// Returns (track_rect, thumb_rect)
    pub fn vertical_scrollbar_geometry(&self, bounds: Rectangle) -> (Rectangle, Rectangle) {
        let track_x = bounds.width - SCROLLBAR_WIDTH;
        let track_y = 0.0;
        let track_h = bounds.height - if self.needs_horizontal_scrollbar() { SCROLLBAR_WIDTH } else { 0.0 };

        let track = Rectangle::new(
            Point::new(track_x, track_y),
            Size::new(SCROLLBAR_WIDTH, track_h),
        );

        let content_h = self.content_height();
        let visible_h = self.text_area_height();

        if content_h <= visible_h {
            // No scrolling needed, thumb fills track
            return (track, track);
        }

        let thumb_ratio = (visible_h / content_h).min(1.0);
        let thumb_h = (track_h * thumb_ratio).max(SCROLLBAR_MIN_THUMB_SIZE);
        let scrollable_track = track_h - thumb_h;
        let max_scroll_y = self.max_scroll().y;
        let scroll_ratio = if max_scroll_y > 0.0 {
            self.scroll_offset.y / max_scroll_y
        } else {
            0.0
        };
        let thumb_y = track_y + scrollable_track * scroll_ratio;

        let thumb = Rectangle::new(
            Point::new(track_x, thumb_y),
            Size::new(SCROLLBAR_WIDTH, thumb_h),
        );

        (track, thumb)
    }

    /// Calculate horizontal scrollbar geometry
    /// Returns (track_rect, thumb_rect)
    pub fn horizontal_scrollbar_geometry(&self, bounds: Rectangle) -> (Rectangle, Rectangle) {
        let gutter = self.gutter_width();
        let track_x = gutter;
        let track_y = bounds.height - SCROLLBAR_WIDTH;
        let track_w = bounds.width - gutter - if self.needs_vertical_scrollbar() { SCROLLBAR_WIDTH } else { 0.0 };

        let track = Rectangle::new(
            Point::new(track_x, track_y),
            Size::new(track_w, SCROLLBAR_WIDTH),
        );

        let content_w = self.max_line_width();
        let visible_w = self.text_area_width();

        if content_w <= visible_w {
            return (track, track);
        }

        let thumb_ratio = (visible_w / content_w).min(1.0);
        let thumb_w = (track_w * thumb_ratio).max(SCROLLBAR_MIN_THUMB_SIZE);
        let scrollable_track = track_w - thumb_w;
        let max_scroll_x = self.max_scroll().x;
        let scroll_ratio = if max_scroll_x > 0.0 {
            self.scroll_offset.x / max_scroll_x
        } else {
            0.0
        };
        let thumb_x = track_x + scrollable_track * scroll_ratio;

        let thumb = Rectangle::new(
            Point::new(thumb_x, track_y),
            Size::new(thumb_w, SCROLLBAR_WIDTH),
        );

        (track, thumb)
    }

    /// Check if a point is within the vertical scrollbar track
    pub fn is_in_vertical_scrollbar(&self, point: Point, bounds: Rectangle) -> bool {
        if !self.needs_vertical_scrollbar() {
            return false;
        }
        let (track, _) = self.vertical_scrollbar_geometry(bounds);
        track.contains(point)
    }

    /// Check if a point is within the horizontal scrollbar track
    pub fn is_in_horizontal_scrollbar(&self, point: Point, bounds: Rectangle) -> bool {
        if !self.needs_horizontal_scrollbar() {
            return false;
        }
        let (track, _) = self.horizontal_scrollbar_geometry(bounds);
        track.contains(point)
    }

    /// Update highlighting for dirty lines
    pub fn update_highlights(&mut self) {
        let dirty: Vec<usize> = self.dirty_lines.drain(..).collect();
        let mut highlighter = StratumHighlighter::new(&self.highlight_settings);

        for line_idx in dirty {
            if line_idx < self.rope.len_lines() {
                let line = self.rope.line(line_idx).to_string();
                let highlights: Vec<(Range<usize>, HighlightKind)> =
                    highlighter.highlight_line(&line).collect();

                // Ensure cache is large enough
                while self.highlight_cache.len() <= line_idx {
                    self.highlight_cache.push(Vec::new());
                }
                self.highlight_cache[line_idx] = highlights;
            }
        }
    }

    /// Handle a message and update state
    pub fn update(&mut self, message: CodeEditorMessage) {
        // Reset cursor blink on any action
        self.cursor_visible = true;

        match message {
            CodeEditorMessage::Input(ch) => {
                // Delete selection first if any
                self.delete_selection();
                self.clear_selection();

                // Handle auto-dedent for closing brace
                if ch == '}' {
                    self.handle_close_brace_dedent();
                }

                self.insert_char(ch);
            }
            CodeEditorMessage::Backspace => {
                if !self.delete_selection() {
                    self.delete_backward();
                }
                self.clear_selection();
            }
            CodeEditorMessage::Delete => {
                if !self.delete_selection() {
                    self.delete_forward();
                }
                self.clear_selection();
            }
            CodeEditorMessage::Enter => {
                self.delete_selection();
                self.clear_selection();
                self.insert_newline_with_indent();
            }
            CodeEditorMessage::Tab => {
                self.delete_selection();
                self.clear_selection();
                self.insert_str("    ");
            }
            CodeEditorMessage::MoveCursor(movement) => {
                self.clear_selection();
                self.move_cursor(movement);
                self.ensure_cursor_visible();
            }
            CodeEditorMessage::MoveCursorSelect(movement) => {
                self.start_selection();
                self.move_cursor(movement);
                self.update_selection_from_anchor();
                self.ensure_cursor_visible();
            }
            CodeEditorMessage::Click(point) => {
                let gutter_width = self.gutter_width();
                if point.x > gutter_width {
                    let pos = self.point_to_position(point, gutter_width);

                    // Check for double/triple click
                    let now = Instant::now();
                    let is_same_pos = self.last_click_pos.map_or(false, |p| {
                        p.line == pos.line && (p.column as i32 - pos.column as i32).abs() <= 1
                    });
                    let is_quick_click = self.last_click_time.map_or(false, |t| {
                        now.duration_since(t).as_millis() < 400
                    });

                    if is_same_pos && is_quick_click {
                        self.click_count = (self.click_count % 3) + 1;
                    } else {
                        self.click_count = 1;
                    }

                    self.last_click_time = Some(now);
                    self.last_click_pos = Some(pos);

                    match self.click_count {
                        1 => {
                            // Single click: position cursor
                            self.move_cursor_to(pos);
                            self.clear_selection();
                            self.selection_anchor = Some(pos);
                            self.dragging = true;
                        }
                        2 => {
                            // Double click: select word
                            self.select_word_at(pos);
                            self.dragging = false;
                        }
                        3 => {
                            // Triple click: select line
                            self.select_line_at(pos);
                            self.dragging = false;
                        }
                        _ => {}
                    }
                }
            }
            CodeEditorMessage::Drag(point) => {
                if self.dragging {
                    let gutter_width = self.gutter_width();
                    let pos = self.point_to_position(point, gutter_width);
                    self.cursor = pos;
                    self.update_selection_from_anchor();
                }
            }
            CodeEditorMessage::Release => {
                self.dragging = false;
                // Clear anchor if no selection was made
                if self.selection.as_ref().map_or(true, |s| s.is_empty()) {
                    self.clear_selection();
                }
            }
            CodeEditorMessage::DoubleClick(point) => {
                let gutter_width = self.gutter_width();
                if point.x > gutter_width {
                    let pos = self.point_to_position(point, gutter_width);
                    self.select_word_at(pos);
                }
            }
            CodeEditorMessage::TripleClick(point) => {
                let gutter_width = self.gutter_width();
                if point.x > gutter_width {
                    let pos = self.point_to_position(point, gutter_width);
                    self.select_line_at(pos);
                }
            }
            CodeEditorMessage::Select(selection) => {
                self.selection = Some(selection);
                self.cursor = selection.end;
                self.cache.clear();
            }
            CodeEditorMessage::ContentChanged(text) => {
                self.set_text(&text);
            }
            CodeEditorMessage::Scroll(offset) => {
                self.scroll_offset.x += offset.x;
                self.scroll_offset.y += offset.y;
                self.clamp_scroll();
                self.cache.clear();
            }
            CodeEditorMessage::StartDragVScrollbar(point) => {
                self.dragging_v_scrollbar = true;
                self.scrollbar_drag_start = Some(point);
                self.scroll_offset_drag_start = self.scroll_offset;
            }
            CodeEditorMessage::StartDragHScrollbar(point) => {
                self.dragging_h_scrollbar = true;
                self.scrollbar_drag_start = Some(point);
                self.scroll_offset_drag_start = self.scroll_offset;
            }
            CodeEditorMessage::DragScrollbar(point) => {
                if let Some(start) = self.scrollbar_drag_start {
                    if self.dragging_v_scrollbar {
                        let delta_y = point.y - start.y;
                        let max_scroll = self.max_scroll();
                        let content_h = self.content_height();
                        let visible_h = self.text_area_height();
                        let track_h = self.viewport_size.height
                            - if self.needs_horizontal_scrollbar() { SCROLLBAR_WIDTH } else { 0.0 };
                        let thumb_ratio = (visible_h / content_h).min(1.0);
                        let thumb_h = (track_h * thumb_ratio).max(SCROLLBAR_MIN_THUMB_SIZE);
                        let scrollable_track = track_h - thumb_h;

                        if scrollable_track > 0.0 {
                            let scroll_ratio = delta_y / scrollable_track;
                            self.scroll_offset.y = self.scroll_offset_drag_start.y + scroll_ratio * max_scroll.y;
                        }
                    } else if self.dragging_h_scrollbar {
                        let delta_x = point.x - start.x;
                        let max_scroll = self.max_scroll();
                        let content_w = self.max_line_width();
                        let visible_w = self.text_area_width();
                        let gutter = self.gutter_width();
                        let track_w = self.viewport_size.width - gutter
                            - if self.needs_vertical_scrollbar() { SCROLLBAR_WIDTH } else { 0.0 };
                        let thumb_ratio = (visible_w / content_w).min(1.0);
                        let thumb_w = (track_w * thumb_ratio).max(SCROLLBAR_MIN_THUMB_SIZE);
                        let scrollable_track = track_w - thumb_w;

                        if scrollable_track > 0.0 {
                            let scroll_ratio = delta_x / scrollable_track;
                            self.scroll_offset.x = self.scroll_offset_drag_start.x + scroll_ratio * max_scroll.x;
                        }
                    }
                    self.clamp_scroll();
                    self.cache.clear();
                }
            }
            CodeEditorMessage::ReleaseScrollbar => {
                self.dragging_v_scrollbar = false;
                self.dragging_h_scrollbar = false;
                self.scrollbar_drag_start = None;
            }
            CodeEditorMessage::ClickVScrollbarTrack(y) => {
                // Page scroll: scroll by viewport height in direction of click
                let (_, thumb) = self.vertical_scrollbar_geometry(Rectangle::new(
                    Point::ORIGIN,
                    self.viewport_size,
                ));
                let page_size = self.text_area_height();
                if y < thumb.y {
                    self.scroll_offset.y -= page_size;
                } else {
                    self.scroll_offset.y += page_size;
                }
                self.clamp_scroll();
                self.cache.clear();
            }
            CodeEditorMessage::ClickHScrollbarTrack(x) => {
                let (_, thumb) = self.horizontal_scrollbar_geometry(Rectangle::new(
                    Point::ORIGIN,
                    self.viewport_size,
                ));
                let page_size = self.text_area_width();
                if x < thumb.x {
                    self.scroll_offset.x -= page_size;
                } else {
                    self.scroll_offset.x += page_size;
                }
                self.clamp_scroll();
                self.cache.clear();
            }
            CodeEditorMessage::CursorBlink => {
                self.toggle_cursor_blink();
            }
            CodeEditorMessage::Focus => {
                self.set_focused(true);
            }
            CodeEditorMessage::Blur => {
                self.set_focused(false);
                self.dragging = false;
                self.dragging_v_scrollbar = false;
                self.dragging_h_scrollbar = false;
            }
            CodeEditorMessage::ViewportResized(size) => {
                self.set_viewport_size(size);
            }
            CodeEditorMessage::Undo => {
                self.undo();
            }
            CodeEditorMessage::Redo => {
                self.redo();
            }
        }

        // Update matching bracket after any state change
        self.update_matching_bracket();
    }
}

/// Canvas program for rendering the code editor
struct CodeEditorProgram<'a> {
    state: &'a CodeEditorState,
}

/// State for tracking drag operations in the canvas program
#[derive(Debug, Default)]
pub struct CanvasState {
    /// Is mouse button currently pressed (for text selection)?
    mouse_pressed: bool,
    /// Is vertical scrollbar being dragged?
    dragging_v_scrollbar: bool,
    /// Is horizontal scrollbar being dragged?
    dragging_h_scrollbar: bool,
}

impl<'a> canvas::Program<CodeEditorMessage> for CodeEditorProgram<'a> {
    type State = CanvasState;

    fn update(
        &self,
        state: &mut Self::State,
        event: &Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> Option<canvas::Action<CodeEditorMessage>> {
        match event {
            // Mouse events
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(position) = cursor.position_in(bounds) {
                    // Check if clicking on vertical scrollbar
                    if self.state.is_in_vertical_scrollbar(position, bounds) {
                        let (_, thumb) = self.state.vertical_scrollbar_geometry(bounds);
                        if thumb.contains(position) {
                            // Clicking on thumb - start drag
                            state.dragging_v_scrollbar = true;
                            return Some(canvas::Action::publish(
                                CodeEditorMessage::StartDragVScrollbar(position)
                            ).and_capture());
                        } else {
                            // Clicking on track - page scroll
                            return Some(canvas::Action::publish(
                                CodeEditorMessage::ClickVScrollbarTrack(position.y)
                            ));
                        }
                    }
                    // Check if clicking on horizontal scrollbar
                    if self.state.is_in_horizontal_scrollbar(position, bounds) {
                        let (_, thumb) = self.state.horizontal_scrollbar_geometry(bounds);
                        if thumb.contains(position) {
                            // Clicking on thumb - start drag
                            state.dragging_h_scrollbar = true;
                            return Some(canvas::Action::publish(
                                CodeEditorMessage::StartDragHScrollbar(position)
                            ).and_capture());
                        } else {
                            // Clicking on track - page scroll
                            return Some(canvas::Action::publish(
                                CodeEditorMessage::ClickHScrollbarTrack(position.x)
                            ));
                        }
                    }
                    // Otherwise, text selection click
                    state.mouse_pressed = true;
                    return Some(canvas::Action::publish(CodeEditorMessage::Click(position)).and_capture());
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                if state.dragging_v_scrollbar || state.dragging_h_scrollbar {
                    state.dragging_v_scrollbar = false;
                    state.dragging_h_scrollbar = false;
                    return Some(canvas::Action::publish(CodeEditorMessage::ReleaseScrollbar));
                }
                if state.mouse_pressed {
                    state.mouse_pressed = false;
                    return Some(canvas::Action::publish(CodeEditorMessage::Release));
                }
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                // Handle scrollbar drag
                if state.dragging_v_scrollbar || state.dragging_h_scrollbar {
                    if let Some(position) = cursor.position_in(bounds) {
                        return Some(canvas::Action::publish(CodeEditorMessage::DragScrollbar(position)));
                    }
                }
                // Handle text selection drag
                if state.mouse_pressed {
                    if let Some(position) = cursor.position_in(bounds) {
                        return Some(canvas::Action::publish(CodeEditorMessage::Drag(position)));
                    }
                }
            }
            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if cursor.is_over(bounds) {
                    let scroll = match delta {
                        mouse::ScrollDelta::Lines { x, y } => {
                            Vector::new(-x * CHAR_WIDTH * 3.0, -y * LINE_HEIGHT * 3.0)
                        }
                        mouse::ScrollDelta::Pixels { x, y } => Vector::new(-x, -y),
                    };
                    return Some(canvas::Action::publish(CodeEditorMessage::Scroll(scroll)));
                }
            }

            // Keyboard events
            Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                // Only handle if we're focused (state is tracked externally)
                if !self.state.focused {
                    return None;
                }

                let shift = modifiers.shift();
                let ctrl_or_cmd = modifiers.command() || modifiers.control();

                match key {
                    Key::Named(keyboard::key::Named::ArrowLeft) => {
                        let movement = if ctrl_or_cmd {
                            CursorMovement::WordLeft
                        } else {
                            CursorMovement::Left
                        };
                        return Some(canvas::Action::publish(if shift {
                            CodeEditorMessage::MoveCursorSelect(movement)
                        } else {
                            CodeEditorMessage::MoveCursor(movement)
                        }));
                    }
                    Key::Named(keyboard::key::Named::ArrowRight) => {
                        let movement = if ctrl_or_cmd {
                            CursorMovement::WordRight
                        } else {
                            CursorMovement::Right
                        };
                        return Some(canvas::Action::publish(if shift {
                            CodeEditorMessage::MoveCursorSelect(movement)
                        } else {
                            CodeEditorMessage::MoveCursor(movement)
                        }));
                    }
                    Key::Named(keyboard::key::Named::ArrowUp) => {
                        return Some(canvas::Action::publish(if shift {
                            CodeEditorMessage::MoveCursorSelect(CursorMovement::Up)
                        } else {
                            CodeEditorMessage::MoveCursor(CursorMovement::Up)
                        }));
                    }
                    Key::Named(keyboard::key::Named::ArrowDown) => {
                        return Some(canvas::Action::publish(if shift {
                            CodeEditorMessage::MoveCursorSelect(CursorMovement::Down)
                        } else {
                            CodeEditorMessage::MoveCursor(CursorMovement::Down)
                        }));
                    }
                    Key::Named(keyboard::key::Named::Home) => {
                        let movement = if ctrl_or_cmd {
                            CursorMovement::DocumentStart
                        } else {
                            CursorMovement::Home
                        };
                        return Some(canvas::Action::publish(if shift {
                            CodeEditorMessage::MoveCursorSelect(movement)
                        } else {
                            CodeEditorMessage::MoveCursor(movement)
                        }));
                    }
                    Key::Named(keyboard::key::Named::End) => {
                        let movement = if ctrl_or_cmd {
                            CursorMovement::DocumentEnd
                        } else {
                            CursorMovement::End
                        };
                        return Some(canvas::Action::publish(if shift {
                            CodeEditorMessage::MoveCursorSelect(movement)
                        } else {
                            CodeEditorMessage::MoveCursor(movement)
                        }));
                    }
                    Key::Named(keyboard::key::Named::PageUp) => {
                        return Some(canvas::Action::publish(if shift {
                            CodeEditorMessage::MoveCursorSelect(CursorMovement::PageUp)
                        } else {
                            CodeEditorMessage::MoveCursor(CursorMovement::PageUp)
                        }));
                    }
                    Key::Named(keyboard::key::Named::PageDown) => {
                        return Some(canvas::Action::publish(if shift {
                            CodeEditorMessage::MoveCursorSelect(CursorMovement::PageDown)
                        } else {
                            CodeEditorMessage::MoveCursor(CursorMovement::PageDown)
                        }));
                    }
                    Key::Named(keyboard::key::Named::Backspace) => {
                        return Some(canvas::Action::publish(CodeEditorMessage::Backspace));
                    }
                    Key::Named(keyboard::key::Named::Delete) => {
                        return Some(canvas::Action::publish(CodeEditorMessage::Delete));
                    }
                    Key::Named(keyboard::key::Named::Enter) => {
                        return Some(canvas::Action::publish(CodeEditorMessage::Enter));
                    }
                    Key::Named(keyboard::key::Named::Tab) => {
                        return Some(canvas::Action::publish(CodeEditorMessage::Tab));
                    }
                    Key::Character(c) => {
                        // Handle Ctrl+] for jump to matching bracket
                        if ctrl_or_cmd && c.as_str() == "]" {
                            return Some(canvas::Action::publish(if shift {
                                CodeEditorMessage::MoveCursorSelect(CursorMovement::MatchingBracket)
                            } else {
                                CodeEditorMessage::MoveCursor(CursorMovement::MatchingBracket)
                            }));
                        }

                        // Handle Ctrl+Z for undo
                        if ctrl_or_cmd && (c.as_str() == "z" || c.as_str() == "Z") && !shift {
                            return Some(canvas::Action::publish(CodeEditorMessage::Undo));
                        }

                        // Handle Ctrl+Y or Ctrl+Shift+Z for redo
                        if ctrl_or_cmd && ((c.as_str() == "y" || c.as_str() == "Y") ||
                            ((c.as_str() == "z" || c.as_str() == "Z") && shift)) {
                            return Some(canvas::Action::publish(CodeEditorMessage::Redo));
                        }

                        // Handle single character input (not control sequences)
                        if !ctrl_or_cmd && c.len() == 1 {
                            if let Some(ch) = c.chars().next() {
                                if !ch.is_control() {
                                    return Some(canvas::Action::publish(CodeEditorMessage::Input(ch)));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Window focus events
            Event::Window(iced::window::Event::Focused) => {
                return Some(canvas::Action::publish(CodeEditorMessage::Focus));
            }
            Event::Window(iced::window::Event::Unfocused) => {
                return Some(canvas::Action::publish(CodeEditorMessage::Blur));
            }

            _ => {}
        }

        None
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let palette = theme.extended_palette();
        let gutter_width = self.state.gutter_width();
        let visible_lines = ((bounds.height / LINE_HEIGHT).ceil() as usize) + 1;
        let first_visible = (self.state.scroll_offset.y / LINE_HEIGHT).floor() as usize;

        let geometry = self.state.cache.draw(renderer, bounds.size(), |frame| {
            // Background
            frame.fill_rectangle(
                Point::ORIGIN,
                bounds.size(),
                palette.background.base.color,
            );

            // Gutter background
            frame.fill_rectangle(
                Point::ORIGIN,
                Size::new(gutter_width, bounds.height),
                palette.background.weak.color,
            );

            // Selection highlight (draw before current line so it's visible)
            if let Some(selection) = self.state.selection {
                let sel = selection.normalized();
                let selection_color = Color::from_rgba(0.3, 0.5, 0.8, 0.3);

                for line_idx in first_visible..first_visible + visible_lines {
                    if line_idx >= self.state.rope.len_lines() {
                        break;
                    }

                    // Check if this line is part of the selection
                    if line_idx < sel.start.line || line_idx > sel.end.line {
                        continue;
                    }

                    let line_len = self.state.line_length(line_idx);
                    let y = ((line_idx - first_visible) as f32) * LINE_HEIGHT + EDITOR_PADDING;

                    let (start_col, end_col) = if sel.start.line == sel.end.line {
                        // Single line selection
                        (sel.start.column, sel.end.column)
                    } else if line_idx == sel.start.line {
                        // First line of multi-line selection
                        (sel.start.column, line_len)
                    } else if line_idx == sel.end.line {
                        // Last line of multi-line selection
                        (0, sel.end.column)
                    } else {
                        // Middle lines of multi-line selection
                        (0, line_len)
                    };

                    if start_col < end_col {
                        let x = gutter_width
                            + EDITOR_PADDING
                            + (start_col as f32) * CHAR_WIDTH
                            - self.state.scroll_offset.x;
                        let width = ((end_col - start_col) as f32) * CHAR_WIDTH;

                        frame.fill_rectangle(
                            Point::new(x.max(gutter_width), y),
                            Size::new(width.min(bounds.width - x.max(gutter_width)), LINE_HEIGHT),
                            selection_color,
                        );
                    }
                }
            }

            // Current line highlight
            let current_line_y =
                (self.state.cursor.line as f32 * LINE_HEIGHT) - self.state.scroll_offset.y + EDITOR_PADDING;
            if current_line_y >= 0.0 && current_line_y < bounds.height {
                let highlight_color = Color::from_rgba(1.0, 1.0, 1.0, 0.05);
                frame.fill_rectangle(
                    Point::new(gutter_width, current_line_y),
                    Size::new(bounds.width - gutter_width, LINE_HEIGHT),
                    highlight_color,
                );
            }

            // Bracket matching highlight
            if let Some(bracket_pair) = self.state.matching_bracket {
                let bracket_color = Color::from_rgba(0.9, 0.8, 0.2, 0.4); // Golden/yellow highlight

                // Helper to draw a bracket highlight at a position
                let draw_bracket_highlight = |frame: &mut iced::widget::canvas::Frame, pos: Position| {
                    // Check if position is in visible range
                    if pos.line >= first_visible && pos.line < first_visible + visible_lines {
                        let y = ((pos.line - first_visible) as f32) * LINE_HEIGHT + EDITOR_PADDING;
                        let x = gutter_width
                            + EDITOR_PADDING
                            + (pos.column as f32) * CHAR_WIDTH
                            - self.state.scroll_offset.x;

                        if x >= gutter_width {
                            frame.fill_rectangle(
                                Point::new(x, y),
                                Size::new(CHAR_WIDTH, LINE_HEIGHT),
                                bracket_color,
                            );
                        }
                    }
                };

                draw_bracket_highlight(frame, bracket_pair.open);
                draw_bracket_highlight(frame, bracket_pair.close);
            }

            // Draw lines
            for (i, line_idx) in (first_visible..first_visible + visible_lines).enumerate() {
                if line_idx >= self.state.rope.len_lines() {
                    break;
                }

                let y = (i as f32) * LINE_HEIGHT + EDITOR_PADDING;

                // Line number
                let line_num = format!("{:>3}", line_idx + 1);
                let line_num_color = if line_idx == self.state.cursor.line {
                    palette.primary.base.text
                } else {
                    Color::from_rgb(0.5, 0.5, 0.5)
                };

                frame.fill_text(Text {
                    content: line_num,
                    position: Point::new(GUTTER_PADDING, y),
                    size: iced::Pixels(14.0),
                    color: line_num_color,
                    font: Font::MONOSPACE,
                    align_x: iced::alignment::Horizontal::Left.into(),
                    align_y: iced::alignment::Vertical::Top.into(),
                    ..Default::default()
                });

                // Line content with syntax highlighting
                let line = self.state.rope.line(line_idx).to_string();
                let x_offset = gutter_width + EDITOR_PADDING - self.state.scroll_offset.x;

                // Get highlights for this line
                let highlights = self
                    .state
                    .highlight_cache
                    .get(line_idx)
                    .cloned()
                    .unwrap_or_default();

                if highlights.is_empty() {
                    // No highlighting, draw plain text
                    frame.fill_text(Text {
                        content: line.trim_end_matches('\n').to_string(),
                        position: Point::new(x_offset, y),
                        size: iced::Pixels(14.0),
                        color: palette.background.base.text,
                        font: Font::MONOSPACE,
                        align_x: iced::alignment::Horizontal::Left.into(),
                        align_y: iced::alignment::Vertical::Top.into(),
                        ..Default::default()
                    });
                } else {
                    // Draw with syntax highlighting
                    let chars: Vec<char> = line.chars().collect();
                    for (range, kind) in highlights {
                        if range.start >= chars.len() {
                            continue;
                        }
                        let end = range.end.min(chars.len());
                        let text: String = chars[range.start..end].iter().collect();
                        let format = highlight_to_format(&kind, theme);
                        let color = format.color.unwrap_or(palette.background.base.text);

                        frame.fill_text(Text {
                            content: text,
                            position: Point::new(
                                x_offset + (range.start as f32) * CHAR_WIDTH,
                                y,
                            ),
                            size: iced::Pixels(14.0),
                            color,
                            font: Font::MONOSPACE,
                            align_x: iced::alignment::Horizontal::Left.into(),
                            align_y: iced::alignment::Vertical::Top.into(),
                            ..Default::default()
                        });
                    }
                }
            }

            // Draw cursor
            if self.state.focused && self.state.cursor_visible {
                let cursor_x = gutter_width
                    + EDITOR_PADDING
                    + (self.state.cursor.column as f32) * CHAR_WIDTH
                    - self.state.scroll_offset.x;
                let cursor_y = (self.state.cursor.line as f32) * LINE_HEIGHT
                    - self.state.scroll_offset.y
                    + EDITOR_PADDING;

                if cursor_y >= 0.0 && cursor_y < bounds.height && cursor_x >= gutter_width {
                    let cursor_path = Path::rectangle(
                        Point::new(cursor_x, cursor_y),
                        Size::new(2.0, LINE_HEIGHT),
                    );
                    frame.fill(&cursor_path, palette.primary.base.color);
                }
            }

            // Draw scrollbars
            let scrollbar_track_color = Color::from_rgba(0.3, 0.3, 0.3, 0.3);
            let scrollbar_thumb_color = Color::from_rgba(0.5, 0.5, 0.5, 0.6);

            // Vertical scrollbar
            if self.state.needs_vertical_scrollbar() {
                let (track, thumb) = self.state.vertical_scrollbar_geometry(bounds);

                // Draw track
                frame.fill_rectangle(
                    Point::new(track.x, track.y),
                    Size::new(track.width, track.height),
                    scrollbar_track_color,
                );

                // Draw thumb
                frame.fill_rectangle(
                    Point::new(thumb.x + 2.0, thumb.y + 1.0),
                    Size::new(thumb.width - 4.0, thumb.height - 2.0),
                    scrollbar_thumb_color,
                );
            }

            // Horizontal scrollbar
            if self.state.needs_horizontal_scrollbar() {
                let (track, thumb) = self.state.horizontal_scrollbar_geometry(bounds);

                // Draw track
                frame.fill_rectangle(
                    Point::new(track.x, track.y),
                    Size::new(track.width, track.height),
                    scrollbar_track_color,
                );

                // Draw thumb
                frame.fill_rectangle(
                    Point::new(thumb.x + 1.0, thumb.y + 2.0),
                    Size::new(thumb.width - 2.0, thumb.height - 4.0),
                    scrollbar_thumb_color,
                );
            }

            // Corner piece (when both scrollbars are visible)
            if self.state.needs_vertical_scrollbar() && self.state.needs_horizontal_scrollbar() {
                frame.fill_rectangle(
                    Point::new(bounds.width - SCROLLBAR_WIDTH, bounds.height - SCROLLBAR_WIDTH),
                    Size::new(SCROLLBAR_WIDTH, SCROLLBAR_WIDTH),
                    scrollbar_track_color,
                );
            }
        });

        vec![geometry]
    }
}

/// Create a code editor element from state
pub fn code_editor(state: &CodeEditorState) -> Element<'_, CodeEditorMessage> {
    Canvas::new(CodeEditorProgram { state })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_editor_state() {
        let state = CodeEditorState::new();
        assert_eq!(state.text(), "");
        assert_eq!(state.cursor, Position::default());
        assert_eq!(state.line_count(), 1);
    }

    #[test]
    fn test_editor_with_text() {
        let state = CodeEditorState::with_text("hello\nworld");
        assert_eq!(state.text(), "hello\nworld");
        assert_eq!(state.line_count(), 2);
        assert_eq!(state.line(0), Some("hello\n".to_string()));
        assert_eq!(state.line(1), Some("world".to_string()));
    }

    #[test]
    fn test_insert_char() {
        let mut state = CodeEditorState::new();
        state.insert_char('a');
        assert_eq!(state.text(), "a");
        assert_eq!(state.cursor.column, 1);

        state.insert_char('b');
        assert_eq!(state.text(), "ab");
        assert_eq!(state.cursor.column, 2);
    }

    #[test]
    fn test_insert_newline() {
        let mut state = CodeEditorState::with_text("hello");
        state.cursor.column = 5;
        state.insert_char('\n');
        assert_eq!(state.text(), "hello\n");
        assert_eq!(state.cursor.line, 1);
        assert_eq!(state.cursor.column, 0);
    }

    #[test]
    fn test_backspace() {
        let mut state = CodeEditorState::with_text("hello");
        state.cursor.column = 5;
        state.delete_backward();
        assert_eq!(state.text(), "hell");
        assert_eq!(state.cursor.column, 4);
    }

    #[test]
    fn test_backspace_at_line_start() {
        let mut state = CodeEditorState::with_text("hello\nworld");
        state.cursor.line = 1;
        state.cursor.column = 0;
        state.delete_backward();
        assert_eq!(state.text(), "helloworld");
        assert_eq!(state.cursor.line, 0);
        assert_eq!(state.cursor.column, 5);
    }

    #[test]
    fn test_cursor_movement() {
        let mut state = CodeEditorState::with_text("hello\nworld");
        state.cursor = Position::new(0, 2);

        state.move_cursor(CursorMovement::Right);
        assert_eq!(state.cursor.column, 3);

        state.move_cursor(CursorMovement::Left);
        assert_eq!(state.cursor.column, 2);

        state.move_cursor(CursorMovement::Down);
        assert_eq!(state.cursor.line, 1);

        state.move_cursor(CursorMovement::Up);
        assert_eq!(state.cursor.line, 0);

        state.move_cursor(CursorMovement::End);
        assert_eq!(state.cursor.column, 5);

        state.move_cursor(CursorMovement::Home);
        assert_eq!(state.cursor.column, 0);
    }

    #[test]
    fn test_selection() {
        let sel = Selection::new(Position::new(0, 5), Position::new(0, 0));
        let normalized = sel.normalized();
        assert_eq!(normalized.start.column, 0);
        assert_eq!(normalized.end.column, 5);
    }

    #[test]
    fn test_line_length() {
        let state = CodeEditorState::with_text("hello\nworld\n");
        assert_eq!(state.line_length(0), 5);
        assert_eq!(state.line_length(1), 5);
    }

    #[test]
    fn test_position_to_char_index() {
        let state = CodeEditorState::with_text("hello\nworld");
        assert_eq!(state.position_to_char_index(Position::new(0, 0)), 0);
        assert_eq!(state.position_to_char_index(Position::new(0, 5)), 5);
        assert_eq!(state.position_to_char_index(Position::new(1, 0)), 6);
        assert_eq!(state.position_to_char_index(Position::new(1, 3)), 9);
    }

    #[test]
    fn test_select_word_at() {
        let mut state = CodeEditorState::with_text("hello world_foo bar");

        // Select "hello"
        state.select_word_at(Position::new(0, 2));
        let sel = state.selection.unwrap().normalized();
        assert_eq!(sel.start, Position::new(0, 0));
        assert_eq!(sel.end, Position::new(0, 5));

        // Select "world_foo" (includes underscore)
        state.select_word_at(Position::new(0, 8));
        let sel = state.selection.unwrap().normalized();
        assert_eq!(sel.start, Position::new(0, 6));
        assert_eq!(sel.end, Position::new(0, 15));
    }

    #[test]
    fn test_select_line_at() {
        let mut state = CodeEditorState::with_text("hello\nworld\nlast");

        // Select first line
        state.select_line_at(Position::new(0, 2));
        let sel = state.selection.unwrap().normalized();
        assert_eq!(sel.start, Position::new(0, 0));
        assert_eq!(sel.end, Position::new(1, 0)); // Goes to start of next line

        // Select last line (no newline after)
        state.select_line_at(Position::new(2, 2));
        let sel = state.selection.unwrap().normalized();
        assert_eq!(sel.start, Position::new(2, 0));
        assert_eq!(sel.end, Position::new(2, 4)); // End of content
    }

    #[test]
    fn test_delete_selection() {
        let mut state = CodeEditorState::with_text("hello world");

        // Select "ello "
        state.selection = Some(Selection::new(
            Position::new(0, 1),
            Position::new(0, 6),
        ));

        assert!(state.delete_selection());
        assert_eq!(state.text(), "hworld");
        assert_eq!(state.cursor, Position::new(0, 1));
        assert!(state.selection.is_none());
    }

    #[test]
    fn test_delete_multiline_selection() {
        let mut state = CodeEditorState::with_text("hello\nworld\nfoo");

        // Select from middle of first line to middle of last line
        state.selection = Some(Selection::new(
            Position::new(0, 3),
            Position::new(2, 2),
        ));

        assert!(state.delete_selection());
        assert_eq!(state.text(), "helo");
        assert_eq!(state.cursor, Position::new(0, 3));
    }

    #[test]
    fn test_move_cursor_with_selection() {
        let mut state = CodeEditorState::with_text("hello world");
        state.cursor = Position::new(0, 0);

        // Start selecting
        state.start_selection();
        state.move_cursor(CursorMovement::Right);
        state.move_cursor(CursorMovement::Right);
        state.move_cursor(CursorMovement::Right);
        state.update_selection_from_anchor();

        let sel = state.selection.unwrap().normalized();
        assert_eq!(sel.start, Position::new(0, 0));
        assert_eq!(sel.end, Position::new(0, 3));
    }

    #[test]
    fn test_input_replaces_selection() {
        let mut state = CodeEditorState::with_text("hello");
        state.selection = Some(Selection::new(
            Position::new(0, 1),
            Position::new(0, 4), // Select "ell"
        ));

        state.update(CodeEditorMessage::Input('X'));
        assert_eq!(state.text(), "hXo");
    }

    #[test]
    fn test_cursor_blink() {
        let mut state = CodeEditorState::new();
        state.set_focused(true);
        assert!(state.cursor_visible);

        state.toggle_cursor_blink();
        assert!(!state.cursor_visible);

        state.toggle_cursor_blink();
        assert!(state.cursor_visible);
    }

    #[test]
    fn test_focus_unfocus() {
        let mut state = CodeEditorState::new();
        assert!(!state.focused);

        state.set_focused(true);
        assert!(state.focused);
        assert!(state.cursor_visible);

        state.set_focused(false);
        assert!(!state.focused);
    }

    // Auto-indentation tests

    #[test]
    fn test_get_line_indentation() {
        let state = CodeEditorState::with_text("    hello\n\tworld\nno indent");
        assert_eq!(state.get_line_indentation(0), "    ");
        assert_eq!(state.get_line_indentation(1), "\t");
        assert_eq!(state.get_line_indentation(2), "");
    }

    #[test]
    fn test_line_ends_with_open_brace() {
        let state = CodeEditorState::with_text("fx main() {\nlet x = 5\n}  \nfx foo() { // comment");
        assert!(state.line_ends_with_open_brace(0));
        assert!(!state.line_ends_with_open_brace(1));
        assert!(!state.line_ends_with_open_brace(2));
        assert!(state.line_ends_with_open_brace(3));
    }

    #[test]
    fn test_newline_preserves_indentation() {
        let mut state = CodeEditorState::with_text("    hello");
        state.cursor = Position::new(0, 9); // End of line
        state.insert_newline_with_indent();

        assert_eq!(state.text(), "    hello\n    ");
        assert_eq!(state.cursor.line, 1);
        assert_eq!(state.cursor.column, 4);
    }

    #[test]
    fn test_newline_adds_indent_after_brace() {
        let mut state = CodeEditorState::with_text("fx main() {");
        state.cursor = Position::new(0, 11); // End of line
        state.insert_newline_with_indent();

        assert_eq!(state.text(), "fx main() {\n    ");
        assert_eq!(state.cursor.line, 1);
        assert_eq!(state.cursor.column, 4);
    }

    #[test]
    fn test_newline_adds_indent_with_existing_indent() {
        let mut state = CodeEditorState::with_text("    if true {");
        state.cursor = Position::new(0, 13); // End of line
        state.insert_newline_with_indent();

        assert_eq!(state.text(), "    if true {\n        ");
        assert_eq!(state.cursor.line, 1);
        assert_eq!(state.cursor.column, 8);
    }

    #[test]
    fn test_close_brace_dedent() {
        let mut state = CodeEditorState::with_text("        "); // 8 spaces
        state.cursor = Position::new(0, 8);

        // Simulate typing '}'
        state.update(CodeEditorMessage::Input('}'));

        assert_eq!(state.text(), "    }"); // Should be 4 spaces + '}'
        assert_eq!(state.cursor.column, 5);
    }

    #[test]
    fn test_close_brace_no_dedent_mid_line() {
        let mut state = CodeEditorState::with_text("    let x = map");
        state.cursor = Position::new(0, 15); // After "map"

        // Simulate typing '}'
        state.update(CodeEditorMessage::Input('}'));

        assert_eq!(state.text(), "    let x = map}"); // No dedent since not at line start
    }

    #[test]
    fn test_is_at_line_start_whitespace() {
        let mut state = CodeEditorState::with_text("    hello");

        state.cursor = Position::new(0, 0);
        assert!(state.is_at_line_start_whitespace());

        state.cursor = Position::new(0, 2);
        assert!(state.is_at_line_start_whitespace());

        state.cursor = Position::new(0, 4);
        assert!(state.is_at_line_start_whitespace());

        state.cursor = Position::new(0, 5);
        assert!(!state.is_at_line_start_whitespace()); // After 'h'
    }

    // Bracket matching tests

    #[test]
    fn test_find_matching_bracket_parentheses() {
        let state = CodeEditorState::with_text("foo(bar)");

        // At opening paren
        let result = state.find_matching_bracket(Position::new(0, 3));
        assert_eq!(result, Some(Position::new(0, 7)));

        // At closing paren
        let result = state.find_matching_bracket(Position::new(0, 7));
        assert_eq!(result, Some(Position::new(0, 3)));
    }

    #[test]
    fn test_find_matching_bracket_braces() {
        let state = CodeEditorState::with_text("fn main() {\n    println!()\n}");

        // At opening brace
        let result = state.find_matching_bracket(Position::new(0, 10));
        assert_eq!(result, Some(Position::new(2, 0)));

        // At closing brace
        let result = state.find_matching_bracket(Position::new(2, 0));
        assert_eq!(result, Some(Position::new(0, 10)));
    }

    #[test]
    fn test_find_matching_bracket_nested() {
        let state = CodeEditorState::with_text("((()))");

        // Outermost opening
        let result = state.find_matching_bracket(Position::new(0, 0));
        assert_eq!(result, Some(Position::new(0, 5)));

        // Middle opening
        let result = state.find_matching_bracket(Position::new(0, 1));
        assert_eq!(result, Some(Position::new(0, 4)));

        // Innermost opening
        let result = state.find_matching_bracket(Position::new(0, 2));
        assert_eq!(result, Some(Position::new(0, 3)));
    }

    #[test]
    fn test_find_matching_bracket_square() {
        let state = CodeEditorState::with_text("arr[i + 1]");

        let result = state.find_matching_bracket(Position::new(0, 3));
        assert_eq!(result, Some(Position::new(0, 9)));
    }

    #[test]
    fn test_find_matching_bracket_no_match() {
        let state = CodeEditorState::with_text("(unclosed");

        // Opening paren with no match
        let result = state.find_matching_bracket(Position::new(0, 0));
        assert_eq!(result, None);
    }

    #[test]
    fn test_find_matching_bracket_not_on_bracket() {
        let state = CodeEditorState::with_text("hello");

        let result = state.find_matching_bracket(Position::new(0, 2));
        assert_eq!(result, None);
    }

    #[test]
    fn test_update_matching_bracket_at_cursor() {
        let mut state = CodeEditorState::with_text("(hello)");

        // Move cursor to opening paren
        state.cursor = Position::new(0, 0);
        state.update_matching_bracket();

        assert!(state.matching_bracket.is_some());
        let pair = state.matching_bracket.unwrap();
        assert_eq!(pair.open, Position::new(0, 0));
        assert_eq!(pair.close, Position::new(0, 6));
    }

    #[test]
    fn test_update_matching_bracket_after_cursor() {
        let mut state = CodeEditorState::with_text("(hello)");

        // Move cursor right after closing paren (common position)
        state.cursor = Position::new(0, 7);
        state.update_matching_bracket();

        assert!(state.matching_bracket.is_some());
        let pair = state.matching_bracket.unwrap();
        assert_eq!(pair.open, Position::new(0, 0));
        assert_eq!(pair.close, Position::new(0, 6));
    }

    #[test]
    fn test_jump_to_matching_bracket() {
        let mut state = CodeEditorState::with_text("fn test() {\n    body\n}");

        // Position cursor at opening brace
        state.cursor = Position::new(0, 10);
        state.move_cursor(CursorMovement::MatchingBracket);

        assert_eq!(state.cursor, Position::new(2, 0));

        // Jump back
        state.move_cursor(CursorMovement::MatchingBracket);
        assert_eq!(state.cursor, Position::new(0, 10));
    }

    #[test]
    fn test_no_matching_bracket_when_not_on_bracket() {
        let mut state = CodeEditorState::with_text("hello world");

        state.cursor = Position::new(0, 5);
        state.update_matching_bracket();

        assert!(state.matching_bracket.is_none());
    }

    // Scroll functionality tests

    #[test]
    fn test_scroll_clamp_prevents_negative() {
        let mut state = CodeEditorState::with_text("hello\nworld");
        state.set_viewport_size(Size::new(800.0, 600.0));

        // Try to scroll negative
        state.update(CodeEditorMessage::Scroll(Vector::new(-100.0, -100.0)));

        // Should be clamped to 0
        assert_eq!(state.scroll_offset.x, 0.0);
        assert_eq!(state.scroll_offset.y, 0.0);
    }

    #[test]
    fn test_scroll_clamp_prevents_past_content() {
        let mut state = CodeEditorState::with_text("short");
        state.set_viewport_size(Size::new(800.0, 600.0));

        // Try to scroll way past content
        state.update(CodeEditorMessage::Scroll(Vector::new(10000.0, 10000.0)));

        // Should be clamped to max scroll (which is 0 for small content)
        assert_eq!(state.scroll_offset.x, 0.0);
        assert_eq!(state.scroll_offset.y, 0.0);
    }

    #[test]
    fn test_content_height_calculation() {
        let state = CodeEditorState::with_text("line1\nline2\nline3\nline4\nline5");
        // 5 lines * LINE_HEIGHT (20.0)
        assert_eq!(state.content_height(), 100.0);
    }

    #[test]
    fn test_max_line_width_calculation() {
        let state = CodeEditorState::with_text("short\nthis is a longer line\nmed");
        // "this is a longer line" = 21 chars * CHAR_WIDTH (8.4)
        assert!((state.max_line_width() - 21.0 * 8.4).abs() < 0.01);
    }

    #[test]
    fn test_ensure_cursor_visible_scrolls_down() {
        let mut state = CodeEditorState::with_text(
            "line1\nline2\nline3\nline4\nline5\nline6\nline7\nline8\nline9\nline10"
        );
        state.set_viewport_size(Size::new(400.0, 100.0)); // Small viewport

        // Move cursor to line 9 (0-indexed as 8)
        state.cursor = Position::new(8, 0);
        state.ensure_cursor_visible();

        // Should have scrolled down
        assert!(state.scroll_offset.y > 0.0);
    }

    #[test]
    fn test_ensure_cursor_visible_scrolls_right() {
        let mut state = CodeEditorState::with_text(
            "this is a very long line that extends beyond the viewport width by a lot of characters"
        );
        state.set_viewport_size(Size::new(200.0, 100.0)); // Small viewport

        // Move cursor to end of line
        state.cursor = Position::new(0, 80);
        state.ensure_cursor_visible();

        // Should have scrolled right
        assert!(state.scroll_offset.x > 0.0);
    }

    #[test]
    fn test_viewport_resize_clamps_scroll() {
        let mut state = CodeEditorState::with_text("line1\nline2\nline3");
        state.set_viewport_size(Size::new(100.0, 30.0)); // Small viewport
        state.scroll_offset.y = 50.0; // Some scroll

        // Resize to larger viewport where no scroll needed
        state.set_viewport_size(Size::new(800.0, 600.0));

        // Scroll should be clamped
        assert_eq!(state.scroll_offset.y, 0.0);
    }

    #[test]
    fn test_needs_scrollbar_detection() {
        // Small content, no scrollbars needed
        let mut state = CodeEditorState::with_text("hello");
        state.set_viewport_size(Size::new(800.0, 600.0));
        assert!(!state.needs_vertical_scrollbar());
        assert!(!state.needs_horizontal_scrollbar());

        // Tall content, vertical scrollbar needed
        let mut state = CodeEditorState::with_text(
            &(0..100).map(|i| format!("line{}", i)).collect::<Vec<_>>().join("\n")
        );
        state.set_viewport_size(Size::new(800.0, 200.0));
        assert!(state.needs_vertical_scrollbar());

        // Wide content, horizontal scrollbar needed
        let mut state = CodeEditorState::with_text(
            "this is a very very very very very very very very very very very very long line"
        );
        state.set_viewport_size(Size::new(100.0, 600.0));
        assert!(state.needs_horizontal_scrollbar());
    }

    #[test]
    fn test_cursor_movement_triggers_ensure_visible() {
        let mut state = CodeEditorState::with_text(
            &(0..50).map(|i| format!("line{}", i)).collect::<Vec<_>>().join("\n")
        );
        state.set_viewport_size(Size::new(400.0, 100.0));
        state.cursor = Position::new(0, 0);
        state.scroll_offset = Vector::new(0.0, 0.0);

        // Move cursor down many times
        for _ in 0..40 {
            state.update(CodeEditorMessage::MoveCursor(CursorMovement::Down));
        }

        // Should have scrolled to keep cursor visible
        assert!(state.scroll_offset.y > 0.0);
        assert_eq!(state.cursor.line, 40);
    }

    // Undo/Redo tests

    #[test]
    fn test_undo_insert_char() {
        let mut state = CodeEditorState::new();
        state.insert_char('a');
        assert_eq!(state.text(), "a");
        assert!(state.can_undo());
        assert!(!state.can_redo());

        state.undo();
        assert_eq!(state.text(), "");
        assert!(!state.can_undo());
        assert!(state.can_redo());
    }

    #[test]
    fn test_redo_insert_char() {
        let mut state = CodeEditorState::new();
        state.insert_char('a');
        state.undo();
        assert_eq!(state.text(), "");

        state.redo();
        assert_eq!(state.text(), "a");
        assert!(state.can_undo());
        assert!(!state.can_redo());
    }

    #[test]
    fn test_undo_backspace() {
        let mut state = CodeEditorState::with_text("hello");
        state.cursor = Position::new(0, 5);
        state.delete_backward();
        assert_eq!(state.text(), "hell");

        state.undo();
        assert_eq!(state.text(), "hello");
        assert_eq!(state.cursor, Position::new(0, 5));
    }

    #[test]
    fn test_undo_delete_forward() {
        let mut state = CodeEditorState::with_text("hello");
        state.cursor = Position::new(0, 0);
        state.delete_forward();
        assert_eq!(state.text(), "ello");

        state.undo();
        assert_eq!(state.text(), "hello");
        assert_eq!(state.cursor, Position::new(0, 0));
    }

    #[test]
    fn test_undo_delete_selection() {
        let mut state = CodeEditorState::with_text("hello world");
        state.selection = Some(Selection::new(
            Position::new(0, 0),
            Position::new(0, 6),
        ));
        state.delete_selection();
        assert_eq!(state.text(), "world");

        state.undo();
        assert_eq!(state.text(), "hello world");
    }

    #[test]
    fn test_new_edit_clears_redo_stack() {
        let mut state = CodeEditorState::new();
        state.insert_char('a');
        state.undo();
        assert!(state.can_redo());

        // New edit should clear redo stack
        state.insert_char('b');
        assert!(!state.can_redo());
    }

    #[test]
    fn test_undo_via_message() {
        let mut state = CodeEditorState::new();
        state.update(CodeEditorMessage::Input('a'));
        state.update(CodeEditorMessage::Input('b'));
        assert_eq!(state.text(), "ab");

        state.update(CodeEditorMessage::Undo);
        // Note: rapid edits may be grouped, so we check the text is shorter
        assert!(state.text().len() < 2 || state.text() == "a");
    }

    #[test]
    fn test_undo_preserves_cursor_position() {
        let mut state = CodeEditorState::with_text("hello");
        let original_cursor = Position::new(0, 5);
        state.cursor = original_cursor;

        state.insert_char('!');
        assert_eq!(state.cursor, Position::new(0, 6));

        state.undo();
        assert_eq!(state.cursor, original_cursor);
    }

    #[test]
    fn test_multiple_undo_redo() {
        let mut state = CodeEditorState::new();

        // Add some text with pauses to prevent grouping
        state.insert_char('a');
        // Force new group by making last_edit_time old
        state.last_edit_time = None;
        state.insert_char('b');
        state.last_edit_time = None;
        state.insert_char('c');

        assert_eq!(state.text(), "abc");

        // Undo all three
        state.undo();
        state.undo();
        state.undo();
        assert_eq!(state.text(), "");

        // Redo all three
        state.redo();
        state.redo();
        state.redo();
        assert_eq!(state.text(), "abc");
    }

    #[test]
    fn test_undo_newline() {
        let mut state = CodeEditorState::with_text("hello");
        state.cursor = Position::new(0, 5);
        state.insert_char('\n');
        assert_eq!(state.line_count(), 2);

        state.undo();
        assert_eq!(state.text(), "hello");
        assert_eq!(state.line_count(), 1);
    }
}
