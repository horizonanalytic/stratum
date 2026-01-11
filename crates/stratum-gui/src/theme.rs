//! Theming system for Stratum GUI
//!
//! This module provides a comprehensive theming system that bridges Stratum's
//! type system with iced's theming capabilities. It supports:
//! - Built-in themes (Light, Dark, and 20+ presets)
//! - Custom theme creation from Stratum code
//! - Runtime theme switching
//! - Widget-level styling

use iced::theme::Palette;
use iced::Theme;

/// A color in RGBA format
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    /// Red component (0-255)
    pub r: u8,
    /// Green component (0-255)
    pub g: u8,
    /// Blue component (0-255)
    pub b: u8,
    /// Alpha component (0-255, 255 = opaque)
    pub a: u8,
}

impl Color {
    /// Create a new color from RGBA values
    #[must_use]
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Create a new color from RGB values (alpha = 255)
    #[must_use]
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    /// Create a color from a hex string (e.g., "#FF5733" or "FF5733")
    #[must_use]
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        let len = hex.len();

        match len {
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some(Self::rgb(r, g, b))
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
                Some(Self::rgba(r, g, b, a))
            }
            _ => None,
        }
    }

    /// Convert to iced Color
    #[must_use]
    pub fn to_iced(self) -> iced::Color {
        iced::Color::from_rgba8(self.r, self.g, self.b, f32::from(self.a) / 255.0)
    }

    /// Create from iced Color
    #[must_use]
    pub fn from_iced(color: iced::Color) -> Self {
        Self {
            r: (color.r * 255.0) as u8,
            g: (color.g * 255.0) as u8,
            b: (color.b * 255.0) as u8,
            a: (color.a * 255.0) as u8,
        }
    }

    /// Convert to tuple format used elsewhere in the codebase
    #[must_use]
    pub const fn to_tuple(self) -> (u8, u8, u8, u8) {
        (self.r, self.g, self.b, self.a)
    }

    /// Create from tuple format
    #[must_use]
    pub const fn from_tuple(tuple: (u8, u8, u8, u8)) -> Self {
        Self {
            r: tuple.0,
            g: tuple.1,
            b: tuple.2,
            a: tuple.3,
        }
    }

    // Common color constants
    /// Black color
    pub const BLACK: Self = Self::rgb(0, 0, 0);
    /// White color
    pub const WHITE: Self = Self::rgb(255, 255, 255);
    /// Transparent color
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);
    /// Red color
    pub const RED: Self = Self::rgb(255, 0, 0);
    /// Green color
    pub const GREEN: Self = Self::rgb(0, 255, 0);
    /// Blue color
    pub const BLUE: Self = Self::rgb(0, 0, 255);
}

impl Default for Color {
    fn default() -> Self {
        Self::BLACK
    }
}

/// A color palette defining the semantic colors for a theme
#[derive(Debug, Clone, PartialEq)]
pub struct StratumPalette {
    /// Background color
    pub background: Color,
    /// Primary text color
    pub text: Color,
    /// Primary accent color (buttons, links, highlights)
    pub primary: Color,
    /// Success state color (confirmations, success messages)
    pub success: Color,
    /// Warning state color (warnings, cautions)
    pub warning: Color,
    /// Danger state color (errors, destructive actions)
    pub danger: Color,
}

impl StratumPalette {
    /// Create a new palette with all colors specified
    #[must_use]
    pub const fn new(
        background: Color,
        text: Color,
        primary: Color,
        success: Color,
        warning: Color,
        danger: Color,
    ) -> Self {
        Self {
            background,
            text,
            primary,
            success,
            warning,
            danger,
        }
    }

    /// Convert to iced Palette
    #[must_use]
    pub fn to_iced(&self) -> Palette {
        Palette {
            background: self.background.to_iced(),
            text: self.text.to_iced(),
            primary: self.primary.to_iced(),
            success: self.success.to_iced(),
            warning: self.warning.to_iced(),
            danger: self.danger.to_iced(),
        }
    }

    /// Create from iced Palette
    #[must_use]
    pub fn from_iced(palette: &Palette) -> Self {
        Self {
            background: Color::from_iced(palette.background),
            text: Color::from_iced(palette.text),
            primary: Color::from_iced(palette.primary),
            success: Color::from_iced(palette.success),
            warning: Color::from_iced(palette.warning),
            danger: Color::from_iced(palette.danger),
        }
    }

