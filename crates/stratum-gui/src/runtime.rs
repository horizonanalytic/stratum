//! Core GUI runtime that bridges Stratum values to iced
//!
//! This module implements the main runtime for integrating Stratum
//! with iced's Elm-inspired architecture. Supports both single-window
//! and multi-window applications.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use iced::widget::{button, column, container, row, scrollable, text};
use iced::{window, Center, Color, Element, Fill, Subscription, Task, Theme};

use stratum_core::bytecode::Value;
use stratum_core::VM;

use crate::callback::{Callback, CallbackExecutor, CallbackId, CallbackRegistry};
use crate::element::GuiElement;
use crate::error::{GuiError, GuiResult};
use crate::lifecycle::{LifecycleHooks, LifecycleManager};
use crate::modal::{ModalConfig, ModalManager, ModalResult};
use crate::state::ReactiveState;
use crate::theme::{StratumPalette, StratumTheme, ThemePreset};
use crate::widgets::LayoutConfig;
use crate::window::{WindowId, WindowManager, WindowSettings};

/// Supported GUI backends
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Backend {
    /// iced backend (default, recommended)
    #[default]
    Iced,
}

/// Messages that can be sent to the GUI runtime.
///
/// These correspond to user interactions and state changes.
/// Note: Messages must be `Send` for iced compatibility, so we don't
/// include `Value` directly. Callbacks access state via ReactiveState.
#[derive(Debug, Clone)]
pub enum Message {
    /// Increment a counter (for demos)
    Increment,
    /// Decrement a counter (for demos)
    Decrement,
    /// Set a field to a specific integer value
    SetIntField { field: String, value: i64 },
    /// Set a field to a specific string value
    SetStringField { field: String, value: String },
    /// Set a field to a specific boolean value
    SetBoolField { field: String, value: bool },
    /// Set a field to a specific float value
    SetFloatField { field: String, value: f64 },
    /// TextField value changed - invokes callback with new value
    TextFieldChanged { callback_id: CallbackId, value: String },
    /// Checkbox toggled - invokes callback with new checked state
    CheckboxToggled { callback_id: CallbackId, checked: bool },
    /// Radio button selected - invokes callback with selected value
    RadioButtonSelected { callback_id: CallbackId, value: String },
    /// Dropdown selection changed - invokes callback with selected value
    DropdownSelected { callback_id: CallbackId, value: String },
    /// Slider value changed - invokes callback with new value
    SliderChanged { callback_id: CallbackId, value: f64 },
    /// Toggle switched - invokes callback with new state
    ToggleSwitched { callback_id: CallbackId, is_on: bool },
    /// DataTable column sort requested - invokes callback with column name
    DataTableSort { callback_id: CallbackId, column: String },
    /// DataTable page changed - invokes callback with new page number
    DataTablePageChange { callback_id: CallbackId, page: usize },
    /// DataTable row selection changed - invokes callback with selected row indices
    DataTableRowSelect { callback_id: CallbackId, rows: Vec<usize> },
    /// DataTable row clicked - invokes callback with row index
    DataTableRowClick { callback_id: CallbackId, row: usize },
    /// DataTable cell clicked - invokes callback with row index and column name
    DataTableCellClick {
        callback_id: CallbackId,
        row: usize,
        column: String,
    },
    /// Invoke a registered callback by ID (callback accesses state directly)
    InvokeCallback(CallbackId),
    /// Request application shutdown
    RequestShutdown,
    /// No operation (used for subscriptions that don't produce meaningful messages)
    NoOp,

    // Window events
    /// A new window was opened
    WindowOpened(window::Id),
    /// A window was closed
    WindowClosed(window::Id),
    /// Window focus gained
    WindowFocused(window::Id),
    /// Window focus lost
    WindowBlurred(window::Id),
    /// Window resized
    WindowResized {
        id: window::Id,
        width: u32,
        height: u32,
    },
    /// Request to open a new window
    OpenWindow(WindowSettings),
    /// Request to close a specific window
    CloseWindow(window::Id),

    // Modal events
    /// Show a modal dialog
    ShowModal(ModalConfig),
    /// Modal result received
    ModalResult(ModalResult),
    /// Modal backdrop clicked
    ModalBackdropClicked,

    // OLAP Cube events
    /// Cube drill-down requested on a dimension
    CubeDrillDown {
        callback_id: CallbackId,
        dimension: String,
        value: Option<String>,
    },
    /// Cube roll-up requested on a dimension
    CubeRollUp {
        callback_id: CallbackId,
        dimension: String,
    },
    /// Cube dimension filter changed
    CubeDimensionSelect {
        callback_id: CallbackId,
        dimension: String,
        value: Option<String>,
    },
    /// Cube hierarchy level changed
    CubeHierarchyLevelChange {
        callback_id: CallbackId,
        hierarchy: String,
        level: String,
    },
    /// Cube measure selection changed
    CubeMeasureSelect {
        callback_id: CallbackId,
        measures: Vec<String>,
    },
    /// Cube chart element clicked
    CubeChartClick {
        callback_id: CallbackId,
        dimension: String,
        value: String,
    },
    /// Internal measure toggle (when no callback is registered)
    /// Updates the element's internal selected_measures state
    InternalMeasureToggle {
        /// The measure name being toggled
        measure: String,
        /// Whether the measure is now selected
        selected: bool,
        /// Optional field path to update in state
        field_path: Option<String>,
    },
    /// Internal dimension selection (when no callback is registered)
    /// Updates the element's internal selection state
    InternalDimensionSelect {
        /// The dimension name
        dimension: String,
        /// The selected value (None = "All")
        value: Option<String>,
        /// Optional field path to update in state
        field_path: Option<String>,
    },

    // Theme events
    /// Set application theme by preset name
    SetThemePreset(ThemePreset),
    /// Set custom theme with palette
    SetCustomTheme {
        name: String,
        palette: StratumPalette,
    },

    // Interactive/Mouse events
    /// Mouse button pressed on an element
    MousePress {
        callback_id: CallbackId,
        x: f32,
        y: f32,
    },
    /// Mouse button released on an element
    MouseRelease {
        callback_id: CallbackId,
        x: f32,
        y: f32,
    },
    /// Mouse double-clicked on an element
    MouseDoubleClick {
        callback_id: CallbackId,
        x: f32,
        y: f32,
    },
    /// Right mouse button pressed on an element
    MouseRightPress {
        callback_id: CallbackId,
        x: f32,
        y: f32,
    },
    /// Right mouse button released on an element
    MouseRightRelease {
        callback_id: CallbackId,
        x: f32,
        y: f32,
    },
    /// Middle mouse button pressed on an element
    MouseMiddlePress { callback_id: CallbackId },
    /// Middle mouse button released on an element
    MouseMiddleRelease { callback_id: CallbackId },
    /// Mouse cursor entered an element's area
    MouseEnter { callback_id: CallbackId },
    /// Mouse cursor exited an element's area
    MouseExit { callback_id: CallbackId },
    /// Mouse cursor moved within an element's area
    MouseMove {
        callback_id: CallbackId,
        x: f32,
        y: f32,
    },
    /// Mouse scroll wheel used on an element
    MouseScroll {
        callback_id: CallbackId,
        delta_x: f32,
        delta_y: f32,
    },

    // Keyboard events
    /// Keyboard key pressed
    KeyPressed {
        callback_id: CallbackId,
        key: String,
        modifiers: KeyModifiers,
    },
    /// Keyboard key released
    KeyReleased {
        callback_id: CallbackId,
        key: String,
        modifiers: KeyModifiers,
    },

    // Focus events
    /// Text field focused
    TextFieldFocused { callback_id: CallbackId },
    /// Text field unfocused
    TextFieldUnfocused { callback_id: CallbackId },
    /// Focus a specific widget by ID
    FocusWidget { widget_id: String },

    // File drop events
    /// File(s) being hovered over the window
    FileHovered { paths: Vec<std::path::PathBuf> },
    /// File(s) dropped on the window
    FileDropped { paths: Vec<std::path::PathBuf> },
    /// File hover left the window
    FileHoverLeft,

    // Context menu events
    /// Show context menu at position
    ShowContextMenu {
        x: f32,
        y: f32,
        items: Vec<ContextMenuItem>,
    },
    /// Context menu item selected
    ContextMenuSelect {
        callback_id: CallbackId,
        item_index: usize,
    },
    /// Hide context menu
    HideContextMenu,
}

/// Keyboard modifier keys state
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KeyModifiers {
    /// Shift key is pressed
    pub shift: bool,
    /// Control key is pressed
    pub ctrl: bool,
    /// Alt/Option key is pressed
    pub alt: bool,
    /// Command (Mac) or Windows key is pressed
    pub logo: bool,
}

impl KeyModifiers {
    /// Create new empty modifiers
    #[must_use]
    pub const fn none() -> Self {
        Self {
            shift: false,
            ctrl: false,
            alt: false,
            logo: false,
        }
    }

    /// Check if any modifier is pressed
    #[must_use]
    pub const fn any(&self) -> bool {
        self.shift || self.ctrl || self.alt || self.logo
    }

    /// Convert from iced keyboard modifiers
    #[must_use]
    pub fn from_iced(modifiers: iced::keyboard::Modifiers) -> Self {
        Self {
            shift: modifiers.shift(),
            ctrl: modifiers.control(),
            alt: modifiers.alt(),
            logo: modifiers.logo(),
        }
    }
}

/// A single item in a context menu
#[derive(Debug, Clone)]
pub struct ContextMenuItem {
    /// Display label for the menu item
    pub label: String,
    /// Optional icon name/path
    pub icon: Option<String>,
    /// Callback to invoke when selected
    pub on_select: Option<CallbackId>,
    /// Whether the item is disabled
    pub disabled: bool,
    /// Whether this is a separator
    pub separator: bool,
}

impl ContextMenuItem {
    /// Create a new menu item
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            on_select: None,
            disabled: false,
            separator: false,
        }
    }

    /// Create a separator
    #[must_use]
    pub fn separator() -> Self {
        Self {
            label: String::new(),
            icon: None,
            on_select: None,
            disabled: false,
            separator: true,
        }
    }

    /// Set callback
    #[must_use]
    pub fn on_select(mut self, callback_id: CallbackId) -> Self {
        self.on_select = Some(callback_id);
        self
    }

    /// Set disabled state
    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set icon
    #[must_use]
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }
}

/// Configuration for building a GUI application
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Main window configuration
    pub window: WindowSettings,
    /// Layout configuration
    pub layout: LayoutConfig,
    /// GUI backend to use
    pub backend: Backend,
    /// Theme to use (new theming system)
    pub theme: StratumTheme,
    /// Whether to run in multi-window mode
    pub multi_window: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            window: WindowSettings::default(),
            layout: LayoutConfig {
                spacing: 16.0,
                padding: 20.0,
            },
            backend: Backend::default(),
            theme: StratumTheme::default(),
            multi_window: false,
        }
    }
}

/// Application theme (legacy enum for backwards compatibility)
///
/// For new code, prefer using `StratumTheme` directly which provides
/// access to all 23 built-in themes plus custom theme support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppTheme {
    /// Light theme
    Light,
    /// Dark theme
    #[default]
    Dark,
    /// Follow system preference
    System,
}

