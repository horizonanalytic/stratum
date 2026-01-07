//! Window management for Stratum GUI
//!
//! This module provides types and functionality for managing windows,
//! including multi-window support and window configuration.

use std::collections::BTreeMap;

pub use iced::window::Id as IcedWindowId;

/// Unique identifier for a window
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WindowId(IcedWindowId);

impl WindowId {
    /// Create a new WindowId from an iced window ID
    #[must_use]
    pub fn from_iced(id: IcedWindowId) -> Self {
        Self(id)
    }

    /// Get the underlying iced window ID
    #[must_use]
    pub fn to_iced(self) -> IcedWindowId {
        self.0
    }

    /// Create a new unique window ID
    #[must_use]
    pub fn unique() -> Self {
        Self(IcedWindowId::unique())
    }
}

impl From<IcedWindowId> for WindowId {
    fn from(id: IcedWindowId) -> Self {
        Self(id)
    }
}

impl From<WindowId> for IcedWindowId {
    fn from(id: WindowId) -> Self {
        id.0
    }
}

/// Window position on screen
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Position {
    /// Let the window manager decide
    Default,
    /// Center the window on the primary monitor
    Centered,
    /// Specific position (x, y) in logical pixels
    Specific(f32, f32),
}

impl Default for Position {
    fn default() -> Self {
        Self::Default
    }
}

impl Position {
    /// Convert to iced Position
    #[must_use]
    pub fn to_iced(self) -> iced::window::Position {
        match self {
            Self::Default => iced::window::Position::Default,
            Self::Centered => iced::window::Position::Centered,
            Self::Specific(x, y) => {
                iced::window::Position::Specific(iced::Point::new(x, y))
            }
        }
    }
}

/// Window level (z-order)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WindowLevel {
    /// Normal window
    #[default]
    Normal,
    /// Always on top of other windows
    AlwaysOnTop,
    /// Always below other windows
    AlwaysOnBottom,
}

impl WindowLevel {
    /// Convert to iced Level
    #[must_use]
    pub fn to_iced(self) -> iced::window::Level {
        match self {
            Self::Normal => iced::window::Level::Normal,
            Self::AlwaysOnTop => iced::window::Level::AlwaysOnTop,
            Self::AlwaysOnBottom => iced::window::Level::AlwaysOnBottom,
        }
    }
}

/// Full window configuration
#[derive(Debug, Clone)]
pub struct WindowSettings {
    /// Window title
    pub title: String,
    /// Initial size (width, height) in logical pixels
    pub size: (u32, u32),
    /// Minimum size constraint
    pub min_size: Option<(u32, u32)>,
    /// Maximum size constraint
    pub max_size: Option<(u32, u32)>,
    /// Initial position
    pub position: Position,
    /// Whether the window is resizable
    pub resizable: bool,
    /// Whether the window has decorations (title bar, borders)
    pub decorations: bool,
    /// Whether the window is visible on creation
    pub visible: bool,
    /// Whether the window is transparent
    pub transparent: bool,
    /// Window level (z-order)
    pub level: WindowLevel,
    /// Whether to exit application when this window closes
    pub exit_on_close: bool,
}

impl Default for WindowSettings {
    fn default() -> Self {
        Self {
            title: "Stratum App".to_string(),
            size: (800, 600),
            min_size: None,
            max_size: None,
            position: Position::Default,
            resizable: true,
            decorations: true,
            visible: true,
            transparent: false,
            level: WindowLevel::Normal,
            exit_on_close: true,
        }
    }
}

