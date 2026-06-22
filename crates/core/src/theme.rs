//! Color themes for the terminal UI.
//!
//! Themes are kept UI-agnostic here (plain RGB triples); the frontend maps them
//! to its own color type. Two core themes — a modern `dark` and `light` — plus a
//! couple of popular standards.

/// An RGB color.
pub type Rgb = (u8, u8, u8);

/// The set of colors a frontend needs to render the calculator.
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    /// Console background.
    pub background: Rgb,
    /// Default text.
    pub foreground: Rgb,
    /// The input prompt and echoed input lines.
    pub prompt: Rgb,
    /// A computed result (`ans = …`).
    pub result: Rgb,
    /// Error messages.
    pub error: Rgb,
    /// Secondary / informational text.
    pub info: Rgb,
    /// Window border.
    pub border: Rgb,
    /// Status bar foreground / background.
    pub status_fg: Rgb,
    pub status_bg: Rgb,
}

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub name: &'static str,
    pub dark: bool,
    pub palette: Palette,
}

pub const DEFAULT_THEME: &str = "dark";

// Modern neutral dark (zinc-ish).
const DARK: Theme = Theme {
    name: "dark",
    dark: true,
    palette: Palette {
        background: (24, 24, 27),
        foreground: (212, 212, 216),
        prompt: (96, 165, 250),
        result: (244, 244, 245),
        error: (248, 113, 113),
        info: (113, 113, 122),
        border: (63, 63, 70),
        status_fg: (161, 161, 170),
        status_bg: (39, 39, 42),
    },
};

// Modern neutral light.
const LIGHT: Theme = Theme {
    name: "light",
    dark: false,
    palette: Palette {
        background: (250, 250, 250),
        foreground: (39, 39, 42),
        prompt: (37, 99, 235),
        result: (24, 24, 27),
        error: (220, 38, 38),
        info: (161, 161, 170),
        border: (212, 212, 216),
        status_fg: (82, 82, 91),
        status_bg: (228, 228, 231),
    },
};

// Nord (dark).
const NORD: Theme = Theme {
    name: "nord",
    dark: true,
    palette: Palette {
        background: (46, 52, 64),
        foreground: (216, 222, 233),
        prompt: (136, 192, 208),
        result: (236, 239, 244),
        error: (191, 97, 106),
        info: (110, 120, 140),
        border: (67, 76, 94),
        status_fg: (216, 222, 233),
        status_bg: (59, 66, 82),
    },
};

// Solarized (light).
const SOLARIZED_LIGHT: Theme = Theme {
    name: "solarized-light",
    dark: false,
    palette: Palette {
        background: (253, 246, 227),
        foreground: (101, 123, 131),
        prompt: (38, 139, 210),
        result: (88, 110, 117),
        error: (220, 50, 47),
        info: (147, 161, 161),
        border: (238, 232, 213),
        status_fg: (101, 123, 131),
        status_bg: (238, 232, 213),
    },
};

/// All built-in themes.
pub const THEMES: &[Theme] = &[DARK, LIGHT, NORD, SOLARIZED_LIGHT];

/// Look up a theme by (case-insensitive) name.
pub fn find(name: &str) -> Option<&'static Theme> {
    THEMES.iter().find(|t| t.name.eq_ignore_ascii_case(name))
}

/// The default theme.
pub fn default_theme() -> &'static Theme {
    find(DEFAULT_THEME).unwrap_or(&THEMES[0])
}
