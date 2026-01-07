//! Debug panel for displaying call stack and variables
//!
//! This panel shows the current call stack and local variables
//! when the debugger is paused.

use iced::widget::{column, container, scrollable, text, Column, Row};
use iced::{Element, Font, Length, Theme};
use stratum_core::{DebugStackFrame, DebugVariable};

/// Message type for debug panel actions
#[derive(Debug, Clone)]
pub enum DebugPanelMessage {
    /// Select a stack frame
    SelectFrame(usize),
}

/// Debug panel for displaying debug state
#[derive(Debug, Default)]
pub struct DebugPanel {
    /// Currently selected stack frame
    selected_frame: usize,
}

impl DebugPanel {
    /// Create a new debug panel
    pub fn new() -> Self {
        Self { selected_frame: 0 }
    }

    /// Handle a debug panel message
    pub fn update(&mut self, message: DebugPanelMessage) {
        match message {
            DebugPanelMessage::SelectFrame(index) => {
                self.selected_frame = index;
            }
        }
    }

    /// Clear the panel state
    pub fn clear(&mut self) {
        self.selected_frame = 0;
    }

    /// Render the debug panel
    pub fn view<'a>(
        &'a self,
        call_stack: &'a [DebugStackFrame],
        locals: &'a [DebugVariable],
        is_debugging: bool,
    ) -> Element<'a, DebugPanelMessage> {
        if !is_debugging || call_stack.is_empty() {
            return self.empty_view();
        }

        let call_stack_section = self.call_stack_view(call_stack);
        let variables_section = self.variables_view(locals);

        container(
            column![
                call_stack_section,
                variables_section,
            ]
            .spacing(10)
            .padding(10),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    /// Render empty view when not debugging
    fn empty_view(&self) -> Element<'_, DebugPanelMessage> {
        container(
            text("Not debugging")
                .size(12)
                .color(iced::Color::from_rgb(0.5, 0.5, 0.5)),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    }

    /// Render the call stack section
    fn call_stack_view<'a>(&self, call_stack: &'a [DebugStackFrame]) -> Element<'a, DebugPanelMessage> {
        let header = text("Call Stack")
            .size(14)
            .font(Font::DEFAULT)
            .color(iced::Color::from_rgb(0.8, 0.8, 0.8));

        let frames: Vec<Element<'a, DebugPanelMessage>> = call_stack
            .iter()
            .map(|frame| {
                let is_selected = frame.index == self.selected_frame;
                let color = if is_selected {
                    iced::Color::from_rgb(1.0, 0.8, 0.0)
                } else {
                    iced::Color::from_rgb(0.7, 0.7, 0.7)
                };

                let location = if let Some(ref file) = frame.file {
                    format!("{}:{}", file, frame.line)
                } else {
                    format!("line {}", frame.line)
                };

                text(format!("{} - {}", frame.function_name, location))
                    .size(12)
                    .font(Font::MONOSPACE)
                    .color(color)
                    .into()
            })
            .collect();

        let content = Column::with_children(frames).spacing(2);

        column![
            header,
            scrollable(content).height(Length::Fixed(120.0)),
        ]
        .spacing(4)
        .into()
    }

    /// Render the variables section
    fn variables_view<'a>(&self, locals: &'a [DebugVariable]) -> Element<'a, DebugPanelMessage> {
        let header = text("Variables")
            .size(14)
            .font(Font::DEFAULT)
            .color(iced::Color::from_rgb(0.8, 0.8, 0.8));

        if locals.is_empty() {
            return column![
                header,
                text("(no variables)")
                    .size(11)
                    .color(iced::Color::from_rgb(0.5, 0.5, 0.5)),
            ]
            .spacing(4)
            .into();
        }

        let vars: Vec<Element<'a, DebugPanelMessage>> = locals
            .iter()
            .map(|var| {
                let name_color = iced::Color::from_rgb(0.6, 0.8, 1.0);
                let value_color = iced::Color::from_rgb(0.9, 0.7, 0.5);
                let type_color = iced::Color::from_rgb(0.5, 0.7, 0.5);

                Row::new()
                    .push(
                        text(&var.name)
                            .size(12)
                            .font(Font::MONOSPACE)
                            .color(name_color),
                    )
                    .push(text(": ").size(12).color(iced::Color::from_rgb(0.6, 0.6, 0.6)))
                    .push(
                        text(&var.type_name)
                            .size(12)
                            .font(Font::MONOSPACE)
                            .color(type_color),
                    )
                    .push(text(" = ").size(12).color(iced::Color::from_rgb(0.6, 0.6, 0.6)))
                    .push(
                        text(&var.value)
                            .size(12)
                            .font(Font::MONOSPACE)
                            .color(value_color),
                    )
                    .spacing(0)
                    .into()
            })
            .collect();

        let content = Column::with_children(vars).spacing(2);

        column![
            header,
            scrollable(content).height(Length::Fill),
        ]
        .spacing(4)
        .into()
    }
}