    /// Light theme palette
    pub const LIGHT: Self = Self {
        background: Color::rgb(255, 255, 255),
        text: Color::rgb(0, 0, 0),
        primary: Color::rgb(56, 145, 255),
        success: Color::rgb(18, 165, 91),
        warning: Color::rgb(255, 196, 0),
        danger: Color::rgb(205, 51, 51),
    };

    /// Dark theme palette
    pub const DARK: Self = Self {
        background: Color::rgb(32, 34, 37),
        text: Color::rgb(255, 255, 255),
        primary: Color::rgb(56, 145, 255),
        success: Color::rgb(18, 165, 91),
        warning: Color::rgb(255, 196, 0),
        danger: Color::rgb(205, 51, 51),
    };
}

impl Default for StratumPalette {
    fn default() -> Self {
        Self::DARK
    }
}

/// Built-in theme presets available in Stratum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemePreset {
    /// Light theme
    Light,
    /// Dark theme (default)
    #[default]
    Dark,
    /// Dracula theme (dark purple)
    Dracula,
    /// Nord theme (arctic blue)
    Nord,
    /// Solarized Light theme
    SolarizedLight,
    /// Solarized Dark theme
    SolarizedDark,
    /// Gruvbox Light theme
    GruvboxLight,
    /// Gruvbox Dark theme
    GruvboxDark,
    /// Catppuccin Latte theme (light pastel)
    CatppuccinLatte,
    /// Catppuccin Frappe theme
    CatppuccinFrappe,
    /// Catppuccin Macchiato theme
    CatppuccinMacchiato,
    /// Catppuccin Mocha theme (dark pastel)
    CatppuccinMocha,
    /// Tokyo Night theme
    TokyoNight,
    /// Tokyo Night Storm theme
    TokyoNightStorm,
    /// Tokyo Night Light theme
    TokyoNightLight,
    /// Kanagawa Wave theme
    KanagawaWave,
    /// Kanagawa Dragon theme
    KanagawaDragon,
    /// Kanagawa Lotus theme
    KanagawaLotus,
    /// Moonfly theme
    Moonfly,
    /// Nightfly theme
    Nightfly,
    /// Oxocarbon theme
    Oxocarbon,
    /// Ferra theme
    Ferra,
    /// Follow system preference
    System,
}

