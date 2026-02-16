//! # Theme System
//!
//! Provides a centralized color theme system for the Jarvis TUI.
//!
//! ## Overview
//!
//! The [`Theme`] struct defines all colors used throughout the UI. Instead of
//! hardcoding `ratatui::style::Color` values, rendering code references theme
//! fields. The active theme can be switched at runtime via the theme picker.
//!
//! ## Built-in Themes
//!
//! Jarvis ships with 11 built-in themes:
//!
//! - **Catppuccin Mocha** (default) - warm, dark pastel theme
//! - **Catppuccin Macchiato** - medium-dark pastel theme
//! - **Catppuccin Frappe** - medium pastel theme
//! - **Dracula** - dark theme with vivid colors
//! - **Nord** - arctic, north-bluish color palette
//! - **Tokyo Night** - dark theme inspired by Tokyo city lights
//! - **Solarized Dark** - precision colors for machines and people
//! - **Gruvbox Dark** - retro groove color scheme
//! - **One Dark** - Atom's iconic dark theme
//! - **Monokai** - classic dark theme with vibrant colors
//! - **Rose Pine** - all natural pine, faux fur, and a bit of soho vibes

use ratatui::style::Color;

/// All colors used by the Jarvis TUI, grouped by semantic role.
#[derive(Debug, Clone)]
pub struct Theme {
    /// Human-readable name displayed in the theme picker.
    pub name: &'static str,

    // -- Background colors --
    /// Main background color for panels and modals.
    pub bg: Color,

    // -- Foreground / text colors --
    /// Primary text color (e.g. list items, descriptions).
    pub fg: Color,
    /// Muted/secondary text (e.g. separators, hints, footer).
    pub fg_dim: Color,

    // -- Accent / brand colors --
    /// Primary accent used for branding, focused borders, selected-item bg.
    pub accent: Color,
    /// Secondary accent for highlighted names, search text, running borders.
    pub secondary: Color,

    // -- Semantic status colors --
    /// Success / green indicator.
    pub success: Color,
    /// Error / red indicator.
    pub error: Color,

    // -- Selection --
    /// Background color for mouse-drag text selection in terminal output.
    pub selection_bg: Color,
}

impl Theme {
    /// Return the list of all built-in themes (order = display order in picker).
    pub fn all() -> &'static [Theme] {
        &BUILT_IN_THEMES
    }

    /// Find a built-in theme by name (case-insensitive).
    pub fn by_name(name: &str) -> Option<&'static Theme> {
        BUILT_IN_THEMES
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(name))
    }

    /// Return the default theme (Catppuccin Mocha).
    pub fn default_theme() -> &'static Theme {
        &BUILT_IN_THEMES[0]
    }
}

// ---------------------------------------------------------------------------
// Built-in theme definitions
// ---------------------------------------------------------------------------