impl WindowSettings {
    /// Create new settings with a title
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Default::default()
        }
    }

    /// Set the window size
    #[must_use]
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.size = (width, height);
        self
    }

    /// Set minimum size
    #[must_use]
    pub fn with_min_size(mut self, width: u32, height: u32) -> Self {
        self.min_size = Some((width, height));
        self
    }

    /// Set maximum size
    #[must_use]
    pub fn with_max_size(mut self, width: u32, height: u32) -> Self {
        self.max_size = Some((width, height));
        self
    }

    /// Set the window position
    #[must_use]
    pub fn with_position(mut self, position: Position) -> Self {
        self.position = position;
        self
    }

    /// Center the window
    #[must_use]
    pub fn centered(mut self) -> Self {
        self.position = Position::Centered;
        self
    }

    /// Set whether the window is resizable
    #[must_use]
    pub fn with_resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Set whether the window has decorations
    #[must_use]
    pub fn with_decorations(mut self, decorations: bool) -> Self {
        self.decorations = decorations;
        self
    }

    /// Set whether the window is visible
    #[must_use]
    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Set whether the window is transparent
    #[must_use]
    pub fn with_transparent(mut self, transparent: bool) -> Self {
        self.transparent = transparent;
        self
    }

    /// Set the window level
    #[must_use]
    pub fn with_level(mut self, level: WindowLevel) -> Self {
        self.level = level;
        self
    }

    /// Set whether closing this window exits the application
    #[must_use]
    pub fn with_exit_on_close(mut self, exit: bool) -> Self {
        self.exit_on_close = exit;
        self
    }

    /// Convert to iced window settings
    #[must_use]
    pub fn to_iced(&self) -> iced::window::Settings {
        let size = iced::Size::new(self.size.0 as f32, self.size.1 as f32);

        iced::window::Settings {
            size,
            position: self.position.to_iced(),
            min_size: self.min_size.map(|(w, h)| iced::Size::new(w as f32, h as f32)),
            max_size: self.max_size.map(|(w, h)| iced::Size::new(w as f32, h as f32)),
            visible: self.visible,
            resizable: self.resizable,
            decorations: self.decorations,
            transparent: self.transparent,
            level: self.level.to_iced(),
            exit_on_close_request: self.exit_on_close,
            ..Default::default()
        }
    }
}

/// Window state tracked by the window manager
#[derive(Debug, Clone)]
pub struct WindowState {
    /// Window settings
    pub settings: WindowSettings,
    /// Whether the window is focused
    pub focused: bool,
    /// Current size (may differ from settings after resize)
    pub current_size: (u32, u32),
}

impl WindowState {
    /// Create new window state from settings
    #[must_use]
    pub fn new(settings: WindowSettings) -> Self {
        let current_size = settings.size;
        Self {
            settings,
            focused: false,
            current_size,
        }
    }
}

/// Manages multiple windows for a Stratum GUI application
#[derive(Debug, Default)]
pub struct WindowManager {
    /// All tracked windows
    windows: BTreeMap<WindowId, WindowState>,
    /// Counter for naming windows
    window_count: usize,
    /// The ID of the first/main window (set when first window is registered)
    main_window_id: Option<WindowId>,
}

impl WindowManager {
    /// Create a new window manager
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new window with the given ID and settings
    pub fn register(&mut self, id: WindowId, settings: WindowSettings) {
        if self.main_window_id.is_none() {
            self.main_window_id = Some(id);
        }
        self.windows.insert(id, WindowState::new(settings));
        self.window_count += 1;
    }

    /// Register the main window with a generated ID
    /// Returns the generated WindowId
    pub fn register_main(&mut self, settings: WindowSettings) -> WindowId {
        let id = WindowId::unique();
        self.register(id, settings);
        id
    }

    /// Get the main window ID if set
    #[must_use]
    pub fn main_window_id(&self) -> Option<WindowId> {
        self.main_window_id
    }

    /// Remove a window from tracking
    pub fn unregister(&mut self, id: WindowId) -> Option<WindowState> {
        self.windows.remove(&id)
    }

    /// Get a window's state
    #[must_use]
    pub fn get(&self, id: WindowId) -> Option<&WindowState> {
        self.windows.get(&id)
    }

    /// Get a mutable reference to a window's state
    pub fn get_mut(&mut self, id: WindowId) -> Option<&mut WindowState> {
        self.windows.get_mut(&id)
    }

    /// Check if a window exists
    #[must_use]
    pub fn contains(&self, id: WindowId) -> bool {
        self.windows.contains_key(&id)
    }

    /// Get the title for a window
    #[must_use]
    pub fn title(&self, id: WindowId) -> String {
        self.windows
            .get(&id)
            .map(|w| w.settings.title.clone())
            .unwrap_or_default()
    }

    /// Update window focus state
    pub fn set_focused(&mut self, id: WindowId, focused: bool) {
        if let Some(state) = self.windows.get_mut(&id) {
            state.focused = focused;
        }
    }