impl AppTheme {
    /// Convert to iced Theme (convenience wrapper for to_stratum_theme().to_iced_theme())
    #[allow(dead_code)]
    fn to_iced_theme(self) -> Theme {
        self.to_stratum_theme().to_iced_theme()
    }

    /// Convert to the new StratumTheme type
    #[must_use]
    pub const fn to_stratum_theme(self) -> StratumTheme {
        match self {
            Self::Light => StratumTheme::Preset(ThemePreset::Light),
            Self::Dark => StratumTheme::Preset(ThemePreset::Dark),
            Self::System => StratumTheme::Preset(ThemePreset::System),
        }
    }
}

/// The core GUI runtime that manages the iced application.
///
/// This provides the bridge between Stratum's value system and
/// iced's reactive rendering. Supports both single-window and
/// multi-window applications.
pub struct GuiRuntime {
    /// The current application state
    state: ReactiveState,
    /// Application configuration
    config: AppConfig,
    /// Callback registry
    registry: Rc<RefCell<CallbackRegistry>>,
    /// VM instance for executing callbacks
    vm: Option<Rc<RefCell<VM>>>,
    /// Lifecycle hooks
    lifecycle_hooks: LifecycleHooks,
    /// Root GUI element tree to render (optional - falls back to demo if None)
    root_element: Option<Arc<GuiElement>>,
    /// View function for reactive rendering (Stratum closure that takes state, returns GuiElement)
    view_fn: Option<Arc<Value>>,
}

impl GuiRuntime {
    /// Create a new GUI runtime with the given initial state
    #[must_use]
    pub fn new(initial_state: Value) -> Self {
        Self {
            state: ReactiveState::new(initial_state),
            config: AppConfig::default(),
            registry: Rc::new(RefCell::new(CallbackRegistry::new())),
            vm: None,
            lifecycle_hooks: LifecycleHooks::default(),
            root_element: None,
            view_fn: None,
        }
    }

    /// Set the view function for reactive rendering
    ///
    /// The view function is a Stratum closure that takes the current state
    /// and returns a GuiElement. It is re-invoked whenever state changes
    /// to automatically update the UI.
    #[must_use]
    pub fn with_view_fn(mut self, view_fn: Arc<Value>) -> Self {
        self.view_fn = Some(view_fn);
        self
    }

    /// Set the root GUI element tree to render
    ///
    /// When a root element is provided, the runtime will render it instead
    /// of the default counter demo.
    #[must_use]
    pub fn with_root(mut self, element: GuiElement) -> Self {
        self.root_element = Some(Arc::new(element));
        self
    }

    /// Set the root GUI element tree from an Arc (for sharing)
    #[must_use]
    pub fn with_root_arc(mut self, element: Arc<GuiElement>) -> Self {
        self.root_element = Some(element);
        self
    }

    /// Create a runtime with a VM for callback execution
    #[must_use]
    pub fn with_vm(mut self, vm: VM) -> Self {
        self.vm = Some(Rc::new(RefCell::new(vm)));
        self
    }

    /// Create a runtime with a shared VM reference
    #[must_use]
    pub fn with_shared_vm(mut self, vm: Rc<RefCell<VM>>) -> Self {
        self.vm = Some(vm);
        self
    }

    /// Configure the main window with title and size
    #[must_use]
    pub fn with_window(mut self, title: &str, size: (u32, u32)) -> Self {
        self.config.window.title = title.to_string();
        self.config.window.size = size;
        self
    }

    /// Configure the main window with full settings
    #[must_use]
    pub fn with_window_settings(mut self, settings: WindowSettings) -> Self {
        self.config.window = settings;
        self
    }

    /// Set minimum window size
    #[must_use]
    pub fn with_min_size(mut self, width: u32, height: u32) -> Self {
        self.config.window.min_size = Some((width, height));
        self
    }

    /// Set maximum window size
    #[must_use]
    pub fn with_max_size(mut self, width: u32, height: u32) -> Self {
        self.config.window.max_size = Some((width, height));
        self
    }

    /// Configure layout spacing
    #[must_use]
    pub fn with_spacing(mut self, spacing: f32) -> Self {
        self.config.layout.spacing = spacing;
        self
    }

    /// Configure padding
    #[must_use]
    pub fn with_padding(mut self, padding: f32) -> Self {
        self.config.layout.padding = padding;
        self
    }

    /// Set the theme using the legacy AppTheme enum
    #[must_use]
    pub fn with_theme(mut self, theme: AppTheme) -> Self {
        self.config.theme = theme.to_stratum_theme();
        self
    }

    /// Set the theme using StratumTheme (supports all 23 presets + custom)
    #[must_use]
    pub fn with_stratum_theme(mut self, theme: StratumTheme) -> Self {
        self.config.theme = theme;
        self
    }

    /// Set theme by preset name
    #[must_use]
    pub fn with_theme_preset(mut self, preset: ThemePreset) -> Self {
        self.config.theme = StratumTheme::preset(preset);
        self
    }

    /// Set whether the window is resizable
    #[must_use]
    pub fn with_resizable(mut self, resizable: bool) -> Self {
        self.config.window.resizable = resizable;
        self
    }

    /// Set whether the window has decorations
    #[must_use]
    pub fn with_decorations(mut self, decorations: bool) -> Self {
        self.config.window.decorations = decorations;
        self
    }

    /// Enable multi-window mode
    #[must_use]
    pub fn with_multi_window(mut self, enabled: bool) -> Self {
        self.config.multi_window = enabled;
        self
    }

    /// Set lifecycle hooks
    #[must_use]
    pub fn with_lifecycle_hooks(mut self, hooks: LifecycleHooks) -> Self {
        self.lifecycle_hooks = hooks;
        self
    }

    /// Register a callback and return its ID
    ///
    /// # Errors
    /// Returns an error if the value is not callable
    pub fn register_callback(&self, handler: Value) -> GuiResult<CallbackId> {
        let callback = Callback::new(handler)?;
        Ok(self.registry.borrow_mut().register(callback))
    }

    /// Get the callback registry
    #[must_use]
    pub fn registry(&self) -> &Rc<RefCell<CallbackRegistry>> {
        &self.registry
    }

    /// Get the reactive state
    #[must_use]
    pub fn state(&self) -> &ReactiveState {
        &self.state
    }

    /// Get the current counter value (for the proof-of-concept demo)
    #[allow(dead_code)]
    fn get_count(&self) -> i64 {
        self.state
            .get_field("count")
            .and_then(|v| {
                if let Value::Int(i) = v {
                    Some(i)
                } else {
                    None
                }
            })
            .unwrap_or(0)
    }

    /// Run the GUI application
    ///
    /// This starts the iced event loop and blocks until all windows are closed.
    pub fn run(self) -> GuiResult<()> {
        // Register any pending callbacks from Stratum code (e.g., from view functions)
        // These are stored in thread-local storage by Gui.register_callback()
        // Must be done before extracting values from self
        let pending_callbacks = crate::bindings::take_pending_callbacks();
        for callback in pending_callbacks {
            if let Err(e) = self.register_callback(callback) {
                eprintln!("Warning: Failed to register pending callback: {e}");
            }
        }

        let title = self.config.window.title.clone();
        let theme = self.config.theme;
        let main_window_settings = self.config.window.clone();

        // Create the callback executor if we have a VM
        let executor = self
            .vm
            .as_ref()
            .map(|vm| CallbackExecutor::new(vm.clone(), self.registry.clone()));

        // Create lifecycle manager
        let mut lifecycle = LifecycleManager::new();
        lifecycle.set_hooks(self.lifecycle_hooks.clone());
        if let Some(ref exec) = executor {
            lifecycle.set_executor(exec.clone());
        }

        // Invoke on_init if present
        lifecycle.start().map_err(|e| {
            GuiError::EventHandling(format!("Failed to start lifecycle: {e}"))
        })?;

        // Create initial window manager
        let window_manager = WindowManager::new();

        // Build iced settings from WindowSettings
        let window_size = main_window_settings.size;
        let resizable = main_window_settings.resizable;
        let min_size = main_window_settings.min_size;
        let _max_size = main_window_settings.max_size;

        // Capture values for the boot closure
        // Wrap non-Clone types in Rc<RefCell<Option<T>>> so the closure can implement Fn
        let state = self.state.clone();
        let spacing = self.config.layout.spacing;
        let padding = self.config.layout.padding;
        let registry = self.registry.clone();
        let main_window_settings_clone = main_window_settings.clone();
        let initial_theme = theme.clone();
        let root_element = self.root_element.clone();
        let view_fn = self.view_fn.clone();

        // Wrap types that need to be moved out of the closure
        let executor_cell = Rc::new(RefCell::new(Some(executor)));
        let lifecycle_cell = Rc::new(RefCell::new(Some(lifecycle)));
        let window_manager_cell = Rc::new(RefCell::new(Some(window_manager)));

        // In iced 0.14, application() takes (boot, update, view) where boot returns (State, Task)
        // The boot function must implement Fn (not just FnOnce), so we use Option::take()
        let mut app_builder = iced::application(
            move || {
                // Take values out of cells (this works because boot is only called once)
                // executor is Option<Option<CallbackExecutor>>, flatten it
                let executor = executor_cell.borrow_mut().take().flatten();
                let lifecycle = lifecycle_cell
                    .borrow_mut()
                    .take()
                    .expect("boot should only be called once");
                let mut window_manager = window_manager_cell
                    .borrow_mut()
                    .take()
                    .expect("boot should only be called once");

                // Register main window (we'll get the actual ID from WindowOpened event)
                window_manager.register_main(main_window_settings_clone.clone());

                let app = App {
                    state: state.clone(),
                    spacing,
                    padding,
                    registry: registry.clone(),
                    executor,
                    lifecycle,
                    window_manager,
                    modals: ModalManager::new(),
                    theme: initial_theme.clone(),
                    file_hover_callback: None,
                    file_drop_callback: None,
                    file_hover_left_callback: None,
                    key_press_callback: None,
                    key_release_callback: None,
                    context_menu: None,
                    root_element: root_element.clone(),
                    view_fn: view_fn.clone(),
                    selected_measures: Vec::new(),
                };

                (app, Task::none())
            },
            App::update,
            App::view,
        )
        .title(move |_: &App| title.clone())
        .theme(|app: &App| app.theme.to_iced_theme())
        .subscription(App::subscription)
        .window_size((window_size.0 as f32, window_size.1 as f32))
        .resizable(resizable);

        if let Some((w, h)) = min_size {
            app_builder = app_builder.window(iced::window::Settings {
                min_size: Some(iced::Size::new(w as f32, h as f32)),
                ..Default::default()
            });
        }

        app_builder.run().map_err(|e| GuiError::Iced(e.to_string()))
    }
}