impl ThemePreset {
    /// Get all available preset names
    #[must_use]
    pub fn all_names() -> &'static [&'static str] {
        &[
            "light",
            "dark",
            "dracula",
            "nord",
            "solarized_light",
            "solarized_dark",
            "gruvbox_light",
            "gruvbox_dark",
            "catppuccin_latte",
            "catppuccin_frappe",
            "catppuccin_macchiato",
            "catppuccin_mocha",
            "tokyo_night",
            "tokyo_night_storm",
            "tokyo_night_light",
            "kanagawa_wave",
            "kanagawa_dragon",
            "kanagawa_lotus",
            "moonfly",
            "nightfly",
            "oxocarbon",
            "ferra",
            "system",
        ]
    }

    /// Parse a preset from a string name
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().replace('-', "_").as_str() {
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            "dracula" => Some(Self::Dracula),
            "nord" => Some(Self::Nord),
            "solarized_light" => Some(Self::SolarizedLight),
            "solarized_dark" => Some(Self::SolarizedDark),
            "gruvbox_light" => Some(Self::GruvboxLight),
            "gruvbox_dark" => Some(Self::GruvboxDark),
            "catppuccin_latte" => Some(Self::CatppuccinLatte),
            "catppuccin_frappe" => Some(Self::CatppuccinFrappe),
            "catppuccin_macchiato" => Some(Self::CatppuccinMacchiato),
            "catppuccin_mocha" => Some(Self::CatppuccinMocha),
            "tokyo_night" => Some(Self::TokyoNight),
            "tokyo_night_storm" => Some(Self::TokyoNightStorm),
            "tokyo_night_light" => Some(Self::TokyoNightLight),
            "kanagawa_wave" => Some(Self::KanagawaWave),
            "kanagawa_dragon" => Some(Self::KanagawaDragon),
            "kanagawa_lotus" => Some(Self::KanagawaLotus),
            "moonfly" => Some(Self::Moonfly),
            "nightfly" => Some(Self::Nightfly),
            "oxocarbon" => Some(Self::Oxocarbon),
            "ferra" => Some(Self::Ferra),
            "system" => Some(Self::System),
            _ => None,
        }
    }

    /// Get the name of this preset
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::Dracula => "dracula",
            Self::Nord => "nord",
            Self::SolarizedLight => "solarized_light",
            Self::SolarizedDark => "solarized_dark",
            Self::GruvboxLight => "gruvbox_light",
            Self::GruvboxDark => "gruvbox_dark",
            Self::CatppuccinLatte => "catppuccin_latte",
            Self::CatppuccinFrappe => "catppuccin_frappe",
            Self::CatppuccinMacchiato => "catppuccin_macchiato",
            Self::CatppuccinMocha => "catppuccin_mocha",
            Self::TokyoNight => "tokyo_night",
            Self::TokyoNightStorm => "tokyo_night_storm",
            Self::TokyoNightLight => "tokyo_night_light",
            Self::KanagawaWave => "kanagawa_wave",
            Self::KanagawaDragon => "kanagawa_dragon",
            Self::KanagawaLotus => "kanagawa_lotus",
            Self::Moonfly => "moonfly",
            Self::Nightfly => "nightfly",
            Self::Oxocarbon => "oxocarbon",
            Self::Ferra => "ferra",
            Self::System => "system",
        }
    }

    /// Convert to iced Theme
    #[must_use]
    pub fn to_iced_theme(&self) -> Theme {
        match self {
            Self::Light => Theme::Light,
            Self::Dark => Theme::Dark,
            Self::Dracula => Theme::Dracula,
            Self::Nord => Theme::Nord,
            Self::SolarizedLight => Theme::SolarizedLight,
            Self::SolarizedDark => Theme::SolarizedDark,
            Self::GruvboxLight => Theme::GruvboxLight,
            Self::GruvboxDark => Theme::GruvboxDark,
            Self::CatppuccinLatte => Theme::CatppuccinLatte,
            Self::CatppuccinFrappe => Theme::CatppuccinFrappe,
            Self::CatppuccinMacchiato => Theme::CatppuccinMacchiato,
            Self::CatppuccinMocha => Theme::CatppuccinMocha,
            Self::TokyoNight => Theme::TokyoNight,
            Self::TokyoNightStorm => Theme::TokyoNightStorm,
            Self::TokyoNightLight => Theme::TokyoNightLight,
            Self::KanagawaWave => Theme::KanagawaWave,
            Self::KanagawaDragon => Theme::KanagawaDragon,
            Self::KanagawaLotus => Theme::KanagawaLotus,
            Self::Moonfly => Theme::Moonfly,
            Self::Nightfly => Theme::Nightfly,
            Self::Oxocarbon => Theme::Oxocarbon,
            Self::Ferra => Theme::Ferra,
            // System defaults to Dark for now
            // TODO: Detect system preference
            Self::System => Theme::Dark,
        }
    }

    /// Check if this is a light theme
    #[must_use]
    pub const fn is_light(&self) -> bool {
        matches!(
            self,
            Self::Light
                | Self::SolarizedLight
                | Self::GruvboxLight
                | Self::CatppuccinLatte
                | Self::TokyoNightLight
                | Self::KanagawaLotus
        )
    }
}

/// A Stratum theme combining a preset or custom palette
#[derive(Debug, Clone)]
pub enum StratumTheme {
    /// A built-in preset theme
    Preset(ThemePreset),
    /// A custom theme with user-defined palette
    Custom {
        /// Name of the custom theme
        name: String,
        /// The color palette
        palette: StratumPalette,
    },
}

impl StratumTheme {
    /// Create a theme from a preset
    #[must_use]
    pub const fn preset(preset: ThemePreset) -> Self {
        Self::Preset(preset)
    }

    /// Create a custom theme with a name and palette
    #[must_use]
    pub fn custom(name: impl Into<String>, palette: StratumPalette) -> Self {
        Self::Custom {
            name: name.into(),
            palette,
        }
    }

