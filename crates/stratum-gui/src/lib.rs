//! Stratum GUI - GUI framework for the Stratum programming language
//!
//! This crate provides a declarative, reactive GUI framework built on iced.
//! It bridges Stratum's Value types to iced's Elm-inspired architecture.
//!
//! # Architecture
//!
//! The framework uses iced's retained-mode architecture with:
//! - **State**: Application state stored as Stratum values
//! - **Message**: Events that trigger state changes
//! - **Update**: Pure functions that handle messages
//! - **View**: Declarative widget tree built from state
//!
//! # Backend Choice: iced
//!
//! iced was chosen over egui for several reasons:
//! - Elm-inspired architecture aligns with Stratum's reactive state binding
//! - Reactive rendering (0.14+) provides 60-80% CPU savings
//! - Structured message-passing scales better for complex apps
//! - Time travel debugging aids Workshop IDE development
//! - Active development (used by System76's COSMIC desktop)
//!
//! # Core Components
//!
//! - [`GuiRuntime`]: Main runtime that manages the iced application
//! - [`ReactiveState`]: Reactive state container with change detection
//! - [`CallbackRegistry`]: Registry for Stratum closures as event handlers
//! - [`CallbackExecutor`]: Executor for invoking callbacks via the VM
//! - [`LifecycleManager`]: Manages app lifecycle (init, running, shutdown)
//! - [`WindowManager`]: Manages multiple windows in the application
//! - [`ModalManager`]: Manages modal dialogs with overlay support

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Error types for GUI operations
pub mod error;

/// Callback system for Stratum closure integration
pub mod callback;

/// Application lifecycle management
pub mod lifecycle;

/// Modal dialog system
pub mod modal;

/// Core runtime that bridges Stratum values to iced
pub mod runtime;

/// State binding and reactivity
pub mod state;

/// Widget abstractions over iced
pub mod widgets;

/// Window management and multi-window support
pub mod window;

/// Layout components (VStack, HStack, Grid, etc.)
pub mod layout;

/// GUI element types for Stratum integration
pub mod element;

/// Native functions for GUI element creation
pub mod natives;

/// Chart widgets (BarChart, LineChart, PieChart)
pub mod charts;

/// Theming and styling system
pub mod theme;

// Re-exports for convenience
pub use callback::{Callback, CallbackExecutor, CallbackId, CallbackRegistry};
pub use error::{GuiError, GuiResult};
pub use lifecycle::{LifecycleBuilder, LifecycleHooks, LifecycleManager, LifecyclePhase};
pub use modal::{Modal, ModalConfig, ModalManager, ModalMessage, ModalResult};
pub use runtime::{AppConfig, AppTheme, Backend, GuiRuntime, Message};
pub use state::{ComputedProperty, ComputedPropertyAccess, FieldBinding, ReactiveState, StateSubscription};
pub use widgets::{
    get_binding_path, is_state_binding, resolve_binding, LayoutConfig, ResolvedBinding, TextStyle,
};
pub use window::{Position, WindowEvent, WindowId, WindowLevel, WindowManager, WindowSettings, WindowState};
pub use layout::{Container, Grid, HAlign, HStack, LayoutProps, ScrollDirection, ScrollView, Size, Spacer, VAlign, VStack, ZStack};
pub use element::{
    ConditionalConfig, ForEachConfig, GuiElement, GuiElementKind,
    // OLAP Cube widget configs
    CubeTableConfig, CubeChartConfig, CubeChartType,
    DimensionFilterConfig, HierarchyNavigatorConfig, MeasureSelectorConfig,
};
pub use natives::gui_native_functions;
pub use charts::{BarChartConfig, LineChartConfig, PieChartConfig, DataPoint, DataSeries, CHART_COLORS};
pub use theme::{Color, Shadow, StratumPalette, StratumTheme, ThemePreset, WidgetStyle};
