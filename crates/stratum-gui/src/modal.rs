//! Modal dialog system for Stratum GUI
//!
//! This module provides overlay-based modal dialogs that can be displayed
//! on top of the main content. Modals block interaction with the underlying
//! content until dismissed.

use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, column, container, mouse_area, opaque, row, stack, text, Space};
use iced::{Color, Element, Fill, Length, Theme};

use crate::callback::CallbackId;

/// Modal dialog result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModalResult {
    /// User confirmed/accepted
    Confirm,
    /// User cancelled/dismissed
    Cancel,
    /// Custom button was pressed (index)
    Custom(usize),
}

/// Modal dialog configuration
#[derive(Debug, Clone)]
pub struct ModalConfig {
    /// Dialog title
    pub title: String,
    /// Dialog message/content
    pub message: String,
    /// Primary button text
    pub confirm_text: String,
    /// Cancel button text (if shown)
    pub cancel_text: Option<String>,
    /// Custom button texts
    pub custom_buttons: Vec<String>,
    /// Whether clicking the backdrop dismisses the modal
    pub dismiss_on_backdrop: bool,
    /// Callback to invoke when modal closes
    pub on_close: Option<CallbackId>,
}

impl Default for ModalConfig {
    fn default() -> Self {
        Self {
            title: "Dialog".to_string(),
            message: String::new(),
            confirm_text: "OK".to_string(),
            cancel_text: None,
            custom_buttons: Vec::new(),
            dismiss_on_backdrop: true,
            on_close: None,
        }
    }
}

impl ModalConfig {
    /// Create an alert dialog with just an OK button
    #[must_use]
    pub fn alert(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            ..Default::default()
        }
    }

    /// Create a confirmation dialog with OK and Cancel buttons
    #[must_use]
    pub fn confirm(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            cancel_text: Some("Cancel".to_string()),
            ..Default::default()
        }
    }

    /// Set the confirm button text
    #[must_use]
    pub fn with_confirm_text(mut self, text: impl Into<String>) -> Self {
        self.confirm_text = text.into();
        self
    }

    /// Set the cancel button text (shows cancel button if set)
    #[must_use]
    pub fn with_cancel_text(mut self, text: impl Into<String>) -> Self {
        self.cancel_text = Some(text.into());
        self
    }

    /// Add a custom button
    #[must_use]
    pub fn with_custom_button(mut self, text: impl Into<String>) -> Self {
        self.custom_buttons.push(text.into());
        self
    }

    /// Set whether clicking backdrop dismisses the modal
    #[must_use]
    pub fn with_dismiss_on_backdrop(mut self, dismiss: bool) -> Self {
        self.dismiss_on_backdrop = dismiss;
        self
    }

    /// Set the callback to invoke when modal closes
    #[must_use]
    pub fn with_on_close(mut self, callback: CallbackId) -> Self {
        self.on_close = Some(callback);
        self
    }
}

/// Modal dialog state
#[derive(Debug, Clone)]
pub struct Modal {
    /// Configuration
    config: ModalConfig,
    /// Whether the modal is visible
    visible: bool,
}

impl Modal {
    /// Create a new modal with the given configuration
    #[must_use]
    pub fn new(config: ModalConfig) -> Self {
        Self {
            config,
            visible: false,
        }
    }

    /// Create and show a modal
    #[must_use]
    pub fn show(config: ModalConfig) -> Self {
        Self {
            config,
            visible: true,
        }
    }

    /// Show the modal
    pub fn open(&mut self) {
        self.visible = true;
    }

    /// Hide the modal
    pub fn close(&mut self) {
        self.visible = false;
    }

    /// Check if the modal is visible
    #[must_use]
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Get the modal configuration
    #[must_use]
    pub fn config(&self) -> &ModalConfig {
        &self.config
    }

    /// Get the on_close callback if set
    #[must_use]
    pub fn on_close_callback(&self) -> Option<CallbackId> {
        self.config.on_close
    }
}

/// Modal manager that handles multiple modal dialogs
#[derive(Debug, Default)]
pub struct ModalManager {
    /// Stack of active modals (last is topmost)
    modals: Vec<Modal>,
}

impl ModalManager {
    /// Create a new modal manager
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Show a modal dialog
    pub fn show(&mut self, config: ModalConfig) {
        self.modals.push(Modal::show(config));
    }

    /// Show an alert dialog
    pub fn alert(&mut self, title: impl Into<String>, message: impl Into<String>) {
        self.show(ModalConfig::alert(title, message));
    }

    /// Show a confirmation dialog
    pub fn confirm(&mut self, title: impl Into<String>, message: impl Into<String>) {
        self.show(ModalConfig::confirm(title, message));
    }

    /// Close the topmost modal
    pub fn close_top(&mut self) -> Option<CallbackId> {
        self.modals.pop().and_then(|m| m.on_close_callback())
    }

    /// Close all modals
    pub fn close_all(&mut self) {
        self.modals.clear();
    }

    /// Check if any modal is visible
    #[must_use]
    pub fn has_modal(&self) -> bool {
        !self.modals.is_empty()
    }

    /// Get the topmost modal
    #[must_use]
    pub fn top(&self) -> Option<&Modal> {
        self.modals.last()
    }

