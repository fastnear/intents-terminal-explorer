//! Color theme system for Ratacat
//!
//! Provides 4 retro-inspired color schemes that can be selected via CLI flag.

use ratatui::style::Color;
use std::fmt;

/// Available color themes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    /// Nord-inspired theme (default) - modern muted colors
    Nord,
    /// Classic DOS Blue - bright white on blue background
    DosBlue,
    /// Amber CRT - orange/amber text on black (retro terminal)
    AmberCrt,
    /// Green Phosphor - green text on black (classic terminal)
    GreenPhosphor,
}

impl Theme {
    /// Parse theme name from string
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "nord" => Ok(Theme::Nord),
            "dos" | "dosblue" | "dos-blue" => Ok(Theme::DosBlue),
            "amber" | "ambercrt" | "amber-crt" => Ok(Theme::AmberCrt),
            "green" | "greenphosphor" | "green-phosphor" => Ok(Theme::GreenPhosphor),
            _ => Err(format!(
                "Unknown theme '{}'. Available: nord, dos-blue, amber-crt, green-phosphor",
                s
            )),
        }
    }

    /// Get the color scheme for this theme
    pub fn colors(&self) -> ColorScheme {
        match self {
            Theme::Nord => ColorScheme::nord(),
            Theme::DosBlue => ColorScheme::dos_blue(),
            Theme::AmberCrt => ColorScheme::amber_crt(),
            Theme::GreenPhosphor => ColorScheme::green_phosphor(),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme::Nord
    }
}

impl fmt::Display for Theme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Theme::Nord => write!(f, "nord"),
            Theme::DosBlue => write!(f, "dos-blue"),
            Theme::AmberCrt => write!(f, "amber-crt"),
            Theme::GreenPhosphor => write!(f, "green-phosphor"),
        }
    }
}

/// Color scheme for a theme
#[derive(Debug, Clone, Copy)]
pub struct ColorScheme {
    /// Background color for normal content
    pub background: Color,
    /// Background color for focused panes (subtle tint)
    pub background_focused: Color,
    /// Primary text color
    pub text: Color,
    /// Dimmed text color (for secondary info)
    pub text_dim: Color,
    /// Border color for focused elements
    pub focus_border: Color,
    /// Border color for unfocused elements
    pub unfocused_border: Color,
    /// Background for selected list items
    pub selection_bg: Color,
    /// Foreground for selected list items
    pub selection_fg: Color,
    /// Badge color (e.g., [OWNED])
    pub badge: Color,
    /// Toast success message color
    pub toast_success: Color,
    /// Toast error message color
    pub toast_error: Color,
    /// Debug panel indicator color
    pub debug_indicator: Color,
}

impl ColorScheme {
    /// Nord theme (default) - Modern muted colors
    pub fn nord() -> Self {
        Self {
            background: Color::Black,
            background_focused: Color::Rgb(40, 40, 40), // ~1.6:1 contrast (VS Code-inspired)
            text: Color::White,
            text_dim: Color::Gray,
            focus_border: Color::Yellow,
            unfocused_border: Color::Gray,
            selection_bg: Color::Yellow,
            selection_fg: Color::Black,
            badge: Color::Cyan,
            toast_success: Color::Green,
            toast_error: Color::Red,
            debug_indicator: Color::Magenta,
        }
    }

    /// DOS Blue theme - Classic DOS aesthetic
    pub fn dos_blue() -> Self {
        Self {
            background: Color::Blue,
            background_focused: Color::Rgb(20, 20, 255), // Brighter blue for focus
            text: Color::White,
            text_dim: Color::LightBlue,
            focus_border: Color::Yellow,
            unfocused_border: Color::Cyan,
            selection_bg: Color::Cyan,
            selection_fg: Color::Black,
            badge: Color::LightCyan,
            toast_success: Color::LightGreen,
            toast_error: Color::LightRed,
            debug_indicator: Color::LightMagenta,
        }
    }

    /// Amber CRT theme - Retro terminal
    pub fn amber_crt() -> Self {
        let amber = Color::Rgb(255, 176, 0);
        let amber_bright = Color::Rgb(255, 200, 100);
        let amber_dim = Color::Rgb(180, 120, 0);

        Self {
            background: Color::Black,
            background_focused: Color::Rgb(30, 20, 0), // Stronger amber tint for focus
            text: amber,
            text_dim: amber_dim,
            focus_border: amber_bright,
            unfocused_border: amber_dim,
            selection_bg: amber,
            selection_fg: Color::Black,
            badge: amber_bright,
            toast_success: Color::Rgb(100, 255, 100), // Bright green stands out
            toast_error: Color::Red,
            debug_indicator: Color::Rgb(255, 100, 255), // Magenta
        }
    }

    /// Green Phosphor theme - Classic green screen
    pub fn green_phosphor() -> Self {
        let green = Color::Rgb(0, 255, 0);
        let green_dim = Color::Rgb(0, 180, 0);
        let green_bright = Color::Rgb(100, 255, 100);

        Self {
            background: Color::Black,
            background_focused: Color::Rgb(0, 25, 0), // Stronger green tint for focus
            text: green,
            text_dim: green_dim,
            focus_border: green_bright,
            unfocused_border: green_dim,
            selection_bg: green,
            selection_fg: Color::Black,
            badge: green_bright,
            toast_success: green_bright,
            toast_error: Color::Red, // Red stands out against green
            debug_indicator: Color::Cyan, // Cyan provides contrast
        }
    }
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self::nord()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_parsing() {
        assert_eq!(Theme::from_str("nord").unwrap(), Theme::Nord);
        assert_eq!(Theme::from_str("NORD").unwrap(), Theme::Nord);
        assert_eq!(Theme::from_str("dos").unwrap(), Theme::DosBlue);
        assert_eq!(Theme::from_str("dos-blue").unwrap(), Theme::DosBlue);
        assert_eq!(Theme::from_str("amber").unwrap(), Theme::AmberCrt);
        assert_eq!(Theme::from_str("green").unwrap(), Theme::GreenPhosphor);
        assert!(Theme::from_str("invalid").is_err());
    }

    #[test]
    fn test_all_themes_have_colors() {
        // Ensure all themes return valid color schemes
        for theme in &[Theme::Nord, Theme::DosBlue, Theme::AmberCrt, Theme::GreenPhosphor] {
            let colors = theme.colors();
            // Just verify we can access the fields
            let _ = colors.background;
            let _ = colors.focus_border;
        }
    }
}