static BUILT_IN_THEMES: [Theme; 11] = [
    // 0 - Catppuccin Mocha (default)
    Theme {
        name: "Catppuccin Mocha",
        bg: Color::Rgb(30, 30, 46),           // base
        fg: Color::Rgb(205, 214, 244),        // text
        fg_dim: Color::Rgb(108, 112, 134),    // overlay0
        accent: Color::Rgb(137, 180, 250),    // blue
        secondary: Color::Rgb(249, 226, 175), // yellow
        success: Color::Rgb(166, 227, 161),   // green
        error: Color::Rgb(243, 139, 168),     // red
        selection_bg: Color::Rgb(69, 71, 90), // surface1
    },
    // 1 - Catppuccin Macchiato
    Theme {
        name: "Catppuccin Macchiato",
        bg: Color::Rgb(36, 39, 58),            // base
        fg: Color::Rgb(202, 211, 245),         // text
        fg_dim: Color::Rgb(110, 115, 141),     // overlay0
        accent: Color::Rgb(138, 173, 244),     // blue
        secondary: Color::Rgb(238, 212, 159),  // yellow
        success: Color::Rgb(166, 218, 149),    // green
        error: Color::Rgb(237, 135, 150),      // red
        selection_bg: Color::Rgb(73, 77, 100), // surface1
    },
    // 2 - Catppuccin Frappe
    Theme {
        name: "Catppuccin Frappe",
        bg: Color::Rgb(48, 52, 70),            // base
        fg: Color::Rgb(198, 208, 245),         // text
        fg_dim: Color::Rgb(115, 121, 148),     // overlay0
        accent: Color::Rgb(140, 170, 238),     // blue
        secondary: Color::Rgb(229, 200, 144),  // yellow
        success: Color::Rgb(166, 209, 137),    // green
        error: Color::Rgb(231, 130, 132),      // red
        selection_bg: Color::Rgb(81, 87, 109), // surface1
    },
    // 3 - Dracula
    Theme {
        name: "Dracula",
        bg: Color::Rgb(40, 42, 54),
        fg: Color::Rgb(248, 248, 242),
        fg_dim: Color::Rgb(98, 114, 164),
        accent: Color::Rgb(139, 233, 253),    // cyan
        secondary: Color::Rgb(241, 250, 140), // yellow
        success: Color::Rgb(80, 250, 123),
        error: Color::Rgb(255, 85, 85),
        selection_bg: Color::Rgb(68, 71, 90),
    },
    // 4 - Nord
    Theme {
        name: "Nord",
        bg: Color::Rgb(46, 52, 64),
        fg: Color::Rgb(216, 222, 233),
        fg_dim: Color::Rgb(76, 86, 106),
        accent: Color::Rgb(136, 192, 208),    // frost
        secondary: Color::Rgb(235, 203, 139), // yellow
        success: Color::Rgb(163, 190, 140),
        error: Color::Rgb(191, 97, 106),
        selection_bg: Color::Rgb(67, 76, 94),
    },
    // 5 - Tokyo Night
    Theme {
        name: "Tokyo Night",
        bg: Color::Rgb(26, 27, 38),
        fg: Color::Rgb(169, 177, 214),
        fg_dim: Color::Rgb(86, 95, 137),
        accent: Color::Rgb(122, 162, 247),    // blue
        secondary: Color::Rgb(224, 175, 104), // yellow
        success: Color::Rgb(115, 218, 202),
        error: Color::Rgb(247, 118, 142),
        selection_bg: Color::Rgb(41, 46, 66),
    },
    // 6 - Solarized Dark
    Theme {
        name: "Solarized Dark",
        bg: Color::Rgb(0, 43, 54),
        fg: Color::Rgb(131, 148, 150),
        fg_dim: Color::Rgb(88, 110, 117),
        accent: Color::Rgb(38, 139, 210),   // blue
        secondary: Color::Rgb(181, 137, 0), // yellow
        success: Color::Rgb(133, 153, 0),
        error: Color::Rgb(220, 50, 47),
        selection_bg: Color::Rgb(7, 54, 66),
    },
    // 7 - Gruvbox Dark
    Theme {
        name: "Gruvbox Dark",
        bg: Color::Rgb(40, 40, 40),
        fg: Color::Rgb(235, 219, 178),
        fg_dim: Color::Rgb(146, 131, 116),
        accent: Color::Rgb(131, 165, 152),   // blue
        secondary: Color::Rgb(250, 189, 47), // yellow
        success: Color::Rgb(184, 187, 38),
        error: Color::Rgb(251, 73, 52),
        selection_bg: Color::Rgb(80, 73, 69),
    },
    // 8 - One Dark
    Theme {
        name: "One Dark",
        bg: Color::Rgb(40, 44, 52),
        fg: Color::Rgb(171, 178, 191),
        fg_dim: Color::Rgb(92, 99, 112),
        accent: Color::Rgb(97, 175, 239),     // blue
        secondary: Color::Rgb(229, 192, 123), // yellow
        success: Color::Rgb(152, 195, 121),
        error: Color::Rgb(224, 108, 117),
        selection_bg: Color::Rgb(62, 68, 82),
    },
    // 9 - Monokai
    Theme {
        name: "Monokai",
        bg: Color::Rgb(39, 40, 34),
        fg: Color::Rgb(232, 232, 227),
        fg_dim: Color::Rgb(117, 113, 94),
        accent: Color::Rgb(102, 217, 239),    // cyan
        secondary: Color::Rgb(230, 219, 116), // yellow
        success: Color::Rgb(166, 226, 45),
        error: Color::Rgb(249, 39, 114),
        selection_bg: Color::Rgb(73, 72, 62),
    },
    // 10 - Rose Pine
    Theme {
        name: "Rose Pine",
        bg: Color::Rgb(25, 23, 36),
        fg: Color::Rgb(224, 222, 244),
        fg_dim: Color::Rgb(110, 106, 134),
        accent: Color::Rgb(156, 207, 216),    // foam
        secondary: Color::Rgb(246, 193, 119), // gold
        success: Color::Rgb(156, 207, 216),
        error: Color::Rgb(235, 111, 146),
        selection_bg: Color::Rgb(38, 35, 58),
    },
];

