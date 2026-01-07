//! Application lifecycle management for Stratum GUI
//!
//! This module handles the initialization, event loop, and shutdown phases
//! of a GUI application.

use stratum_core::bytecode::Value;

use crate::callback::{Callback, CallbackExecutor, CallbackId, CallbackRegistry};
use crate::error::GuiResult;

/// Lifecycle phase of the application
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecyclePhase {
    /// Application is being constructed (before window opens)
    Initializing,
    /// Application is running (window is open, event loop active)
    Running,
    /// Application is shutting down (window closing)
    ShuttingDown,
    /// Application has terminated
    Terminated,
}

/// Lifecycle hooks for application events
///
/// These callbacks are invoked at specific points in the application lifecycle.
#[derive(Debug, Default, Clone)]
pub struct LifecycleHooks {
    /// Called when the application starts, before the window opens
    pub on_init: Option<CallbackId>,
    /// Called when the window is about to close
    pub on_shutdown: Option<CallbackId>,
    /// Called when the window gains focus
    pub on_focus: Option<CallbackId>,
    /// Called when the window loses focus
    pub on_blur: Option<CallbackId>,
    /// Called when the window is resized
    pub on_resize: Option<CallbackId>,
}

impl LifecycleHooks {
    /// Create new empty lifecycle hooks
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the on_init callback
    #[must_use]
    pub fn with_on_init(mut self, id: CallbackId) -> Self {
        self.on_init = Some(id);
        self
    }

    /// Set the on_shutdown callback
    #[must_use]
    pub fn with_on_shutdown(mut self, id: CallbackId) -> Self {
        self.on_shutdown = Some(id);
        self
    }

    /// Set the on_focus callback
    #[must_use]
    pub fn with_on_focus(mut self, id: CallbackId) -> Self {
        self.on_focus = Some(id);
        self
    }

    /// Set the on_blur callback
    #[must_use]
    pub fn with_on_blur(mut self, id: CallbackId) -> Self {
        self.on_blur = Some(id);
        self
    }

    /// Set the on_resize callback
    #[must_use]
    pub fn with_on_resize(mut self, id: CallbackId) -> Self {
        self.on_resize = Some(id);
        self
    }
}

/// Lifecycle manager for coordinating application phases
pub struct LifecycleManager {
    /// Current phase
    phase: LifecyclePhase,
    /// Registered hooks
    hooks: LifecycleHooks,
    /// Callback executor for invoking hooks
    executor: Option<CallbackExecutor>,
}

impl LifecycleManager {
    /// Create a new lifecycle manager
    #[must_use]
    pub fn new() -> Self {
        Self {
            phase: LifecyclePhase::Initializing,
            hooks: LifecycleHooks::new(),
            executor: None,
        }
    }

    /// Set the callback executor
    pub fn set_executor(&mut self, executor: CallbackExecutor) {
        self.executor = Some(executor);
    }

    /// Get the current lifecycle phase
    #[must_use]
    pub fn phase(&self) -> LifecyclePhase {
        self.phase
    }

    /// Set lifecycle hooks
    pub fn set_hooks(&mut self, hooks: LifecycleHooks) {
        self.hooks = hooks;
    }

    /// Get a reference to the hooks
    #[must_use]
    pub fn hooks(&self) -> &LifecycleHooks {
        &self.hooks
    }

    /// Transition to the Running phase and invoke on_init hook
    pub fn start(&mut self) -> GuiResult<()> {
        if self.phase != LifecyclePhase::Initializing {
            return Ok(()); // Already started
        }

        if let (Some(executor), Some(callback_id)) = (&self.executor, self.hooks.on_init) {
            executor.execute_no_args(callback_id)?;
        }

        self.phase = LifecyclePhase::Running;
        Ok(())
    }

    /// Transition to ShuttingDown phase and invoke on_shutdown hook
    pub fn shutdown(&mut self) -> GuiResult<()> {
        if self.phase == LifecyclePhase::Terminated {
            return Ok(()); // Already terminated
        }

        self.phase = LifecyclePhase::ShuttingDown;

        if let (Some(executor), Some(callback_id)) = (&self.executor, self.hooks.on_shutdown) {
            executor.execute_no_args(callback_id)?;
        }

        self.phase = LifecyclePhase::Terminated;
        Ok(())
    }

    /// Handle window focus event
    pub fn on_focus(&mut self) -> GuiResult<()> {
        if let (Some(executor), Some(callback_id)) = (&self.executor, self.hooks.on_focus) {
            executor.execute_no_args(callback_id)?;
        }
        Ok(())
    }

    /// Handle window blur event
    pub fn on_blur(&mut self) -> GuiResult<()> {
        if let (Some(executor), Some(callback_id)) = (&self.executor, self.hooks.on_blur) {
            executor.execute_no_args(callback_id)?;
        }
        Ok(())
    }