/// The iced application state
struct App {
    state: ReactiveState,
    spacing: f32,
    padding: f32,
    /// Callback registry (callbacks executed via executor, registry kept for future direct access)
    #[allow(dead_code)]
    registry: Rc<RefCell<CallbackRegistry>>,
    executor: Option<CallbackExecutor>,
    lifecycle: LifecycleManager,
    window_manager: WindowManager,
    modals: ModalManager,
    /// Current theme (for runtime theme switching)
    theme: StratumTheme,
    /// Callback for when files are hovered over the window
    file_hover_callback: Option<CallbackId>,
    /// Callback for when files are dropped on the window
    file_drop_callback: Option<CallbackId>,
    /// Callback for when file hover leaves the window
    file_hover_left_callback: Option<CallbackId>,
    /// Callback for keyboard key presses (global)
    key_press_callback: Option<CallbackId>,
    /// Callback for keyboard key releases (global)
    key_release_callback: Option<CallbackId>,
    /// Current context menu state
    context_menu: Option<ContextMenuState>,
    /// Root GUI element tree to render (if provided)
    root_element: Option<Arc<GuiElement>>,
    /// View function for reactive rendering (Stratum closure)
    view_fn: Option<Arc<Value>>,
    /// Internal state for selected measures (when no callback registered)
    selected_measures: Vec<String>,
}

/// State for an active context menu
#[derive(Debug, Clone)]
pub struct ContextMenuState {
    /// X position of the menu
    pub x: f32,
    /// Y position of the menu
    pub y: f32,
    /// Menu items
    pub items: Vec<ContextMenuItem>,
}

impl App {
    /// Re-invoke the view function and update root_element
    /// Called after callbacks execute to reflect state changes in the UI
    fn refresh_view(&mut self) {
        use crate::element::GuiElement;
        use crate::bindings::take_pending_callbacks;
        use crate::callback::Callback;

        // Only refresh if we have both a view_fn and an executor
        if let (Some(ref view_fn), Some(ref executor)) = (&self.view_fn, &self.executor) {
            // Get current state value
            let state_value = self.state.get().clone();

            // Invoke view_fn with current state
            match executor.execute_closure(view_fn.as_ref(), vec![state_value]) {
                Ok(result) => {
                    // Register any new callbacks that were created during view function execution
                    let pending_callbacks = take_pending_callbacks();
                    for callback_value in pending_callbacks {
                        if let Ok(callback) = Callback::new(callback_value) {
                            self.registry.borrow_mut().register(callback);
                        }
                    }

                    // Extract GuiElement from result
                    if let Value::GuiElement(elem) = result {
                        if let Some(gui_elem) = elem.as_any().downcast_ref::<GuiElement>() {
                            self.root_element = Some(Arc::new(gui_elem.clone()));
                        }
                    }
                }
                Err(e) => {
                    eprintln!("View function error: {e}");
                }
            }
        }
    }

    /// Check if quit was requested and return appropriate task
    fn check_quit_requested(&mut self) -> Option<Task<Message>> {
        use crate::bindings::take_quit_request;

        if take_quit_request() {
            let _ = self.lifecycle.shutdown();
            Some(iced::exit())
        } else {
            None
        }
    }

    /// Check if a theme change was requested and apply it
    fn check_pending_theme(&mut self) {
        use crate::bindings::{take_pending_theme, PendingTheme};

        if let Some(pending) = take_pending_theme() {
            match pending {
                PendingTheme::Preset(preset) => {
                    self.theme = StratumTheme::preset(preset);
                }
                PendingTheme::Custom { name, palette } => {
                    self.theme = StratumTheme::custom(name, palette);
                }
            }
        }
    }

    /// Update the application state based on a message
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Increment => {
                let current = self.get_int_field("count").unwrap_or(0);
                self.state.update_field("count", Value::Int(current + 1));
            }
            Message::Decrement => {
                let current = self.get_int_field("count").unwrap_or(0);
                self.state.update_field("count", Value::Int(current - 1));
            }
            Message::SetIntField { field, value } => {
                self.state.update_field(&field, Value::Int(value));
            }
            Message::SetStringField { field, value } => {
                self.state
                    .update_field(&field, Value::String(Rc::new(value)));
            }
            Message::SetBoolField { field, value } => {
                self.state.update_field(&field, Value::Bool(value));
            }
            Message::SetFloatField { field, value } => {
                self.state.update_field(&field, Value::Float(value));
            }
            Message::TextFieldChanged { callback_id, value } => {
                if let Some(ref executor) = self.executor {
                    let value_arg = Value::String(Rc::new(value));
                    if let Err(e) = executor.execute(callback_id, vec![value_arg]) {
                        eprintln!("TextField on_change callback error: {e}");
                    }
                }
            }
            Message::CheckboxToggled { callback_id, checked } => {
                if let Some(ref executor) = self.executor {
                    let checked_arg = Value::Bool(checked);
                    if let Err(e) = executor.execute(callback_id, vec![checked_arg]) {
                        eprintln!("Checkbox on_toggle callback error: {e}");
                    }
                }
            }
            Message::RadioButtonSelected { callback_id, value } => {
                if let Some(ref executor) = self.executor {
                    let value_arg = Value::String(Rc::new(value));
                    if let Err(e) = executor.execute(callback_id, vec![value_arg]) {
                        eprintln!("RadioButton on_select callback error: {e}");
                    }
                }
            }
            Message::DropdownSelected { callback_id, value } => {
                if let Some(ref executor) = self.executor {
                    let value_arg = Value::String(Rc::new(value));
                    if let Err(e) = executor.execute(callback_id, vec![value_arg]) {
                        eprintln!("Dropdown on_select callback error: {e}");
                    }
                }
            }
            Message::SliderChanged { callback_id, value } => {
                if let Some(ref executor) = self.executor {
                    let value_arg = Value::Float(value);
                    if let Err(e) = executor.execute(callback_id, vec![value_arg]) {
                        eprintln!("Slider on_change callback error: {e}");
                    }
                }
            }
            Message::ToggleSwitched { callback_id, is_on } => {
                if let Some(ref executor) = self.executor {
                    let is_on_arg = Value::Bool(is_on);
                    if let Err(e) = executor.execute(callback_id, vec![is_on_arg]) {
                        eprintln!("Toggle on_toggle callback error: {e}");
                    }
                }
            }
            Message::DataTableSort { callback_id, column } => {
                if let Some(ref executor) = self.executor {
                    let col_arg = Value::String(column.into());
                    if let Err(e) = executor.execute(callback_id, vec![col_arg]) {
                        eprintln!("DataTable on_sort callback error: {e}");
                    }
                }
            }
            Message::DataTablePageChange { callback_id, page } => {
                if let Some(ref executor) = self.executor {
                    let page_arg = Value::Int(page as i64);
                    if let Err(e) = executor.execute(callback_id, vec![page_arg]) {
                        eprintln!("DataTable on_page_change callback error: {e}");
                    }
                }
            }
            Message::DataTableRowSelect { callback_id, rows } => {
                if let Some(ref executor) = self.executor {
                    let row_values: Vec<Value> = rows.into_iter().map(|r| Value::Int(r as i64)).collect();
                    let rows_arg = Value::List(std::rc::Rc::new(std::cell::RefCell::new(row_values)));
                    if let Err(e) = executor.execute(callback_id, vec![rows_arg]) {
                        eprintln!("DataTable on_selection_change callback error: {e}");
                    }
                }
            }
            Message::DataTableRowClick { callback_id, row } => {
                if let Some(ref executor) = self.executor {
                    let row_arg = Value::Int(row as i64);
                    if let Err(e) = executor.execute(callback_id, vec![row_arg]) {
                        eprintln!("DataTable on_row_click callback error: {e}");
                    }
                }
            }
            Message::DataTableCellClick {
                callback_id,
                row,
                column,
            } => {
                if let Some(ref executor) = self.executor {
                    let row_arg = Value::Int(row as i64);
                    let col_arg = Value::String(column.into());
                    if let Err(e) = executor.execute(callback_id, vec![row_arg, col_arg]) {
                        eprintln!("DataTable on_cell_click callback error: {e}");
                    }
                }
            }
            Message::InvokeCallback(id) => {
                if let Some(ref executor) = self.executor {
                    if let Err(e) = executor.execute_with_state(id, &self.state) {
                        eprintln!("Callback execution error: {e}");
                    }
                    // Process any field updates queued by the callback via Gui.update_field()
                    use crate::bindings::take_pending_field_updates;
                    let updates = take_pending_field_updates();
                    let had_updates = !updates.is_empty();
                    for update in updates {
                        self.state.update_field(&update.field, update.value);
                    }
                    // Re-invoke view function if state was updated
                    if had_updates {
                        self.refresh_view();
                    }
                }
            }
            Message::RequestShutdown => {
                let _ = self.lifecycle.shutdown();
                return iced::exit();
            }
            Message::NoOp => {}

            // Window events
            Message::WindowOpened(id) => {
                // Window already registered in run_with, just update focus
                self.window_manager.set_focused(WindowId::from_iced(id), true);
            }
            Message::WindowClosed(id) => {
                self.window_manager.unregister(WindowId::from_iced(id));
                if self.window_manager.is_empty() {
                    let _ = self.lifecycle.shutdown();
                    return iced::exit();
                }
            }
            Message::WindowFocused(id) => {
                // Unfocus all, then focus the active one
                for wid in self.window_manager.ids() {
                    self.window_manager.set_focused(wid, false);
                }
                self.window_manager
                    .set_focused(WindowId::from_iced(id), true);
                let _ = self.lifecycle.on_focus();
            }
            Message::WindowBlurred(id) => {
                self.window_manager
                    .set_focused(WindowId::from_iced(id), false);
                let _ = self.lifecycle.on_blur();
            }
            Message::WindowResized { id, width, height } => {
                self.window_manager
                    .set_size(WindowId::from_iced(id), width, height);
                let _ = self.lifecycle.on_resize(width, height);
            }
            Message::OpenWindow(settings) => {
                let (id, open_task) = window::open(settings.to_iced());
                // Pre-register with a temporary title, will be updated on open
                let window_id = WindowId::from_iced(id);
                self.window_manager.register(window_id, settings);
                return open_task.map(Message::WindowOpened);
            }
            Message::CloseWindow(id) => {
                return window::close(id);
            }

            // Modal events
            Message::ShowModal(config) => {
                self.modals.show(config);
            }
            Message::ModalResult(result) => {
                if let Some(callback_id) = self.modals.close_top() {
                    if let Some(ref executor) = self.executor {
                        // Pass result to callback
                        let result_value = match result {
                            ModalResult::Confirm => Value::String(Rc::new("confirm".to_string())),
                            ModalResult::Cancel => Value::String(Rc::new("cancel".to_string())),
                            ModalResult::Custom(i) => Value::Int(i as i64),
                        };
                        if let Err(e) = executor.execute(callback_id, vec![result_value]) {
                            eprintln!("Modal callback error: {e}");
                        }
                    }
                }
            }
            Message::ModalBackdropClicked => {
                if let Some(modal) = self.modals.top() {
                    if modal.config().dismiss_on_backdrop {
                        return Task::done(Message::ModalResult(ModalResult::Cancel));
                    }
                }
            }