    /// Convert to iced Theme
    #[must_use]
    pub fn to_iced_theme(&self) -> Theme {
        match self {
            Self::Preset(preset) => preset.to_iced_theme(),
            Self::Custom { name, palette } => Theme::custom(name.clone(), palette.to_iced()),
        }
    }

    /// Get the palette for this theme
    #[must_use]
    pub fn palette(&self) -> StratumPalette {
        match self {
            Self::Preset(preset) => StratumPalette::from_iced(&preset.to_iced_theme().palette()),
            Self::Custom { palette, .. } => palette.clone(),
        }
    }

    /// Get the name of this theme
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Preset(preset) => preset.name(),
            Self::Custom { name, .. } => name,
        }
    }

    /// Check if this is a light theme
    #[must_use]
    pub fn is_light(&self) -> bool {
        match self {
            Self::Preset(preset) => preset.is_light(),
            Self::Custom { palette, .. } => {
                // Determine if light based on background luminance
                let bg = &palette.background;
                let luminance =
                    0.299 * f32::from(bg.r) + 0.587 * f32::from(bg.g) + 0.114 * f32::from(bg.b);
                luminance > 127.5
            }
        }
    }
}

impl Default for StratumTheme {
    fn default() -> Self {
        Self::Preset(ThemePreset::default())
    }
}

/// Widget-level styling that can be applied to individual elements
#[derive(Debug, Clone, Default)]
pub struct WidgetStyle {
    /// Background color
    pub background: Option<Color>,
    /// Foreground/text color
    pub foreground: Option<Color>,
    /// Border color
    pub border_color: Option<Color>,
    /// Border width in pixels
    pub border_width: Option<f32>,
    /// Corner radius for rounded corners
    pub corner_radius: Option<f32>,
    /// Shadow configuration
    pub shadow: Option<Shadow>,
}

impl WidgetStyle {
    /// Create a new empty widget style
    #[must_use]
    pub const fn new() -> Self {
        Self {
            background: None,
            foreground: None,
            border_color: None,
            border_width: None,
            corner_radius: None,
            shadow: None,
        }
    }

    /// Set background color
    #[must_use]
    pub const fn with_background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// Set foreground/text color
    #[must_use]
    pub const fn with_foreground(mut self, color: Color) -> Self {
        self.foreground = Some(color);
        self
    }

    /// Set border color
    #[must_use]
    pub const fn with_border_color(mut self, color: Color) -> Self {
        self.border_color = Some(color);
        self
    }

    /// Set border width
    #[must_use]
    pub const fn with_border_width(mut self, width: f32) -> Self {
        self.border_width = Some(width);
        self
    }

    /// Set corner radius
    #[must_use]
    pub const fn with_corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius = Some(radius);
        self
    }

    /// Set shadow
    #[must_use]
    pub const fn with_shadow(mut self, shadow: Shadow) -> Self {
        self.shadow = Some(shadow);
        self
    }

    /// Check if any styling is set
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.background.is_none()
            && self.foreground.is_none()
            && self.border_color.is_none()
            && self.border_width.is_none()
            && self.corner_radius.is_none()
            && self.shadow.is_none()
    }
}

/// Shadow configuration for widgets
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shadow {
    /// Horizontal offset
    pub offset_x: f32,
    /// Vertical offset
    pub offset_y: f32,
    /// Blur radius
    pub blur_radius: f32,
    /// Shadow color
    pub color: Color,
}

impl Shadow {
    /// Create a new shadow
    #[must_use]
    pub const fn new(offset_x: f32, offset_y: f32, blur_radius: f32, color: Color) -> Self {
        Self {
            offset_x,
            offset_y,
            blur_radius,
            color,
        }
    }

    /// Create a subtle shadow
    #[must_use]
    pub const fn subtle() -> Self {
        Self::new(0.0, 2.0, 4.0, Color::rgba(0, 0, 0, 51))
    }

    /// Create a medium shadow
    #[must_use]
    pub const fn medium() -> Self {
        Self::new(0.0, 4.0, 8.0, Color::rgba(0, 0, 0, 77))
    }