    /// Get the number of open modals
    #[must_use]
    pub fn count(&self) -> usize {
        self.modals.len()
    }
}

/// Messages for modal interaction
#[derive(Debug, Clone)]
pub enum ModalMessage {
    /// Modal backdrop was clicked
    BackdropClicked,
    /// Confirm button was clicked
    Confirm,
    /// Cancel button was clicked
    Cancel,
    /// Custom button was clicked
    CustomButton(usize),
}

/// Render a modal overlay on top of content
///
/// This function creates a layered element with the modal on top.
/// The backdrop dims the content underneath and can optionally dismiss the modal.
pub fn modal_overlay<'a, Message: Clone + 'a>(
    base: impl Into<Element<'a, Message>>,
    modal: Option<&'a Modal>,
    on_result: impl Fn(ModalResult) -> Message + 'a,
    on_backdrop: Option<Message>,
) -> Element<'a, Message> {
    let base = base.into();

    let Some(modal) = modal else {
        return base;
    };

    if !modal.is_visible() {
        return base;
    }

    let config = modal.config();

    // Create backdrop
    let backdrop = mouse_area(
        container(Space::new().width(Fill).height(Fill))
            .style(|_theme: &Theme| container::Style {
                background: Some(Color::from_rgba(0.0, 0.0, 0.0, 0.5).into()),
                ..Default::default()
            })
            .width(Fill)
            .height(Fill),
    );

    let backdrop = if let Some(msg) = on_backdrop {
        backdrop.on_press(msg)
    } else {
        backdrop
    };

    // Create dialog content
    let title = text(&config.title).size(20);

    let message = text(&config.message).size(14);

    // Build buttons row
    let mut buttons = row![].spacing(8);

    // Cancel button (if present)
    if let Some(cancel_text) = &config.cancel_text {
        let on_cancel = on_result(ModalResult::Cancel);
        buttons = buttons.push(
            button(text(cancel_text))
                .on_press(on_cancel)
                .padding([8, 16]),
        );
    }

    // Custom buttons
    for (i, btn_text) in config.custom_buttons.iter().enumerate() {
        let on_custom = on_result(ModalResult::Custom(i));
        buttons = buttons.push(button(text(btn_text)).on_press(on_custom).padding([8, 16]));
    }

    // Confirm button
    let on_confirm = on_result(ModalResult::Confirm);
    buttons = buttons.push(
        button(text(&config.confirm_text))
            .on_press(on_confirm)
            .padding([8, 16]),
    );

    let dialog_content = column![title, message, buttons]
        .spacing(16)
        .padding(24)
        .width(Length::Shrink);

    let dialog = container(dialog_content)
        .style(|theme: &Theme| {
            let palette = theme.palette();
            container::Style {
                background: Some(palette.background.into()),
                border: iced::Border {
                    color: palette.primary,
                    width: 1.0,
                    radius: 8.0.into(),
                },
                shadow: iced::Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.3),
                    offset: iced::Vector::new(0.0, 4.0),
                    blur_radius: 12.0,
                },
                ..Default::default()
            }
        })
        .max_width(400);

    // Wrap dialog in opaque to prevent backdrop click from passing through
    let dialog_layer = container(opaque(dialog))
        .width(Fill)
        .height(Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center);

    // Stack layers
    stack![base, backdrop, dialog_layer].into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modal_config_alert() {
        let config = ModalConfig::alert("Error", "Something went wrong");
        assert_eq!(config.title, "Error");
        assert_eq!(config.message, "Something went wrong");
        assert_eq!(config.confirm_text, "OK");
        assert!(config.cancel_text.is_none());
    }

    #[test]
    fn test_modal_config_confirm() {
        let config = ModalConfig::confirm("Delete?", "Are you sure?");
        assert_eq!(config.title, "Delete?");
        assert!(config.cancel_text.is_some());
    }

    #[test]
    fn test_modal_config_builder() {
        let config = ModalConfig::alert("Test", "Message")
            .with_confirm_text("Yes")
            .with_cancel_text("No")
            .with_custom_button("Maybe")
            .with_dismiss_on_backdrop(false);

        assert_eq!(config.confirm_text, "Yes");
        assert_eq!(config.cancel_text, Some("No".to_string()));
        assert_eq!(config.custom_buttons.len(), 1);
        assert!(!config.dismiss_on_backdrop);
    }

    #[test]
    fn test_modal_visibility() {
        let mut modal = Modal::new(ModalConfig::alert("Test", ""));
        assert!(!modal.is_visible());

        modal.open();
        assert!(modal.is_visible());

        modal.close();
        assert!(!modal.is_visible());
    }

    #[test]
    fn test_modal_manager() {
        let mut manager = ModalManager::new();
        assert!(!manager.has_modal());

        manager.alert("Test 1", "Message 1");
        assert!(manager.has_modal());
        assert_eq!(manager.count(), 1);

        manager.confirm("Test 2", "Message 2");
        assert_eq!(manager.count(), 2);

        manager.close_top();
        assert_eq!(manager.count(), 1);

        manager.close_all();
        assert!(!manager.has_modal());
    }
}
