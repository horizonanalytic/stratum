//! Layout components for the Stratum GUI framework
//!
//! This module provides SwiftUI/Flutter-style layout components that wrap
//! iced's layout primitives. These components are exposed to Stratum code
//! as native GUI elements.

use iced::widget::{column, container, row, scrollable, stack, Space};
use iced::{Alignment, Element, Length, Padding};

use crate::runtime::Message;

/// Horizontal alignment options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HAlign {
    /// Align to the start (left in LTR)
    Start,
    /// Center horizontally
    #[default]
    Center,
    /// Align to the end (right in LTR)
    End,
}

impl HAlign {
    /// Convert to iced Alignment
    #[must_use]
    pub fn to_iced(self) -> Alignment {
        match self {
            Self::Start => Alignment::Start,
            Self::Center => Alignment::Center,
            Self::End => Alignment::End,
        }
    }
}

/// Vertical alignment options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VAlign {
    /// Align to the top
    Top,
    /// Center vertically
    #[default]
    Center,
    /// Align to the bottom
    Bottom,
}

impl VAlign {
    /// Convert to iced Alignment
    #[must_use]
    pub fn to_iced(self) -> Alignment {
        match self {
            Self::Top => Alignment::Start,
            Self::Center => Alignment::Center,
            Self::Bottom => Alignment::End,
        }
    }
}

/// Size specification for layout elements
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Size {
    /// Shrink to fit content
    Shrink,
    /// Fill available space
    Fill,
    /// Fill a portion of available space (0.0-1.0)
    FillPortion(u16),
    /// Fixed size in pixels
    Fixed(f32),
}

impl Default for Size {
    fn default() -> Self {
        Self::Shrink
    }
}

impl Size {
    /// Convert to iced Length
    #[must_use]
    pub fn to_iced(self) -> Length {
        match self {
            Self::Shrink => Length::Shrink,
            Self::Fill => Length::Fill,
            Self::FillPortion(portion) => Length::FillPortion(portion),
            Self::Fixed(px) => Length::Fixed(px),
        }
    }
}

/// Common layout properties shared by all layout components
#[derive(Debug, Clone, Default)]
pub struct LayoutProps {
    /// Spacing between children
    pub spacing: f32,
    /// Padding around content
    pub padding: Padding,
    /// Width of the layout
    pub width: Size,
    /// Height of the layout
    pub height: Size,
    /// Maximum width constraint
    pub max_width: Option<f32>,
    /// Maximum height constraint
    pub max_height: Option<f32>,
}

impl LayoutProps {
    /// Create new layout properties with default values
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set spacing between children
    #[must_use]
    pub fn with_spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    /// Set uniform padding
    #[must_use]
    pub fn with_padding(mut self, padding: f32) -> Self {
        self.padding = Padding::new(padding);
        self
    }

    /// Set padding with different values per side
    #[must_use]
    pub fn with_padding_sides(mut self, top: f32, right: f32, bottom: f32, left: f32) -> Self {
        self.padding = Padding::new(top).right(right).bottom(bottom).left(left);
        self
    }

    /// Set width
    #[must_use]
    pub fn with_width(mut self, width: Size) -> Self {
        self.width = width;
        self
    }

    /// Set height
    #[must_use]
    pub fn with_height(mut self, height: Size) -> Self {
        self.height = height;
        self
    }

    /// Set maximum width
    #[must_use]
    pub fn with_max_width(mut self, max_width: f32) -> Self {
        self.max_width = Some(max_width);
        self
    }

    /// Set maximum height
    #[must_use]
    pub fn with_max_height(mut self, max_height: f32) -> Self {
        self.max_height = Some(max_height);
        self
    }
}

/// Vertical stack layout (children arranged top to bottom)
#[derive(Debug, Clone, Default)]
pub struct VStack {
    /// Layout properties
    pub props: LayoutProps,
    /// Horizontal alignment of children
    pub align: HAlign,
}