    /// Create a strong shadow
    #[must_use]
    pub const fn strong() -> Self {
        Self::new(0.0, 8.0, 16.0, Color::rgba(0, 0, 0, 102))
    }
}

impl Default for Shadow {
    fn default() -> Self {
        Self::subtle()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_creation() {
        let c = Color::rgb(255, 128, 64);
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 128);
        assert_eq!(c.b, 64);
        assert_eq!(c.a, 255);

        let c2 = Color::rgba(100, 150, 200, 128);
        assert_eq!(c2.a, 128);
    }

    #[test]
    fn test_color_from_hex() {
        let c = Color::from_hex("#FF5733").unwrap();
        assert_eq!(c.r, 255);
        assert_eq!(c.g, 87);
        assert_eq!(c.b, 51);
        assert_eq!(c.a, 255);

        let c2 = Color::from_hex("aabbcc").unwrap();
        assert_eq!(c2.r, 170);
        assert_eq!(c2.g, 187);
        assert_eq!(c2.b, 204);

        let c3 = Color::from_hex("#AABBCC80").unwrap();
        assert_eq!(c3.a, 128);

        assert!(Color::from_hex("invalid").is_none());
    }

    #[test]
    fn test_color_to_tuple() {
        let c = Color::rgba(10, 20, 30, 40);
        assert_eq!(c.to_tuple(), (10, 20, 30, 40));
        assert_eq!(Color::from_tuple((10, 20, 30, 40)), c);
    }

    #[test]
    fn test_color_to_iced() {
        let c = Color::rgb(255, 255, 255);
        let iced_c = c.to_iced();
        assert!((iced_c.r - 1.0).abs() < 0.01);
        assert!((iced_c.g - 1.0).abs() < 0.01);
        assert!((iced_c.b - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_palette_creation() {
        let palette = StratumPalette::LIGHT;
        assert_eq!(palette.background, Color::rgb(255, 255, 255));

        let palette2 = StratumPalette::DARK;
        assert_ne!(palette.background, palette2.background);
    }

    #[test]
    fn test_theme_preset_from_name() {
        assert_eq!(ThemePreset::from_name("light"), Some(ThemePreset::Light));
        assert_eq!(ThemePreset::from_name("DARK"), Some(ThemePreset::Dark));
        assert_eq!(
            ThemePreset::from_name("catppuccin-mocha"),
            Some(ThemePreset::CatppuccinMocha)
        );
        assert_eq!(
            ThemePreset::from_name("tokyo_night"),
            Some(ThemePreset::TokyoNight)
        );
        assert!(ThemePreset::from_name("nonexistent").is_none());
    }

    #[test]
    fn test_theme_preset_names() {
        let names = ThemePreset::all_names();
        assert!(names.contains(&"light"));
        assert!(names.contains(&"dark"));
        assert!(names.contains(&"dracula"));
        assert_eq!(names.len(), 23);
    }

    #[test]
    fn test_stratum_theme() {
        let preset_theme = StratumTheme::preset(ThemePreset::Dark);
        assert_eq!(preset_theme.name(), "dark");
        assert!(!preset_theme.is_light());

        let custom_theme = StratumTheme::custom("My Theme", StratumPalette::LIGHT);
        assert_eq!(custom_theme.name(), "My Theme");
        assert!(custom_theme.is_light());
    }

    #[test]
    fn test_widget_style() {
        let style = WidgetStyle::new()
            .with_background(Color::rgb(100, 100, 100))
            .with_corner_radius(8.0)
            .with_border_width(1.0);

        assert!(style.background.is_some());
        assert_eq!(style.corner_radius, Some(8.0));
        assert!(!style.is_empty());

        let empty = WidgetStyle::new();
        assert!(empty.is_empty());
    }

    #[test]
    fn test_is_light_theme() {
        assert!(ThemePreset::Light.is_light());
        assert!(ThemePreset::SolarizedLight.is_light());
        assert!(!ThemePreset::Dark.is_light());
        assert!(!ThemePreset::Dracula.is_light());
    }

    #[test]
    fn test_shadow_presets() {
        let subtle = Shadow::subtle();
        assert!(subtle.blur_radius < Shadow::medium().blur_radius);
        assert!(Shadow::medium().blur_radius < Shadow::strong().blur_radius);
    }
}