    /// Handle window resize event
    pub fn on_resize(&mut self, width: u32, height: u32) -> GuiResult<()> {
        if let (Some(executor), Some(callback_id)) = (&self.executor, self.hooks.on_resize) {
            let args = vec![Value::Int(width as i64), Value::Int(height as i64)];
            executor.execute(callback_id, args)?;
        }
        Ok(())
    }

    /// Check if the application is running
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.phase == LifecyclePhase::Running
    }

    /// Check if the application has terminated
    #[must_use]
    pub fn is_terminated(&self) -> bool {
        self.phase == LifecyclePhase::Terminated
    }
}

impl Default for LifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for LifecycleManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LifecycleManager")
            .field("phase", &self.phase)
            .field("hooks", &self.hooks)
            .field("has_executor", &self.executor.is_some())
            .finish()
    }
}

/// Builder for configuring lifecycle hooks
pub struct LifecycleBuilder<'a> {
    registry: &'a mut CallbackRegistry,
    hooks: LifecycleHooks,
}

impl<'a> LifecycleBuilder<'a> {
    /// Create a new lifecycle builder
    pub fn new(registry: &'a mut CallbackRegistry) -> Self {
        Self {
            registry,
            hooks: LifecycleHooks::new(),
        }
    }

    /// Set the on_init callback from a Stratum value
    ///
    /// # Errors
    /// Returns an error if the value is not callable
    pub fn on_init(mut self, handler: Value) -> GuiResult<Self> {
        let callback = Callback::new(handler)?.with_description("on_init");
        let id = self.registry.register(callback);
        self.hooks.on_init = Some(id);
        Ok(self)
    }

    /// Set the on_shutdown callback from a Stratum value
    ///
    /// # Errors
    /// Returns an error if the value is not callable
    pub fn on_shutdown(mut self, handler: Value) -> GuiResult<Self> {
        let callback = Callback::new(handler)?.with_description("on_shutdown");
        let id = self.registry.register(callback);
        self.hooks.on_shutdown = Some(id);
        Ok(self)
    }

    /// Set the on_focus callback from a Stratum value
    ///
    /// # Errors
    /// Returns an error if the value is not callable
    pub fn on_focus(mut self, handler: Value) -> GuiResult<Self> {
        let callback = Callback::new(handler)?.with_description("on_focus");
        let id = self.registry.register(callback);
        self.hooks.on_focus = Some(id);
        Ok(self)
    }

    /// Set the on_blur callback from a Stratum value
    ///
    /// # Errors
    /// Returns an error if the value is not callable
    pub fn on_blur(mut self, handler: Value) -> GuiResult<Self> {
        let callback = Callback::new(handler)?.with_description("on_blur");
        let id = self.registry.register(callback);
        self.hooks.on_blur = Some(id);
        Ok(self)
    }

    /// Set the on_resize callback from a Stratum value
    ///
    /// # Errors
    /// Returns an error if the value is not callable
    pub fn on_resize(mut self, handler: Value) -> GuiResult<Self> {
        let callback = Callback::new(handler)?.with_description("on_resize");
        let id = self.registry.register(callback);
        self.hooks.on_resize = Some(id);
        Ok(self)
    }

    /// Build the lifecycle hooks
    #[must_use]
    pub fn build(self) -> LifecycleHooks {
        self.hooks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lifecycle_phases() {
        let mut manager = LifecycleManager::new();
        assert_eq!(manager.phase(), LifecyclePhase::Initializing);
        assert!(!manager.is_running());
        assert!(!manager.is_terminated());

        manager.start().unwrap();
        assert_eq!(manager.phase(), LifecyclePhase::Running);
        assert!(manager.is_running());

        manager.shutdown().unwrap();
        assert_eq!(manager.phase(), LifecyclePhase::Terminated);
        assert!(manager.is_terminated());
    }

    #[test]
    fn test_lifecycle_hooks_builder() {
        let hooks = LifecycleHooks::new()
            .with_on_init(CallbackId::new(1))
            .with_on_shutdown(CallbackId::new(2));

        assert_eq!(hooks.on_init, Some(CallbackId::new(1)));
        assert_eq!(hooks.on_shutdown, Some(CallbackId::new(2)));
        assert!(hooks.on_focus.is_none());
    }

    #[test]
    fn test_double_start_is_idempotent() {
        let mut manager = LifecycleManager::new();
        manager.start().unwrap();
        manager.start().unwrap(); // Should not panic or error
        assert!(manager.is_running());
    }

    #[test]
    fn test_double_shutdown_is_idempotent() {
        let mut manager = LifecycleManager::new();
        manager.start().unwrap();
        manager.shutdown().unwrap();
        manager.shutdown().unwrap(); // Should not panic or error
        assert!(manager.is_terminated());
    }
}