impl VStack {
    /// Create a new VStack
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set spacing between children
    #[must_use]
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.props.spacing = spacing;
        self
    }

    /// Set uniform padding
    #[must_use]
    pub fn padding(mut self, padding: f32) -> Self {
        self.props.padding = Padding::new(padding);
        self
    }

    /// Set horizontal alignment for children
    #[must_use]
    pub fn align(mut self, align: HAlign) -> Self {
        self.align = align;
        self
    }

    /// Set width
    #[must_use]
    pub fn width(mut self, width: Size) -> Self {
        self.props.width = width;
        self
    }

    /// Set height
    #[must_use]
    pub fn height(mut self, height: Size) -> Self {
        self.props.height = height;
        self
    }

    /// Render this VStack with the given children
    pub fn render<'a>(self, children: Vec<Element<'a, Message>>) -> Element<'a, Message> {
        let mut col = column(children)
            .spacing(self.props.spacing)
            .padding(self.props.padding)
            .align_x(self.align.to_iced());

        col = col.width(self.props.width.to_iced());
        col = col.height(self.props.height.to_iced());

        if let Some(max_width) = self.props.max_width {
            container(col).max_width(max_width).into()
        } else {
            col.into()
        }
    }
}

/// Horizontal stack layout (children arranged left to right)
#[derive(Debug, Clone, Default)]
pub struct HStack {
    /// Layout properties
    pub props: LayoutProps,
    /// Vertical alignment of children
    pub align: VAlign,
}

impl HStack {
    /// Create a new HStack
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set spacing between children
    #[must_use]
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.props.spacing = spacing;
        self
    }

    /// Set uniform padding
    #[must_use]
    pub fn padding(mut self, padding: f32) -> Self {
        self.props.padding = Padding::new(padding);
        self
    }

    /// Set vertical alignment for children
    #[must_use]
    pub fn align(mut self, align: VAlign) -> Self {
        self.align = align;
        self
    }

    /// Set width
    #[must_use]
    pub fn width(mut self, width: Size) -> Self {
        self.props.width = width;
        self
    }

    /// Set height
    #[must_use]
    pub fn height(mut self, height: Size) -> Self {
        self.props.height = height;
        self
    }

    /// Render this HStack with the given children
    pub fn render<'a>(self, children: Vec<Element<'a, Message>>) -> Element<'a, Message> {
        let mut r = row(children)
            .spacing(self.props.spacing)
            .padding(self.props.padding)
            .align_y(self.align.to_iced());

        r = r.width(self.props.width.to_iced());
        r = r.height(self.props.height.to_iced());

        if let Some(max_width) = self.props.max_width {
            container(r).max_width(max_width).into()
        } else {
            r.into()
        }
    }
}

/// Overlay stack layout (children stacked on top of each other)
#[derive(Debug, Clone, Default)]
pub struct ZStack {
    /// Layout properties
    pub props: LayoutProps,
}

impl ZStack {
    /// Create a new ZStack
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set uniform padding
    #[must_use]
    pub fn padding(mut self, padding: f32) -> Self {
        self.props.padding = Padding::new(padding);
        self
    }

    /// Set width
    #[must_use]
    pub fn width(mut self, width: Size) -> Self {
        self.props.width = width;
        self
    }

    /// Set height
    #[must_use]
    pub fn height(mut self, height: Size) -> Self {
        self.props.height = height;
        self
    }

    /// Render this ZStack with the given children (first child is bottommost)
    pub fn render<'a>(self, children: Vec<Element<'a, Message>>) -> Element<'a, Message> {
        let s = stack(children);

        container(s)
            .padding(self.props.padding)
            .width(self.props.width.to_iced())
            .height(self.props.height.to_iced())
            .into()
    }
}

/// Grid layout (children arranged in rows and columns)
#[derive(Debug, Clone)]
pub struct Grid {
    /// Layout properties
    pub props: LayoutProps,
    /// Number of columns
    pub columns: usize,
    /// Horizontal alignment within cells
    pub cell_align_x: HAlign,
    /// Vertical alignment within cells
    pub cell_align_y: VAlign,
}

impl Default for Grid {
    fn default() -> Self {
        Self {
            props: LayoutProps::default(),
            columns: 1,
            cell_align_x: HAlign::Center,
            cell_align_y: VAlign::Center,
        }
    }
}

impl Grid {
    /// Create a new Grid with the specified number of columns
    #[must_use]
    pub fn new(columns: usize) -> Self {
        Self {
            columns: columns.max(1),
            ..Default::default()
        }
    }