// Verify Catppuccin themes use the actual palette values at compile time.
// This also serves as a usage example for the `ctp` helper.
#[cfg(test)]
mod tests {
    use super::*;

    /// Convert a catppuccin color to a ratatui Color via its RGB values.
    fn ctp(color: catppuccin::Color) -> Color {
        Color::Rgb(color.rgb.r, color.rgb.g, color.rgb.b)
    }

    #[test]
    fn test_all_themes_count() {
        assert_eq!(Theme::all().len(), 11);
    }

    #[test]
    fn test_default_is_mocha() {
        assert_eq!(Theme::default_theme().name, "Catppuccin Mocha");
    }

    #[test]
    fn test_by_name_case_insensitive() {
        assert!(Theme::by_name("catppuccin mocha").is_some());
        assert!(Theme::by_name("CATPPUCCIN MOCHA").is_some());
        assert!(Theme::by_name("dracula").is_some());
        assert!(Theme::by_name("nonexistent").is_none());
    }

    #[test]
    fn test_catppuccin_mocha_matches_palette() {
        let mocha = catppuccin::PALETTE.mocha.colors;
        let theme = Theme::default_theme();
        assert_eq!(theme.bg, ctp(mocha.base));
        assert_eq!(theme.fg, ctp(mocha.text));
        assert_eq!(theme.accent, ctp(mocha.blue));
        assert_eq!(theme.secondary, ctp(mocha.yellow));
        assert_eq!(theme.success, ctp(mocha.green));
        assert_eq!(theme.error, ctp(mocha.red));
    }

    #[test]
    fn test_catppuccin_macchiato_matches_palette() {
        let macchiato = catppuccin::PALETTE.macchiato.colors;
        let theme = Theme::by_name("Catppuccin Macchiato").expect("theme exists");
        assert_eq!(theme.bg, ctp(macchiato.base));
        assert_eq!(theme.fg, ctp(macchiato.text));
        assert_eq!(theme.accent, ctp(macchiato.blue));
    }

    #[test]
    fn test_catppuccin_frappe_matches_palette() {
        let frappe = catppuccin::PALETTE.frappe.colors;
        let theme = Theme::by_name("Catppuccin Frappe").expect("theme exists");
        assert_eq!(theme.bg, ctp(frappe.base));
        assert_eq!(theme.fg, ctp(frappe.text));
        assert_eq!(theme.accent, ctp(frappe.blue));
    }

    #[test]
    fn test_all_themes_have_distinct_names() {
        let names: Vec<&str> = Theme::all().iter().map(|t| t.name).collect();
        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(names.len(), unique.len(), "duplicate theme names found");
    }
}