    /// Update window size after resize
    pub fn set_size(&mut self, id: WindowId, width: u32, height: u32) {
        if let Some(state) = self.windows.get_mut(&id) {
            state.current_size = (width, height);
        }
    }

    /// Get all window IDs
    #[must_use]
    pub fn ids(&self) -> Vec<WindowId> {
        self.windows.keys().copied().collect()
    }

    /// Get the number of windows
    #[must_use]
    pub fn len(&self) -> usize {
        self.windows.len()
    }

    /// Check if there are no windows
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.windows.is_empty()
    }

    /// Get the next window number (for naming)
    #[must_use]
    pub fn next_window_number(&self) -> usize {
        self.window_count + 1
    }

    /// Iterate over all windows
    pub fn iter(&self) -> impl Iterator<Item = (WindowId, &WindowState)> {
        self.windows.iter().map(|(id, state)| (*id, state))
    }
}

/// Window events that can be subscribed to
#[derive(Debug, Clone)]
pub enum WindowEvent {
    /// A window was opened
    Opened(WindowId),
    /// A window close was requested (can be intercepted)
    CloseRequested(WindowId),
    /// A window was closed
    Closed(WindowId),
    /// A window was resized
    Resized {
        id: WindowId,
        width: u32,
        height: u32,
    },
    /// A window gained focus
    Focused(WindowId),
    /// A window lost focus
    Unfocused(WindowId),
    /// A window was moved
    Moved {
        id: WindowId,
        x: i32,
        y: i32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_settings_default() {
        let settings = WindowSettings::default();
        assert_eq!(settings.title, "Stratum App");
        assert_eq!(settings.size, (800, 600));
        assert!(settings.resizable);
        assert!(settings.decorations);
        assert!(settings.visible);
        assert!(!settings.transparent);
    }

    #[test]
    fn test_window_settings_builder() {
        let settings = WindowSettings::new("My App")
            .with_size(1024, 768)
            .with_min_size(640, 480)
            .with_max_size(1920, 1080)
            .centered()
            .with_resizable(false)
            .with_decorations(false);

        assert_eq!(settings.title, "My App");
        assert_eq!(settings.size, (1024, 768));
        assert_eq!(settings.min_size, Some((640, 480)));
        assert_eq!(settings.max_size, Some((1920, 1080)));
        assert_eq!(settings.position, Position::Centered);
        assert!(!settings.resizable);
        assert!(!settings.decorations);
    }

    #[test]
    fn test_position_conversion() {
        let default = Position::Default.to_iced();
        assert!(matches!(default, iced::window::Position::Default));

        let centered = Position::Centered.to_iced();
        assert!(matches!(centered, iced::window::Position::Centered));

        let specific = Position::Specific(100.0, 200.0).to_iced();
        assert!(matches!(specific, iced::window::Position::Specific(_)));
    }

    #[test]
    fn test_window_manager() {
        let mut manager = WindowManager::new();
        assert!(manager.is_empty());

        let settings = WindowSettings::new("Test Window");
        let main_id = manager.register_main(settings);

        assert_eq!(manager.len(), 1);
        assert!(manager.contains(main_id));
        assert_eq!(manager.title(main_id), "Test Window");
        assert_eq!(manager.main_window_id(), Some(main_id));
    }

    #[test]
    fn test_window_manager_multiple() {
        let mut manager = WindowManager::new();

        let main_id = manager.register_main(WindowSettings::new("Main"));

        // Open another window
        let id2 = WindowId::unique();
        manager.register(id2, WindowSettings::new("Secondary"));

        assert_eq!(manager.len(), 2);
        assert_eq!(manager.title(main_id), "Main");
        assert_eq!(manager.title(id2), "Secondary");

        // Main window should still be the first one
        assert_eq!(manager.main_window_id(), Some(main_id));

        // Remove secondary
        manager.unregister(id2);
        assert_eq!(manager.len(), 1);
        assert!(!manager.contains(id2));
    }

    #[test]
    fn test_window_state_updates() {
        let mut manager = WindowManager::new();
        let main_id = manager.register_main(WindowSettings::new("Test").with_size(800, 600));

        // Update focus
        manager.set_focused(main_id, true);
        assert!(manager.get(main_id).unwrap().focused);

        // Update size
        manager.set_size(main_id, 1024, 768);
        assert_eq!(manager.get(main_id).unwrap().current_size, (1024, 768));
    }
}