    /// Set spacing between cells (both horizontal and vertical)
    #[must_use]
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.props.spacing = spacing;
        self
    }

    /// Set uniform padding
    #[must_use]
    pub fn padding(mut self, padding: f32) -> Self {
        self.props.padding = Padding::new(padding);
        self
    }

    /// Set horizontal alignment within cells
    #[must_use]
    pub fn cell_align_x(mut self, align: HAlign) -> Self {
        self.cell_align_x = align;
        self
    }

    /// Set vertical alignment within cells
    #[must_use]
    pub fn cell_align_y(mut self, align: VAlign) -> Self {
        self.cell_align_y = align;
        self
    }

    /// Set width
    #[must_use]
    pub fn width(mut self, width: Size) -> Self {
        self.props.width = width;
        self
    }

    /// Set height
    #[must_use]
    pub fn height(mut self, height: Size) -> Self {
        self.props.height = height;
        self
    }

    /// Render this Grid with the given children
    pub fn render<'a>(self, children: Vec<Element<'a, Message>>) -> Element<'a, Message> {
        // Build rows from children
        let mut rows: Vec<Element<'a, Message>> = Vec::new();
        let mut current_row: Vec<Element<'a, Message>> = Vec::new();

        for child in children {
            current_row.push(child);
            if current_row.len() >= self.columns {
                // Create a row and reset
                let r = row(std::mem::take(&mut current_row))
                    .spacing(self.props.spacing)
                    .align_y(self.cell_align_y.to_iced());
                rows.push(r.into());
            }
        }

        // Handle remaining children in last row
        if !current_row.is_empty() {
            let r = row(current_row)
                .spacing(self.props.spacing)
                .align_y(self.cell_align_y.to_iced());
            rows.push(r.into());
        }

        let col = column(rows)
            .spacing(self.props.spacing)
            .padding(self.props.padding)
            .align_x(self.cell_align_x.to_iced());

        container(col)
            .width(self.props.width.to_iced())
            .height(self.props.height.to_iced())
            .into()
    }
}

/// Scrollable container
#[derive(Debug, Clone, Default)]
pub struct ScrollView {
    /// Layout properties
    pub props: LayoutProps,
    /// Scroll direction
    pub direction: ScrollDirection,
}

/// Scroll direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollDirection {
    /// Vertical scrolling only
    #[default]
    Vertical,
    /// Horizontal scrolling only
    Horizontal,
    /// Both directions
    Both,
}

impl ScrollView {
    /// Create a new ScrollView
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set scroll direction
    #[must_use]
    pub fn direction(mut self, direction: ScrollDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Set width
    #[must_use]
    pub fn width(mut self, width: Size) -> Self {
        self.props.width = width;
        self
    }

    /// Set height
    #[must_use]
    pub fn height(mut self, height: Size) -> Self {
        self.props.height = height;
        self
    }

    /// Render this ScrollView with the given content
    pub fn render<'a>(self, content: Element<'a, Message>) -> Element<'a, Message> {
        let mut scroll = scrollable(content);

        scroll = scroll.width(self.props.width.to_iced());
        scroll = scroll.height(self.props.height.to_iced());

        // Note: iced 0.13's scrollable direction is set differently
        // The default is vertical-only; horizontal requires explicit configuration
        match self.direction {
            ScrollDirection::Vertical => scroll.into(),
            ScrollDirection::Horizontal => {
                scroll
                    .direction(scrollable::Direction::Horizontal(
                        scrollable::Scrollbar::default(),
                    ))
                    .into()
            }
            ScrollDirection::Both => {
                scroll
                    .direction(scrollable::Direction::Both {
                        vertical: scrollable::Scrollbar::default(),
                        horizontal: scrollable::Scrollbar::default(),
                    })
                    .into()
            }
        }
    }
}

/// Spacer element - fills available space
#[derive(Debug, Clone)]
pub struct Spacer {
    /// Width (optional - if None, shrinks)
    pub width: Option<Size>,
    /// Height (optional - if None, shrinks)
    pub height: Option<Size>,
}