            // OLAP Cube events
            Message::CubeDrillDown {
                callback_id,
                dimension,
                value,
            } => {
                if let Some(ref executor) = self.executor {
                    let dim_arg = Value::String(Rc::new(dimension));
                    let val_arg = match value {
                        Some(v) => Value::String(Rc::new(v)),
                        None => Value::Null,
                    };
                    if let Err(e) = executor.execute(callback_id, vec![dim_arg, val_arg]) {
                        eprintln!("Cube drill-down callback error: {e}");
                    }
                }
            }
            Message::CubeRollUp {
                callback_id,
                dimension,
            } => {
                if let Some(ref executor) = self.executor {
                    let dim_arg = Value::String(Rc::new(dimension));
                    if let Err(e) = executor.execute(callback_id, vec![dim_arg]) {
                        eprintln!("Cube roll-up callback error: {e}");
                    }
                }
            }
            Message::CubeDimensionSelect {
                callback_id,
                dimension,
                value,
            } => {
                if let Some(ref executor) = self.executor {
                    let dim_arg = Value::String(Rc::new(dimension));
                    let val_arg = match value {
                        Some(v) => Value::String(Rc::new(v)),
                        None => Value::Null,
                    };
                    if let Err(e) = executor.execute(callback_id, vec![dim_arg, val_arg]) {
                        eprintln!("Cube dimension select callback error: {e}");
                    }
                }
            }
            Message::CubeHierarchyLevelChange {
                callback_id,
                hierarchy,
                level,
            } => {
                if let Some(ref executor) = self.executor {
                    let hier_arg = Value::String(Rc::new(hierarchy));
                    let level_arg = Value::String(Rc::new(level));
                    if let Err(e) = executor.execute(callback_id, vec![hier_arg, level_arg]) {
                        eprintln!("Cube hierarchy level change callback error: {e}");
                    }
                }
            }
            Message::CubeMeasureSelect {
                callback_id,
                measures,
            } => {
                if let Some(ref executor) = self.executor {
                    let measure_values: Vec<Value> = measures
                        .into_iter()
                        .map(|m| Value::String(Rc::new(m)))
                        .collect();
                    let measures_arg = Value::List(Rc::new(std::cell::RefCell::new(measure_values)));
                    if let Err(e) = executor.execute(callback_id, vec![measures_arg]) {
                        eprintln!("Cube measure select callback error: {e}");
                    }
                }
            }
            Message::CubeChartClick {
                callback_id,
                dimension,
                value,
            } => {
                if let Some(ref executor) = self.executor {
                    let dim_arg = Value::String(Rc::new(dimension));
                    let val_arg = Value::String(Rc::new(value));
                    if let Err(e) = executor.execute(callback_id, vec![dim_arg, val_arg]) {
                        eprintln!("Cube chart click callback error: {e}");
                    }
                }
            }

            // Theme switching
            Message::SetThemePreset(preset) => {
                self.theme = StratumTheme::preset(preset);
            }
            Message::SetCustomTheme { name, palette } => {
                self.theme = StratumTheme::custom(name, palette);
            }

            // Interactive/Mouse events
            Message::MousePress { callback_id, x, y } => {
                if let Some(ref executor) = self.executor {
                    let x_arg = Value::Float(f64::from(x));
                    let y_arg = Value::Float(f64::from(y));
                    if let Err(e) = executor.execute(callback_id, vec![x_arg, y_arg]) {
                        eprintln!("Mouse press callback error: {e}");
                    }
                }
            }
            Message::MouseRelease { callback_id, x, y } => {
                if let Some(ref executor) = self.executor {
                    let x_arg = Value::Float(f64::from(x));
                    let y_arg = Value::Float(f64::from(y));
                    if let Err(e) = executor.execute(callback_id, vec![x_arg, y_arg]) {
                        eprintln!("Mouse release callback error: {e}");
                    }
                }
            }
            Message::MouseDoubleClick { callback_id, x, y } => {
                if let Some(ref executor) = self.executor {
                    let x_arg = Value::Float(f64::from(x));
                    let y_arg = Value::Float(f64::from(y));
                    if let Err(e) = executor.execute(callback_id, vec![x_arg, y_arg]) {
                        eprintln!("Mouse double-click callback error: {e}");
                    }
                }
            }
            Message::MouseRightPress { callback_id, x, y } => {
                if let Some(ref executor) = self.executor {
                    let x_arg = Value::Float(f64::from(x));
                    let y_arg = Value::Float(f64::from(y));
                    if let Err(e) = executor.execute(callback_id, vec![x_arg, y_arg]) {
                        eprintln!("Mouse right-press callback error: {e}");
                    }
                }
            }
            Message::MouseRightRelease { callback_id, x, y } => {
                if let Some(ref executor) = self.executor {
                    let x_arg = Value::Float(f64::from(x));
                    let y_arg = Value::Float(f64::from(y));
                    if let Err(e) = executor.execute(callback_id, vec![x_arg, y_arg]) {
                        eprintln!("Mouse right-release callback error: {e}");
                    }
                }
            }
            Message::MouseMiddlePress { callback_id } => {
                if let Some(ref executor) = self.executor {
                    if let Err(e) = executor.execute_no_args(callback_id) {
                        eprintln!("Mouse middle-press callback error: {e}");
                    }
                }
            }
            Message::MouseMiddleRelease { callback_id } => {
                if let Some(ref executor) = self.executor {
                    if let Err(e) = executor.execute_no_args(callback_id) {
                        eprintln!("Mouse middle-release callback error: {e}");
                    }
                }
            }
            Message::MouseEnter { callback_id } => {
                if let Some(ref executor) = self.executor {
                    if let Err(e) = executor.execute_no_args(callback_id) {
                        eprintln!("Mouse enter callback error: {e}");
                    }
                }
            }
            Message::MouseExit { callback_id } => {
                if let Some(ref executor) = self.executor {
                    if let Err(e) = executor.execute_no_args(callback_id) {
                        eprintln!("Mouse exit callback error: {e}");
                    }
                }
            }
            Message::MouseMove { callback_id, x, y } => {
                if let Some(ref executor) = self.executor {
                    let x_arg = Value::Float(f64::from(x));
                    let y_arg = Value::Float(f64::from(y));
                    if let Err(e) = executor.execute(callback_id, vec![x_arg, y_arg]) {
                        eprintln!("Mouse move callback error: {e}");
                    }
                }
            }
            Message::MouseScroll {
                callback_id,
                delta_x,
                delta_y,
            } => {
                if let Some(ref executor) = self.executor {
                    let dx_arg = Value::Float(f64::from(delta_x));
                    let dy_arg = Value::Float(f64::from(delta_y));
                    if let Err(e) = executor.execute(callback_id, vec![dx_arg, dy_arg]) {
                        eprintln!("Mouse scroll callback error: {e}");
                    }
                }
            }

            // Keyboard events
            Message::KeyPressed {
                callback_id: _,
                key,
                modifiers,
            } => {
                // Use the registered global key press callback instead of the placeholder in the message
                if let Some(callback_id) = self.key_press_callback {
                    if let Some(ref executor) = self.executor {
                        use stratum_core::bytecode::HashableValue;
                        let key_arg = Value::String(Rc::new(key));
                        // Pack modifiers as a struct-like map
                        let mut mods_map = std::collections::HashMap::new();
                        mods_map.insert(HashableValue::String(Rc::new("shift".to_string())), Value::Bool(modifiers.shift));
                        mods_map.insert(HashableValue::String(Rc::new("ctrl".to_string())), Value::Bool(modifiers.ctrl));
                        mods_map.insert(HashableValue::String(Rc::new("alt".to_string())), Value::Bool(modifiers.alt));
                        mods_map.insert(HashableValue::String(Rc::new("logo".to_string())), Value::Bool(modifiers.logo));
                        let mods_arg = Value::Map(Rc::new(std::cell::RefCell::new(mods_map)));
                        if let Err(e) = executor.execute(callback_id, vec![key_arg, mods_arg]) {
                            eprintln!("Key pressed callback error: {e}");
                        }
                    }
                }
            }
            Message::KeyReleased {
                callback_id: _,
                key,
                modifiers,
            } => {
                // Use the registered global key release callback instead of the placeholder in the message
                if let Some(callback_id) = self.key_release_callback {
                    if let Some(ref executor) = self.executor {
                        use stratum_core::bytecode::HashableValue;
                        let key_arg = Value::String(Rc::new(key));
                        let mut mods_map = std::collections::HashMap::new();
                        mods_map.insert(HashableValue::String(Rc::new("shift".to_string())), Value::Bool(modifiers.shift));
                        mods_map.insert(HashableValue::String(Rc::new("ctrl".to_string())), Value::Bool(modifiers.ctrl));
                        mods_map.insert(HashableValue::String(Rc::new("alt".to_string())), Value::Bool(modifiers.alt));
                        mods_map.insert(HashableValue::String(Rc::new("logo".to_string())), Value::Bool(modifiers.logo));
                        let mods_arg = Value::Map(Rc::new(std::cell::RefCell::new(mods_map)));
                        if let Err(e) = executor.execute(callback_id, vec![key_arg, mods_arg]) {
                            eprintln!("Key released callback error: {e}");
                        }
                    }
                }
            }

            // Focus events
            Message::TextFieldFocused { callback_id } => {
                if let Some(ref executor) = self.executor {
                    if let Err(e) = executor.execute_no_args(callback_id) {
                        eprintln!("TextField focused callback error: {e}");
                    }
                }
            }
            Message::TextFieldUnfocused { callback_id } => {
                if let Some(ref executor) = self.executor {
                    if let Err(e) = executor.execute_no_args(callback_id) {
                        eprintln!("TextField unfocused callback error: {e}");
                    }
                }
            }
            Message::FocusWidget { widget_id: _ } => {
                // Focus a widget by ID - requires widget ID support
                // This feature requires iced widget IDs which need additional setup
                // For now, this is a no-op placeholder
            }

            // File drop events
            Message::FileHovered { paths } => {
                // Store in state or call a global callback if registered
                if let Some(ref executor) = self.executor {
                    if let Some(file_hover_cb) = self.file_hover_callback {
                        let paths_values: Vec<Value> = paths
                            .iter()
                            .map(|p| Value::String(Rc::new(p.to_string_lossy().to_string())))
                            .collect();
                        let paths_arg = Value::List(Rc::new(std::cell::RefCell::new(paths_values)));
                        let _ = executor.execute(file_hover_cb, vec![paths_arg]);
                    }
                }
            }
            Message::FileDropped { paths } => {
                if let Some(ref executor) = self.executor {
                    if let Some(file_drop_cb) = self.file_drop_callback {
                        let paths_values: Vec<Value> = paths
                            .iter()
                            .map(|p| Value::String(Rc::new(p.to_string_lossy().to_string())))
                            .collect();
                        let paths_arg = Value::List(Rc::new(std::cell::RefCell::new(paths_values)));
                        let _ = executor.execute(file_drop_cb, vec![paths_arg]);
                    }
                }
            }
            Message::FileHoverLeft => {
                if let Some(ref executor) = self.executor {
                    if let Some(file_hover_left_cb) = self.file_hover_left_callback {
                        let _ = executor.execute_no_args(file_hover_left_cb);
                    }
                }
            }