impl Default for Spacer {
    fn default() -> Self {
        Self {
            width: Some(Size::Fill),
            height: Some(Size::Fill),
        }
    }
}

impl Spacer {
    /// Create a spacer that fills in both directions
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a horizontal spacer (fills width, zero height)
    #[must_use]
    pub fn horizontal() -> Self {
        Self {
            width: Some(Size::Fill),
            height: Some(Size::Fixed(0.0)),
        }
    }

    /// Create a vertical spacer (fills height, zero width)
    #[must_use]
    pub fn vertical() -> Self {
        Self {
            width: Some(Size::Fixed(0.0)),
            height: Some(Size::Fill),
        }
    }

    /// Create a fixed-size spacer
    #[must_use]
    pub fn fixed(width: f32, height: f32) -> Self {
        Self {
            width: Some(Size::Fixed(width)),
            height: Some(Size::Fixed(height)),
        }
    }

    /// Set width
    #[must_use]
    pub fn width(mut self, width: Size) -> Self {
        self.width = Some(width);
        self
    }

    /// Set height
    #[must_use]
    pub fn height(mut self, height: Size) -> Self {
        self.height = Some(height);
        self
    }

    /// Render this Spacer
    pub fn render<'a>(self) -> Element<'a, Message> {
        let w = self.width.unwrap_or(Size::Shrink).to_iced();
        let h = self.height.unwrap_or(Size::Shrink).to_iced();
        Space::new().width(w).height(h).into()
    }
}

/// A container that wraps content with alignment and sizing
#[derive(Debug, Clone, Default)]
pub struct Container {
    /// Layout properties
    pub props: LayoutProps,
    /// Horizontal alignment
    pub align_x: HAlign,
    /// Vertical alignment
    pub align_y: VAlign,
    /// Whether to center content horizontally (shorthand)
    pub center_x: bool,
    /// Whether to center content vertically (shorthand)
    pub center_y: bool,
}

impl Container {
    /// Create a new Container
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set uniform padding
    #[must_use]
    pub fn padding(mut self, padding: f32) -> Self {
        self.props.padding = Padding::new(padding);
        self
    }

    /// Set width
    #[must_use]
    pub fn width(mut self, width: Size) -> Self {
        self.props.width = width;
        self
    }

    /// Set height
    #[must_use]
    pub fn height(mut self, height: Size) -> Self {
        self.props.height = height;
        self
    }

    /// Set horizontal alignment
    #[must_use]
    pub fn align_x(mut self, align: HAlign) -> Self {
        self.align_x = align;
        self
    }

    /// Set vertical alignment
    #[must_use]
    pub fn align_y(mut self, align: VAlign) -> Self {
        self.align_y = align;
        self
    }

    /// Center content horizontally (using Fill width)
    #[must_use]
    pub fn center_x(mut self) -> Self {
        self.center_x = true;
        self
    }

    /// Center content vertically (using Fill height)
    #[must_use]
    pub fn center_y(mut self) -> Self {
        self.center_y = true;
        self
    }

    /// Center content in both directions
    #[must_use]
    pub fn center(self) -> Self {
        self.center_x().center_y()
    }

    /// Set maximum width
    #[must_use]
    pub fn max_width(mut self, max_width: f32) -> Self {
        self.props.max_width = Some(max_width);
        self
    }

    /// Set maximum height
    #[must_use]
    pub fn max_height(mut self, max_height: f32) -> Self {
        self.props.max_height = Some(max_height);
        self
    }