            // Context menu events
            Message::ShowContextMenu { x, y, items } => {
                self.context_menu = Some(ContextMenuState {
                    x,
                    y,
                    items,
                });
            }
            Message::ContextMenuSelect {
                callback_id,
                item_index,
            } => {
                self.context_menu = None;
                if let Some(ref executor) = self.executor {
                    let index_arg = Value::Int(item_index as i64);
                    if let Err(e) = executor.execute(callback_id, vec![index_arg]) {
                        eprintln!("Context menu select callback error: {e}");
                    }
                }
            }
            Message::HideContextMenu => {
                self.context_menu = None;
            }

            // Internal measure toggle - update internal state without callback
            Message::InternalMeasureToggle {
                measure,
                selected,
                field_path,
            } => {
                // Track the toggle in internal measure selection state
                if selected {
                    if !self.selected_measures.contains(&measure) {
                        self.selected_measures.push(measure.clone());
                    }
                } else {
                    self.selected_measures.retain(|m| m != &measure);
                }

                // If a field path is specified, also update that field in state
                if let Some(field) = field_path {
                    // Store as a list of strings in the state
                    let measure_values: Vec<Value> = self
                        .selected_measures
                        .iter()
                        .map(|m| Value::String(Rc::new(m.clone())))
                        .collect();
                    self.state
                        .update_field(&field, Value::List(Rc::new(RefCell::new(measure_values))));
                }
            }

            // Internal dimension selection - update internal state without callback
            Message::InternalDimensionSelect {
                dimension: _,
                value,
                field_path,
            } => {
                // If a field path is specified, update that field in state
                if let Some(field) = field_path {
                    let val = match value {
                        Some(v) => Value::String(Rc::new(v)),
                        None => Value::Null,
                    };
                    self.state.update_field(&field, val);
                }
                // Note: The actual selection state is maintained in the GuiElement's
                // internal_selection Arc<RwLock<...>> which is updated directly in the closure
            }
        }

        // After any message processing, refresh the view if we have a view_fn
        // This ensures the UI reflects any state changes from callbacks
        self.refresh_view();

        // Check if a theme change was requested by a callback (via Gui.set_theme())
        self.check_pending_theme();

        // Check if quit was requested by a callback (via Gui.quit())
        if let Some(quit_task) = self.check_quit_requested() {
            return quit_task;
        }

        Task::none()
    }

    /// Build the view from the current state
    fn view(&self) -> Element<'_, Message> {
        use crate::modal::modal_overlay;

        // If a root element is provided, render it; otherwise show the demo
        let content: Element<'_, Message> = if let Some(ref root) = self.root_element {
            // Render the user-provided GUI element tree
            root.render()
        } else {
            // Fall back to the counter demo
            self.render_demo_view()
        };

        // Wrap content in scrollable for long content
        let scrollable_content = scrollable(content)
            .width(Fill)
            .height(Fill);

        let base = container(scrollable_content)
            .padding(self.padding)
            .center_x(Fill)
            .center_y(Fill);

        // Wrap with modal overlay if there's an active modal
        let backdrop_msg = self
            .modals
            .top()
            .filter(|m| m.config().dismiss_on_backdrop)
            .map(|_| Message::ModalBackdropClicked);

        let with_modal = modal_overlay(base, self.modals.top(), Message::ModalResult, backdrop_msg);

        // Wrap with context menu overlay if there's an active context menu
        if let Some(ref menu_state) = self.context_menu {
            self.render_context_menu_overlay(with_modal, menu_state)
        } else {
            with_modal
        }
    }

    /// Render the default counter demo view
    fn render_demo_view(&self) -> Element<'_, Message> {
        let count = self.get_int_field("count").unwrap_or(0);
        let window_count = self.window_manager.len();

        column![
            text("Stratum GUI - Counter Demo").size(32),
            text(format!("Count: {count}")).size(48),
            row![
                button(text("-").size(24).align_x(Center))
                    .padding(12)
                    .on_press(Message::Decrement),
                button(text("+").size(24).align_x(Center))
                    .padding(12)
                    .on_press(Message::Increment),
            ]
            .spacing(self.spacing),
            text(format!("Windows: {window_count}")).size(14),
            row![
                button(text("New Window"))
                    .padding(8)
                    .on_press(Message::OpenWindow(
                        WindowSettings::new(format!(
                            "Window {}",
                            self.window_manager.next_window_number()
                        ))
                        .with_size(400, 300)
                    )),
                button(text("Show Modal"))
                    .padding(8)
                    .on_press(Message::ShowModal(ModalConfig::confirm(
                        "Test Modal",
                        "This is a modal dialog. Click OK or Cancel."
                    ))),
            ]
            .spacing(8),
        ]
        .spacing(self.spacing)
        .align_x(Center)
        .into()
    }

    /// Render a context menu overlay
    fn render_context_menu_overlay<'a>(
        &self,
        base: Element<'a, Message>,
        menu: &'a ContextMenuState,
    ) -> Element<'a, Message> {
        use iced::widget::{stack, container, column, button, text, mouse_area};

        // Build menu items
        let menu_items: Vec<Element<'_, Message>> = menu.items
            .iter()
            .enumerate()
            .filter_map(|(idx, item)| {
                if item.separator {
                    // Render separator line
                    Some(
                        container(iced::widget::Space::new().width(Fill).height(1))
                            .style(|_theme: &Theme| container::Style {
                                background: Some(iced::Background::Color(Color::from_rgb(0.5, 0.5, 0.5))),
                                ..Default::default()
                            })
                            .into()
                    )
                } else if let Some(callback_id) = item.on_select {
                    // Render clickable menu item
                    let label = text(&item.label);
                    let btn = if item.disabled {
                        button(label).padding([4, 16])
                    } else {
                        button(label)
                            .padding([4, 16])
                            .on_press(Message::ContextMenuSelect {
                                callback_id,
                                item_index: idx,
                            })
                    };
                    Some(btn.width(Fill).into())
                } else {
                    // Item without callback - just display
                    Some(
                        container(text(&item.label))
                            .padding([4, 16])
                            .width(Fill)
                            .into()
                    )
                }
            })
            .collect();

        // Create menu container
        let menu_column = column(menu_items).spacing(2);
        let menu_container = container(menu_column)
            .padding(4)
            .style(|theme: &Theme| {
                let palette = theme.palette();
                container::Style {
                    background: Some(iced::Background::Color(palette.background)),
                    border: iced::Border {
                        color: palette.text,
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
            });

        // Position the menu at the click location
        let positioned_menu = container(menu_container)
            .padding(iced::Padding {
                top: menu.y,
                left: menu.x,
                bottom: 0.0,
                right: 0.0,
            });

        // Create backdrop that hides menu when clicked
        let backdrop = mouse_area(
            container(iced::widget::Space::new().width(Fill).height(Fill))
        )
        .on_press(Message::HideContextMenu);

        // Stack: base, backdrop, menu
        stack![base, backdrop, positioned_menu].into()
    }

    /// Subscribe to window events
    fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![
            window::close_events().map(Message::WindowClosed),
            window::resize_events().map(|(id, size)| Message::WindowResized {
                id,
                width: size.width as u32,
                height: size.height as u32,
            }),
        ];

        // Add keyboard and file drop event subscriptions
        // Note: We use iced::event::listen_with with a pure function to avoid closure capture issues
        subscriptions.push(
            iced::event::listen_with(|event, _status, _id| {
                match event {
                    // Keyboard events
                    iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, modifiers, .. }) => {
                        let key_str = format!("{key:?}");
                        Some(Message::KeyPressed {
                            callback_id: CallbackId::new(0), // Placeholder - handled in update
                            key: key_str,
                            modifiers: KeyModifiers::from_iced(modifiers),
                        })
                    }
                    iced::Event::Keyboard(iced::keyboard::Event::KeyReleased { key, modifiers, .. }) => {
                        let key_str = format!("{key:?}");
                        Some(Message::KeyReleased {
                            callback_id: CallbackId::new(0), // Placeholder - handled in update
                            key: key_str,
                            modifiers: KeyModifiers::from_iced(modifiers),
                        })
                    }
                    // File drop events
                    iced::Event::Window(iced::window::Event::FileHovered(path)) => {
                        Some(Message::FileHovered { paths: vec![path] })
                    }
                    iced::Event::Window(iced::window::Event::FileDropped(path)) => {
                        Some(Message::FileDropped { paths: vec![path] })
                    }
                    iced::Event::Window(iced::window::Event::FilesHoveredLeft) => {
                        Some(Message::FileHoverLeft)
                    }
                    _ => None
                }
            })
        );

        Subscription::batch(subscriptions)
    }

    /// Helper to get an integer field from state
    fn get_int_field(&self, field: &str) -> Option<i64> {
        self.state.get_field(field).and_then(|v| {
            if let Value::Int(i) = v {
                Some(i)
            } else {
                None
            }
        })
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use stratum_core::bytecode::StructInstance;
    use crate::element::GuiElementKind;

    fn create_counter_state(initial: i64) -> Value {
        let mut fields = HashMap::new();
        fields.insert("count".to_string(), Value::Int(initial));
        let mut instance = StructInstance::new("CounterState".to_string());
        instance.fields = fields;
        Value::Struct(Rc::new(RefCell::new(instance)))
    }

    fn create_test_app(initial: i64) -> App {
        App {
            state: ReactiveState::new(create_counter_state(initial)),
            spacing: 16.0,
            padding: 20.0,
            registry: Rc::new(RefCell::new(CallbackRegistry::new())),
            executor: None,
            lifecycle: LifecycleManager::new(),
            window_manager: WindowManager::new(),
            modals: ModalManager::new(),
            theme: StratumTheme::default(),
            file_hover_callback: None,
            file_drop_callback: None,
            file_hover_left_callback: None,
            key_press_callback: None,
            key_release_callback: None,
            context_menu: None,
            root_element: None,
            view_fn: None,
            selected_measures: Vec::new(),
        }
    }

    #[test]
    fn test_runtime_creation() {
        let state = create_counter_state(0);
        let runtime = GuiRuntime::new(state).with_window("Test", (400, 300));

        assert_eq!(runtime.config.window.title, "Test");
        assert_eq!(runtime.config.window.size, (400, 300));
        assert_eq!(runtime.get_count(), 0);
    }

    #[test]
    fn test_runtime_with_theme() {
        let state = create_counter_state(0);
        let runtime = GuiRuntime::new(state)
            .with_theme(AppTheme::Light)
            .with_spacing(8.0)
            .with_padding(16.0);

        assert_eq!(runtime.config.theme.name(), "light");
        assert_eq!(runtime.config.layout.spacing, 8.0);
        assert_eq!(runtime.config.layout.padding, 16.0);
    }

    #[test]
    fn test_runtime_with_stratum_theme() {
        let state = create_counter_state(0);
        let runtime = GuiRuntime::new(state)
            .with_theme_preset(ThemePreset::Dracula);

        assert_eq!(runtime.config.theme.name(), "dracula");
    }

    #[test]
    fn test_theme_switching() {
        let mut app = create_test_app(0);
        assert_eq!(app.theme.name(), "dark"); // default

        let _ = app.update(Message::SetThemePreset(ThemePreset::Light));
        assert_eq!(app.theme.name(), "light");

        let _ = app.update(Message::SetThemePreset(ThemePreset::Nord));
        assert_eq!(app.theme.name(), "nord");

        let _ = app.update(Message::SetCustomTheme {
            name: "my_theme".to_string(),
            palette: StratumPalette::DARK,
        });
        assert_eq!(app.theme.name(), "my_theme");
    }

    #[test]
    fn test_runtime_window_settings() {
        let state = create_counter_state(0);
        let runtime = GuiRuntime::new(state)
            .with_min_size(320, 240)
            .with_max_size(1920, 1080)
            .with_decorations(false)
            .with_resizable(false);

        assert_eq!(runtime.config.window.min_size, Some((320, 240)));
        assert_eq!(runtime.config.window.max_size, Some((1920, 1080)));
        assert!(!runtime.config.window.decorations);
        assert!(!runtime.config.window.resizable);
    }

    #[test]
    fn test_state_update() {
        let state = create_counter_state(5);
        let reactive = ReactiveState::new(state);

        assert_eq!(reactive.get_field("count"), Some(Value::Int(5)));

        reactive.update_field("count", Value::Int(10));
        assert_eq!(reactive.get_field("count"), Some(Value::Int(10)));
    }

    #[test]
    fn test_message_handling() {
        let mut app = create_test_app(0);

        // Test increment
        let _ = app.update(Message::Increment);
        assert_eq!(app.state.get_field("count"), Some(Value::Int(1)));

        let _ = app.update(Message::Increment);
        assert_eq!(app.state.get_field("count"), Some(Value::Int(2)));

        // Test decrement
        let _ = app.update(Message::Decrement);
        assert_eq!(app.state.get_field("count"), Some(Value::Int(1)));

        // Test set field
        let _ = app.update(Message::SetIntField {
            field: "count".to_string(),
            value: 100,
        });
        assert_eq!(app.state.get_field("count"), Some(Value::Int(100)));
    }

    #[test]
    fn test_window_manager_in_app() {
        let mut app = create_test_app(0);
        let main_id = app.window_manager.register_main(WindowSettings::new("Main"));

        assert_eq!(app.window_manager.len(), 1);
        assert_eq!(app.window_manager.title(main_id), "Main");
    }

    #[test]
    fn test_modal_manager_in_app() {
        let mut app = create_test_app(0);
        assert!(!app.modals.has_modal());

        let _ = app.update(Message::ShowModal(ModalConfig::alert("Test", "Message")));
        assert!(app.modals.has_modal());
        assert_eq!(app.modals.count(), 1);

        let _ = app.update(Message::ModalResult(ModalResult::Confirm));
        assert!(!app.modals.has_modal());
    }

    #[test]
    fn test_backend_default() {
        assert_eq!(Backend::default(), Backend::Iced);
    }

    #[test]
    fn test_app_theme_conversion() {
        let _ = AppTheme::Light.to_iced_theme();
        let _ = AppTheme::Dark.to_iced_theme();
        let _ = AppTheme::System.to_iced_theme();
    }

    #[test]
    fn test_callback_registration() {
        use stratum_core::bytecode::NativeFunction;

        let state = create_counter_state(0);
        let runtime = GuiRuntime::new(state);

        let handler = Value::NativeFunction(NativeFunction::new("test", 0, |_| Ok(Value::Null)));
        let id = runtime.register_callback(handler).unwrap();

        assert!(runtime.registry.borrow().contains(id));
    }

    #[test]
    fn test_runtime_with_vm() {
        let state = create_counter_state(0);
        let vm = VM::new();
        let runtime = GuiRuntime::new(state).with_vm(vm);

        assert!(runtime.vm.is_some());
    }

    // ========================================================================
    // Event System Tests
    // ========================================================================

    #[test]
    fn test_key_modifiers_default() {
        let modifiers = KeyModifiers::default();
        assert!(!modifiers.shift);
        assert!(!modifiers.ctrl);
        assert!(!modifiers.alt);
        assert!(!modifiers.logo);
    }

    #[test]
    fn test_key_modifiers_creation() {
        let modifiers = KeyModifiers {
            shift: true,
            ctrl: true,
            alt: false,
            logo: false,
        };
        assert!(modifiers.shift);
        assert!(modifiers.ctrl);
        assert!(!modifiers.alt);
        assert!(!modifiers.logo);
    }

    #[test]
    fn test_context_menu_item() {
        use crate::callback::CallbackId;

        let item = ContextMenuItem {
            label: "Copy".to_string(),
            icon: Some("copy-icon".to_string()),
            on_select: Some(CallbackId::new(42)),
            disabled: false,
            separator: false,
        };

        assert_eq!(item.label, "Copy");
        assert_eq!(item.icon, Some("copy-icon".to_string()));
        assert!(item.on_select.is_some());
        assert!(!item.disabled);
        assert!(!item.separator);
    }

    #[test]
    fn test_context_menu_item_disabled() {
        use crate::callback::CallbackId;

        let item = ContextMenuItem {
            label: "Delete".to_string(),
            icon: None,
            on_select: Some(CallbackId::new(1)),
            disabled: true,
            separator: false,
        };

        assert_eq!(item.label, "Delete");
        assert!(item.icon.is_none());
        assert!(item.disabled);
    }

    #[test]
    fn test_context_menu_separator() {
        let item = ContextMenuItem::separator();

        assert!(item.separator);
        assert!(item.label.is_empty());
    }

    #[test]
    fn test_context_menu_state() {
        use crate::callback::CallbackId;

        let items = vec![
            ContextMenuItem::new("Cut").on_select(CallbackId::new(1)),
            ContextMenuItem::new("Copy").on_select(CallbackId::new(2)),
            ContextMenuItem::separator(),
            ContextMenuItem::new("Paste").on_select(CallbackId::new(3)),
        ];

        let menu = ContextMenuState {
            x: 100.0,
            y: 200.0,
            items,
        };

        assert_eq!(menu.x, 100.0);
        assert_eq!(menu.y, 200.0);
        assert_eq!(menu.items.len(), 4);
        assert_eq!(menu.items[0].label, "Cut");
        assert_eq!(menu.items[1].label, "Copy");
        assert!(menu.items[2].separator);
        assert_eq!(menu.items[3].label, "Paste");
    }

    // ========================================================================
    // Root Element Tests
    // ========================================================================

    #[test]
    fn test_runtime_with_root_element() {
        use crate::element::GuiElement;

        let state = create_counter_state(0);
        let root = GuiElement::text("Hello World").build();

        let runtime = GuiRuntime::new(state).with_root(root);

        assert!(runtime.root_element.is_some());
    }

    #[test]
    fn test_runtime_without_root_element() {
        let state = create_counter_state(0);
        let runtime = GuiRuntime::new(state);

        assert!(runtime.root_element.is_none());
    }

    #[test]
    fn test_element_vstack_rendering() {
        use crate::element::GuiElement;

        // Build a VStack with children - this tests element creation
        let vstack = GuiElement::vstack()
            .spacing(16.0)
            .child(GuiElement::text("Line 1").build())
            .child(GuiElement::text("Line 2").build())
            .build();

        // Verify structure
        assert_eq!(vstack.children.len(), 2);
    }

    #[test]
    fn test_element_hstack_rendering() {
        use crate::element::GuiElement;

        // Build an HStack with children
        let hstack = GuiElement::hstack()
            .spacing(8.0)
            .child(GuiElement::text("Item A").build())
            .child(GuiElement::text("Item B").build())
            .child(GuiElement::text("Item C").build())
            .build();

        // Verify structure
        assert_eq!(hstack.children.len(), 3);
    }

    #[test]
    fn test_element_grid_rendering() {
        use crate::element::GuiElement;

        // Build a Grid with children
        let grid = GuiElement::grid(3)
            .spacing(4.0)
            .child(GuiElement::text("Cell 1").build())
            .child(GuiElement::text("Cell 2").build())
            .child(GuiElement::text("Cell 3").build())
            .build();

        // Verify structure
        assert_eq!(grid.children.len(), 3);
    }

    #[test]
    fn test_element_nested_layouts() {
        use crate::element::GuiElement;

        // Build nested layouts
        let nested = GuiElement::vstack()
            .child(
                GuiElement::hstack()
                    .child(GuiElement::text("A").build())
                    .child(GuiElement::text("B").build())
                    .build()
            )
            .child(
                GuiElement::hstack()
                    .child(GuiElement::text("C").build())
                    .child(GuiElement::text("D").build())
                    .build()
            )
            .build();

        // Verify top-level has 2 children (the HStacks)
        assert_eq!(nested.children.len(), 2);
        // Verify each HStack has 2 children
        assert_eq!(nested.children[0].children.len(), 2);
        assert_eq!(nested.children[1].children.len(), 2);
    }

    #[test]
    fn test_element_widget_creation() {
        use crate::element::GuiElement;

        // Test button creation
        let button = GuiElement::button("Click Me").build();
        // Button has no children (it's a leaf widget)
        assert!(button.children.is_empty());

        // Test checkbox creation
        let checkbox = GuiElement::checkbox("Agree").build();
        assert!(checkbox.children.is_empty());

        // Test text field creation
        let textfield = GuiElement::text_field()
            .placeholder("Enter text")
            .build();
        assert!(textfield.children.is_empty());
    }

    // ==========================================================================
    // End-to-End Integration Tests for Verification Checklist
    // ==========================================================================

    /// Helper to create state with multiple field types for binding tests
    fn create_binding_test_state() -> Value {
        let mut fields = HashMap::new();
        fields.insert("text_value".to_string(), Value::String(Rc::new("initial".to_string())));
        fields.insert("checked".to_string(), Value::Bool(false));
        fields.insert("slider_value".to_string(), Value::Float(50.0));
        fields.insert("toggle_on".to_string(), Value::Bool(true));
        fields.insert("selected_option".to_string(), Value::String(Rc::new("A".to_string())));
        fields.insert("count".to_string(), Value::Int(0));
        fields.insert("show_details".to_string(), Value::Bool(false));
        fields.insert(
            "items".to_string(),
            Value::List(Rc::new(RefCell::new(vec![
                Value::String(Rc::new("Item 1".to_string())),
                Value::String(Rc::new("Item 2".to_string())),
                Value::String(Rc::new("Item 3".to_string())),
            ]))),
        );
        let mut instance = StructInstance::new("BindingTestState".to_string());
        instance.fields = fields;
        Value::Struct(Rc::new(RefCell::new(instance)))
    }

    fn create_binding_test_app() -> App {
        App {
            state: ReactiveState::new(create_binding_test_state()),
            spacing: 16.0,
            padding: 20.0,
            registry: Rc::new(RefCell::new(CallbackRegistry::new())),
            executor: None,
            lifecycle: LifecycleManager::new(),
            window_manager: WindowManager::new(),
            modals: ModalManager::new(),
            theme: StratumTheme::default(),
            file_hover_callback: None,
            file_drop_callback: None,
            file_hover_left_callback: None,
            key_press_callback: None,
            key_release_callback: None,
            context_menu: None,
            root_element: None,
            view_fn: None,
            selected_measures: Vec::new(),
        }
    }

    // -------------------------------------------------------------------------
    // Two-Way Binding End-to-End Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_e2e_string_field_binding() {
        // Test: TextField with field_path updates state via SetStringField message
        use crate::element::GuiElement;

        let mut app = create_binding_test_app();

        // Verify initial state
        assert_eq!(
            app.state.get_field("text_value"),
            Some(Value::String(Rc::new("initial".to_string())))
        );

        // Create a text field bound to the text_value field
        let textfield = GuiElement::text_field()
            .bind_field("text_value")
            .build();

        // Verify the widget has the correct field_path
        if let GuiElementKind::TextField(config) = &textfield.kind {
            assert_eq!(config.field_path, Some("text_value".to_string()));
        } else {
            panic!("Expected TextField element");
        }

        // Simulate user typing - this is what happens when the widget fires
        let _ = app.update(Message::SetStringField {
            field: "text_value".to_string(),
            value: "user typed this".to_string(),
        });

        // Verify state was updated
        assert_eq!(
            app.state.get_field("text_value"),
            Some(Value::String(Rc::new("user typed this".to_string())))
        );

        // Verify generation incremented (re-render would be triggered)
        assert!(app.state.generation() > 0);
    }

    #[test]
    fn test_e2e_bool_field_binding_checkbox() {
        // Test: Checkbox with field_path updates state via SetBoolField message
        use crate::element::GuiElement;

        let mut app = create_binding_test_app();

        // Verify initial state
        assert_eq!(app.state.get_field("checked"), Some(Value::Bool(false)));

        // Create a checkbox bound to the checked field
        let checkbox = GuiElement::checkbox("Accept terms")
            .bind_field("checked")
            .build();

        // Verify the widget has the correct field_path
        if let GuiElementKind::Checkbox(config) = &checkbox.kind {
            assert_eq!(config.field_path, Some("checked".to_string()));
        } else {
            panic!("Expected Checkbox element");
        }

        // Simulate user clicking checkbox
        let _ = app.update(Message::SetBoolField {
            field: "checked".to_string(),
            value: true,
        });

        // Verify state was updated
        assert_eq!(app.state.get_field("checked"), Some(Value::Bool(true)));
    }

    #[test]
    fn test_e2e_bool_field_binding_toggle() {
        // Test: Toggle with field_path updates state via SetBoolField message
        use crate::element::GuiElement;

        let mut app = create_binding_test_app();

        // Verify initial state
        assert_eq!(app.state.get_field("toggle_on"), Some(Value::Bool(true)));

        // Create a toggle bound to the toggle_on field
        let toggle = GuiElement::toggle("Enable feature")
            .bind_field("toggle_on")
            .build();

        // Verify the widget has the correct field_path
        if let GuiElementKind::Toggle(config) = &toggle.kind {
            assert_eq!(config.field_path, Some("toggle_on".to_string()));
        } else {
            panic!("Expected Toggle element");
        }

        // Simulate user toggling off
        let _ = app.update(Message::SetBoolField {
            field: "toggle_on".to_string(),
            value: false,
        });

        // Verify state was updated
        assert_eq!(app.state.get_field("toggle_on"), Some(Value::Bool(false)));
    }

    #[test]
    fn test_e2e_float_field_binding_slider() {
        // Test: Slider with field_path updates state via SetFloatField message
        use crate::element::GuiElement;

        let mut app = create_binding_test_app();

        // Verify initial state
        assert_eq!(
            app.state.get_field("slider_value"),
            Some(Value::Float(50.0))
        );

        // Create a slider bound to the slider_value field
        let slider = GuiElement::slider(0.0, 100.0)
            .bind_field("slider_value")
            .build();

        // Verify the widget has the correct field_path
        if let GuiElementKind::Slider(config) = &slider.kind {
            assert_eq!(config.field_path, Some("slider_value".to_string()));
        } else {
            panic!("Expected Slider element");
        }

        // Simulate user dragging slider
        let _ = app.update(Message::SetFloatField {
            field: "slider_value".to_string(),
            value: 75.5,
        });

        // Verify state was updated
        assert_eq!(
            app.state.get_field("slider_value"),
            Some(Value::Float(75.5))
        );
    }

    #[test]
    fn test_e2e_string_field_binding_dropdown() {
        // Test: Dropdown with field_path updates state via SetStringField message
        use crate::element::GuiElement;

        let mut app = create_binding_test_app();

        // Verify initial state
        assert_eq!(
            app.state.get_field("selected_option"),
            Some(Value::String(Rc::new("A".to_string())))
        );

        // Create a dropdown bound to the selected_option field
        let dropdown = GuiElement::dropdown(vec![
            "A".to_string(),
            "B".to_string(),
            "C".to_string(),
        ])
        .bind_field("selected_option")
        .build();

        // Verify the widget has the correct field_path
        if let GuiElementKind::Dropdown(config) = &dropdown.kind {
            assert_eq!(config.field_path, Some("selected_option".to_string()));
        } else {
            panic!("Expected Dropdown element");
        }

        // Simulate user selecting an option
        let _ = app.update(Message::SetStringField {
            field: "selected_option".to_string(),
            value: "B".to_string(),
        });

        // Verify state was updated
        assert_eq!(
            app.state.get_field("selected_option"),
            Some(Value::String(Rc::new("B".to_string())))
        );
    }

    #[test]
    fn test_e2e_string_field_binding_radio() {
        // Test: RadioButton with field_path updates state via SetStringField message
        use crate::element::GuiElement;

        let mut app = create_binding_test_app();

        // Verify initial state
        assert_eq!(
            app.state.get_field("selected_option"),
            Some(Value::String(Rc::new("A".to_string())))
        );

        // Create radio buttons bound to the selected_option field
        let radio_b = GuiElement::radio_button("Option B", "B")
            .bind_field("selected_option")
            .build();

        // Verify the widget has the correct field_path
        if let GuiElementKind::RadioButton(config) = &radio_b.kind {
            assert_eq!(config.field_path, Some("selected_option".to_string()));
            assert_eq!(config.value, "B");
        } else {
            panic!("Expected RadioButton element");
        }

        // Simulate user clicking radio button
        let _ = app.update(Message::SetStringField {
            field: "selected_option".to_string(),
            value: "B".to_string(),
        });

        // Verify state was updated
        assert_eq!(
            app.state.get_field("selected_option"),
            Some(Value::String(Rc::new("B".to_string())))
        );
    }

    #[test]
    fn test_e2e_int_field_update() {
        // Test: SetIntField message updates integer state correctly
        let mut app = create_binding_test_app();

        // Verify initial state
        assert_eq!(app.state.get_field("count"), Some(Value::Int(0)));

        // Simulate setting an integer field
        let _ = app.update(Message::SetIntField {
            field: "count".to_string(),
            value: 42,
        });

        // Verify state was updated
        assert_eq!(app.state.get_field("count"), Some(Value::Int(42)));
    }

    #[test]
    fn test_e2e_state_change_triggers_rerender() {
        // Test: State changes increment generation, triggering re-render
        let mut app = create_binding_test_app();
        let initial_gen = app.state.generation();

        // Multiple state updates should each increment generation
        let _ = app.update(Message::SetStringField {
            field: "text_value".to_string(),
            value: "change 1".to_string(),
        });
        assert!(app.state.generation() > initial_gen);
        let gen_after_1 = app.state.generation();

        let _ = app.update(Message::SetBoolField {
            field: "checked".to_string(),
            value: true,
        });
        assert!(app.state.generation() > gen_after_1);
    }

    // -------------------------------------------------------------------------
    // Conditional Rendering End-to-End Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_e2e_conditional_rendering_true_branch() {
        // Test: Conditional element renders true branch when condition is true
        use crate::element::GuiElement;

        let mut app = create_binding_test_app();

        // Set condition to true
        let _ = app.update(Message::SetBoolField {
            field: "show_details".to_string(),
            value: true,
        });

        // Create conditional element
        let conditional = GuiElement::conditional("show_details")
            .true_element(GuiElement::text("Details visible").build())
            .false_element(GuiElement::text("Details hidden").build())
            .build();

        // Verify correct structure
        if let GuiElementKind::Conditional(config) = &conditional.kind {
            assert_eq!(config.condition_field, "show_details");
            assert!(config.true_element.is_some());
            assert!(config.false_element.is_some());
        } else {
            panic!("Expected Conditional element");
        }

        // Verify the condition is true in state
        assert_eq!(
            app.state.get_field("show_details"),
            Some(Value::Bool(true))
        );
    }

    #[test]
    fn test_e2e_conditional_rendering_false_branch() {
        // Test: Conditional element renders false branch when condition is false
        use crate::element::GuiElement;

        let app = create_binding_test_app();

        // Condition is false by default
        assert_eq!(
            app.state.get_field("show_details"),
            Some(Value::Bool(false))
        );

        // Create conditional element
        let conditional = GuiElement::conditional("show_details")
            .true_element(GuiElement::text("Details visible").build())
            .false_element(GuiElement::text("Details hidden").build())
            .build();

        // Verify structure is correct - false branch should be rendered
        if let GuiElementKind::Conditional(config) = &conditional.kind {
            assert_eq!(config.condition_field, "show_details");
            // State resolution happens in render_with_state
            // Here we verify the structure is set up correctly
        } else {
            panic!("Expected Conditional element");
        }
    }

    #[test]
    fn test_e2e_conditional_state_toggle() {
        // Test: Toggling condition field changes which branch would render
        use crate::element::GuiElement;

        let mut app = create_binding_test_app();

        // Create conditional element
        let _conditional = GuiElement::conditional("show_details")
            .true_element(GuiElement::text("TRUE").build())
            .false_element(GuiElement::text("FALSE").build())
            .build();

        // Initially false
        assert_eq!(
            app.state.get_field("show_details"),
            Some(Value::Bool(false))
        );

        // Toggle to true
        let _ = app.update(Message::SetBoolField {
            field: "show_details".to_string(),
            value: true,
        });
        assert_eq!(
            app.state.get_field("show_details"),
            Some(Value::Bool(true))
        );

        // Toggle back to false
        let _ = app.update(Message::SetBoolField {
            field: "show_details".to_string(),
            value: false,
        });
        assert_eq!(
            app.state.get_field("show_details"),
            Some(Value::Bool(false))
        );
    }

    // -------------------------------------------------------------------------
    // List Rendering End-to-End Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_e2e_list_state_access() {
        // Test: ForEach element can access list from state
        use crate::element::GuiElement;

        let app = create_binding_test_app();

        // Verify list exists in state
        let items = app.state.get_field("items");
        assert!(items.is_some());
        if let Some(Value::List(list)) = items {
            let borrowed = list.borrow();
            assert_eq!(borrowed.len(), 3);
            assert_eq!(
                borrowed[0],
                Value::String(Rc::new("Item 1".to_string()))
            );
        } else {
            panic!("Expected list field");
        }

        // Create ForEach element
        let for_each = GuiElement::for_each("items").build();

        // Verify structure
        if let GuiElementKind::ForEach(config) = &for_each.kind {
            assert_eq!(config.list_field, "items");
        } else {
            panic!("Expected ForEach element");
        }
    }

    #[test]
    fn test_e2e_list_with_template() {
        // Test: ForEach element with template callback ID
        use crate::element::GuiElement;
        use crate::callback::CallbackId;

        let template_id = CallbackId::new(100);
        let for_each = GuiElement::for_each_with_template("items", template_id).build();

        // Verify structure
        if let GuiElementKind::ForEach(config) = &for_each.kind {
            assert_eq!(config.list_field, "items");
            assert_eq!(config.template_id, Some(template_id));
        } else {
            panic!("Expected ForEach element");
        }
    }

    #[test]
    fn test_e2e_list_iteration() {
        // Test: ReactiveState.get_list returns iterable list
        let app = create_binding_test_app();

        // Use get_list method for iteration
        if let Some(items) = app.state.get_list("items") {
            assert_eq!(items.len(), 3);
            for (i, item) in items.iter().enumerate() {
                if let Value::String(s) = item {
                    assert_eq!(s.as_str(), format!("Item {}", i + 1));
                } else {
                    panic!("Expected string items");
                }
            }
        } else {
            panic!("Expected list field");
        }
    }

    // -------------------------------------------------------------------------
    // DataTable End-to-End Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_e2e_data_table_creation() {
        // Test: DataTable element can be created and configured
        use crate::element::GuiElement;

        let element = GuiElement::data_table()
            .page_size(Some(20))
            .sortable(true)
            .selectable(true)
            .build();

        if let GuiElementKind::DataTable(config) = &element.kind {
            assert_eq!(config.page_size, Some(20));
            assert!(config.sortable);
            assert!(config.selectable);
        } else {
            panic!("Expected DataTable element");
        }
    }

    #[test]
    fn test_e2e_data_table_with_columns() {
        // Test: DataTable with specific columns
        use crate::element::GuiElement;

        let element = GuiElement::data_table()
            .table_columns(vec!["name".to_string(), "age".to_string()])
            .build();

        if let GuiElementKind::DataTable(config) = &element.kind {
            let cols = config.columns.as_ref().expect("columns should be set");
            assert_eq!(cols.len(), 2);
            assert_eq!(cols[0], "name");
            assert_eq!(cols[1], "age");
        } else {
            panic!("Expected DataTable element");
        }
    }

    #[test]
    fn test_e2e_data_table_pagination() {
        // Test: DataTable pagination state
        use crate::element::GuiElement;

        let element = GuiElement::data_table()
            .page_size(Some(10))
            .current_page(3)
            .build();

        if let GuiElementKind::DataTable(config) = &element.kind {
            assert_eq!(config.page_size, Some(10));
            assert_eq!(config.current_page, 3);
        } else {
            panic!("Expected DataTable element");
        }
    }

    // -------------------------------------------------------------------------
    // Chart Rendering End-to-End Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_e2e_bar_chart_with_data() {
        // Test: BarChart element can be created with data
        use crate::element::GuiElement;
        use crate::charts::DataPoint;

        let data = vec![
            DataPoint::new("Q1", 100.0),
            DataPoint::new("Q2", 150.0),
            DataPoint::new("Q3", 120.0),
            DataPoint::new("Q4", 180.0),
        ];

        let chart = GuiElement::bar_chart_with_data(data.clone())
            .chart_title("Quarterly Revenue")
            .chart_size(600.0, 400.0)
            .show_grid(true)
            .show_values(true)
            .build();

        if let GuiElementKind::BarChart(config) = &chart.kind {
            assert_eq!(config.title, Some("Quarterly Revenue".to_string()));
            assert_eq!(config.data.len(), 4);
            assert!((config.width - 600.0).abs() < f32::EPSILON);
            assert!((config.height - 400.0).abs() < f32::EPSILON);
            assert!(config.show_grid);
            assert!(config.show_values);
        } else {
            panic!("Expected BarChart element");
        }
    }

    #[test]
    fn test_e2e_line_chart_with_series() {
        // Test: LineChart element can be created with multiple series
        use crate::element::GuiElement;
        use crate::charts::DataSeries;

        let labels = vec!["Jan".to_string(), "Feb".to_string(), "Mar".to_string()];
        let series = vec![
            DataSeries::new("Revenue", vec![100.0, 150.0, 120.0]),
            DataSeries::new("Expenses", vec![80.0, 90.0, 85.0]),
        ];

        let chart = GuiElement::line_chart_with_data(labels.clone(), series)
            .chart_title("Financial Overview")
            .show_points(true)
            .show_legend(true)
            .build();

        if let GuiElementKind::LineChart(config) = &chart.kind {
            assert_eq!(config.title, Some("Financial Overview".to_string()));
            assert_eq!(config.labels.len(), 3);
            assert_eq!(config.series.len(), 2);
            assert!(config.show_points);
            assert!(config.show_legend);
        } else {
            panic!("Expected LineChart element");
        }
    }

    #[test]
    fn test_e2e_pie_chart_with_data() {
        // Test: PieChart element can be created with data
        use crate::element::GuiElement;
        use crate::charts::DataPoint;

        let data = vec![
            DataPoint::new("Product A", 45.0),
            DataPoint::new("Product B", 30.0),
            DataPoint::new("Product C", 25.0),
        ];

        let chart = GuiElement::pie_chart_with_data(data.clone())
            .chart_title("Market Share")
            .show_percentages(true)
            .show_legend(true)
            .build();

        if let GuiElementKind::PieChart(config) = &chart.kind {
            assert_eq!(config.title, Some("Market Share".to_string()));
            assert_eq!(config.data.len(), 3);
            assert!(config.show_percentages);
            assert!(config.show_legend);
        } else {
            panic!("Expected PieChart element");
        }
    }

    #[test]
    fn test_e2e_donut_chart() {
        // Test: Donut chart (pie chart with inner radius)
        use crate::element::GuiElement;
        use crate::charts::DataPoint;

        let data = vec![
            DataPoint::new("Yes", 60.0),
            DataPoint::new("No", 40.0),
        ];

        let chart = GuiElement::pie_chart_with_data(data)
            .inner_radius(0.5) // Makes it a donut
            .build();

        if let GuiElementKind::PieChart(config) = &chart.kind {
            assert!((config.inner_radius_ratio - 0.5).abs() < f32::EPSILON);
        } else {
            panic!("Expected PieChart element");
        }
    }

    // -------------------------------------------------------------------------
    // Full Application Flow Test
    // -------------------------------------------------------------------------

    #[test]
    fn test_e2e_complete_ui_flow() {
        // Test: Complete flow simulating a real app interaction
        use crate::element::GuiElement;

        let mut app = create_binding_test_app();

        // Build a complete UI with bound widgets
        let _ui = GuiElement::vstack()
            .spacing(16.0)
            .child(GuiElement::text("Registration Form").text_size(24.0).build())
            .child(
                GuiElement::text_field()
                    .placeholder("Enter name")
                    .bind_field("text_value")
                    .build(),
            )
            .child(
                GuiElement::checkbox("I agree to terms")
                    .bind_field("checked")
                    .build(),
            )
            .child(
                GuiElement::conditional("checked")
                    .true_element(
                        GuiElement::button("Submit").build(),
                    )
                    .false_element(
                        GuiElement::text("Please accept terms").build(),
                    )
                    .build(),
            )
            .build();

        // Simulate user filling out the form
        let _ = app.update(Message::SetStringField {
            field: "text_value".to_string(),
            value: "John Doe".to_string(),
        });

        let _ = app.update(Message::SetBoolField {
            field: "checked".to_string(),
            value: true,
        });

        // Verify final state
        assert_eq!(
            app.state.get_field("text_value"),
            Some(Value::String(Rc::new("John Doe".to_string())))
        );
        assert_eq!(app.state.get_field("checked"), Some(Value::Bool(true)));

        // State changes would trigger re-render showing Submit button
        assert!(app.state.generation() >= 2);
    }

    #[test]
    fn test_e2e_todo_app_pattern() {
        // Test: Todo app pattern - verifies the todo example works correctly
        use crate::element::GuiElement;

        // Create state with todo items (matches todo.rs example structure)
        let mut fields = HashMap::new();
        fields.insert("new_todo_text".to_string(), Value::String(Rc::new(String::new())));
        fields.insert("todo_0_label".to_string(), Value::String(Rc::new("Task 1".to_string())));
        fields.insert("todo_0_completed".to_string(), Value::Bool(false));
        fields.insert("todo_1_label".to_string(), Value::String(Rc::new("Task 2".to_string())));
        fields.insert("todo_1_completed".to_string(), Value::Bool(false));
        fields.insert("total_items".to_string(), Value::Int(2));

        let mut instance = StructInstance::new("TodoState".to_string());
        instance.fields = fields;
        let state = Value::Struct(Rc::new(RefCell::new(instance)));

        // Build todo app UI pattern
        let _ui = GuiElement::vstack()
            .spacing(16.0)
            .child(GuiElement::text("Todo List").text_size(32.0).bold().build())
            .child(
                GuiElement::hstack()
                    .spacing(10.0)
                    .child(GuiElement::text_field().placeholder("What needs to be done?").bind_field("new_todo_text").build())
                    .child(GuiElement::button("Add Todo").build())
                    .build()
            )
            .child(
                GuiElement::vstack()
                    .spacing(8.0)
                    .child(GuiElement::checkbox("Task 1").bind_field("todo_0_completed").build())
                    .child(GuiElement::checkbox("Task 2").bind_field("todo_1_completed").build())
                    .build()
            )
            .build();

        // Create app and verify state management
        let mut app = App {
            state: ReactiveState::new(state),
            spacing: 16.0,
            padding: 20.0,
            registry: Rc::new(RefCell::new(CallbackRegistry::new())),
            executor: None,
            lifecycle: LifecycleManager::new(),
            window_manager: WindowManager::new(),
            modals: ModalManager::new(),
            theme: StratumTheme::default(),
            file_hover_callback: None,
            file_drop_callback: None,
            file_hover_left_callback: None,
            key_press_callback: None,
            key_release_callback: None,
            context_menu: None,
            root_element: None,
            view_fn: None,
            selected_measures: Vec::new(),
        };

        // Initially no todos are completed
        assert_eq!(app.state.get_field("todo_0_completed"), Some(Value::Bool(false)));
        assert_eq!(app.state.get_field("todo_1_completed"), Some(Value::Bool(false)));

        // Toggle first todo - simulates checkbox click
        let _ = app.update(Message::SetBoolField {
            field: "todo_0_completed".to_string(),
            value: true,
        });

        // Verify state updated
        assert_eq!(app.state.get_field("todo_0_completed"), Some(Value::Bool(true)));
        assert_eq!(app.state.get_field("todo_1_completed"), Some(Value::Bool(false)));

        // Toggle second todo
        let _ = app.update(Message::SetBoolField {
            field: "todo_1_completed".to_string(),
            value: true,
        });

        // Verify both are now completed
        assert_eq!(app.state.get_field("todo_0_completed"), Some(Value::Bool(true)));
        assert_eq!(app.state.get_field("todo_1_completed"), Some(Value::Bool(true)));

        // State generation should have increased
        assert!(app.state.generation() >= 2);
    }
}