    /// Render this Container with the given content
    pub fn render<'a>(self, content: Element<'a, Message>) -> Element<'a, Message> {
        let mut c = container(content).padding(self.props.padding);

        // Apply width
        let width = if self.center_x {
            Length::Fill
        } else {
            self.props.width.to_iced()
        };
        c = c.width(width);

        // Apply height
        let height = if self.center_y {
            Length::Fill
        } else {
            self.props.height.to_iced()
        };
        c = c.height(height);

        // Apply centering
        if self.center_x {
            c = c.center_x(Length::Fill);
        }
        if self.center_y {
            c = c.center_y(Length::Fill);
        }

        // Apply max constraints
        if let Some(max_w) = self.props.max_width {
            c = c.max_width(max_w);
        }
        if let Some(max_h) = self.props.max_height {
            c = c.max_height(max_h);
        }

        c.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_halign_to_iced() {
        assert!(matches!(HAlign::Start.to_iced(), Alignment::Start));
        assert!(matches!(HAlign::Center.to_iced(), Alignment::Center));
        assert!(matches!(HAlign::End.to_iced(), Alignment::End));
    }

    #[test]
    fn test_valign_to_iced() {
        assert!(matches!(VAlign::Top.to_iced(), Alignment::Start));
        assert!(matches!(VAlign::Center.to_iced(), Alignment::Center));
        assert!(matches!(VAlign::Bottom.to_iced(), Alignment::End));
    }

    #[test]
    fn test_size_to_iced() {
        assert!(matches!(Size::Shrink.to_iced(), Length::Shrink));
        assert!(matches!(Size::Fill.to_iced(), Length::Fill));
        assert!(matches!(Size::Fixed(100.0).to_iced(), Length::Fixed(100.0)));
        assert!(matches!(
            Size::FillPortion(2).to_iced(),
            Length::FillPortion(2)
        ));
    }

    #[test]
    fn test_vstack_builder() {
        let vstack = VStack::new()
            .spacing(10.0)
            .padding(8.0)
            .align(HAlign::Start)
            .width(Size::Fill);

        assert_eq!(vstack.props.spacing, 10.0);
        assert_eq!(vstack.align, HAlign::Start);
        assert_eq!(vstack.props.width, Size::Fill);
    }

    #[test]
    fn test_hstack_builder() {
        let hstack = HStack::new()
            .spacing(16.0)
            .align(VAlign::Bottom)
            .height(Size::Fixed(50.0));

        assert_eq!(hstack.props.spacing, 16.0);
        assert_eq!(hstack.align, VAlign::Bottom);
        assert_eq!(hstack.props.height, Size::Fixed(50.0));
    }

    #[test]
    fn test_grid_builder() {
        let grid = Grid::new(3)
            .spacing(8.0)
            .cell_align_x(HAlign::End)
            .cell_align_y(VAlign::Top);

        assert_eq!(grid.columns, 3);
        assert_eq!(grid.props.spacing, 8.0);
        assert_eq!(grid.cell_align_x, HAlign::End);
        assert_eq!(grid.cell_align_y, VAlign::Top);
    }

    #[test]
    fn test_grid_min_columns() {
        let grid = Grid::new(0);
        assert_eq!(grid.columns, 1, "Grid should have at least 1 column");
    }

    #[test]
    fn test_spacer_variants() {
        let spacer = Spacer::new();
        assert_eq!(spacer.width, Some(Size::Fill));
        assert_eq!(spacer.height, Some(Size::Fill));

        let h_spacer = Spacer::horizontal();
        assert_eq!(h_spacer.width, Some(Size::Fill));
        assert_eq!(h_spacer.height, Some(Size::Fixed(0.0)));

        let v_spacer = Spacer::vertical();
        assert_eq!(v_spacer.width, Some(Size::Fixed(0.0)));
        assert_eq!(v_spacer.height, Some(Size::Fill));

        let fixed = Spacer::fixed(10.0, 20.0);
        assert_eq!(fixed.width, Some(Size::Fixed(10.0)));
        assert_eq!(fixed.height, Some(Size::Fixed(20.0)));
    }

    #[test]
    fn test_container_builder() {
        let c = Container::new()
            .padding(16.0)
            .width(Size::Fill)
            .center_x()
            .max_width(800.0);

        assert!(c.center_x);
        assert!(!c.center_y);
        assert_eq!(c.props.max_width, Some(800.0));
    }

    #[test]
    fn test_scrollview_directions() {
        let sv = ScrollView::new().direction(ScrollDirection::Both);
        assert_eq!(sv.direction, ScrollDirection::Both);
    }

    #[test]
    fn test_layout_props_builder() {
        let props = LayoutProps::new()
            .with_spacing(12.0)
            .with_padding(8.0)
            .with_width(Size::Fill)
            .with_max_width(600.0);

        assert_eq!(props.spacing, 12.0);
        assert_eq!(props.width, Size::Fill);
        assert_eq!(props.max_width, Some(600.0));
    }
}
