use ratatui::style::Color;

/// Catppuccin Mocha color palette
mod palette {
    use super::Color;

    pub const ROSEWATER: Color = Color::Rgb(245, 224, 220);
    pub const MAUVE: Color = Color::Rgb(203, 166, 247);
    pub const RED: Color = Color::Rgb(243, 139, 168);
    pub const PEACH: Color = Color::Rgb(250, 179, 135);
    pub const YELLOW: Color = Color::Rgb(249, 226, 175);
    pub const GREEN: Color = Color::Rgb(166, 227, 161);
    pub const SKY: Color = Color::Rgb(137, 220, 235);
    pub const BLUE: Color = Color::Rgb(137, 180, 250);
    pub const LAVENDER: Color = Color::Rgb(180, 190, 254);

    pub const TEXT: Color = Color::Rgb(205, 214, 244);
    pub const SUBTEXT0: Color = Color::Rgb(166, 173, 200);
    pub const OVERLAY1: Color = Color::Rgb(127, 132, 156);
    pub const SURFACE1: Color = Color::Rgb(69, 71, 90);
    pub const SURFACE0: Color = Color::Rgb(49, 50, 68);
    pub const MANTLE: Color = Color::Rgb(24, 24, 37);
}

// Semantic color mappings for the UI

/// Background colors
pub const BG_DARK: Color = palette::MANTLE;
pub const BG_HIGHLIGHT: Color = palette::SURFACE0;

/// Text colors
pub const TEXT: Color = palette::TEXT;
pub const TEXT_MUTED: Color = palette::SUBTEXT0;
pub const TEXT_DIM: Color = palette::OVERLAY1;

/// UI element colors
pub const BORDER: Color = palette::SURFACE1;

/// Status colors
pub const SUCCESS: Color = palette::GREEN;
pub const WARNING: Color = palette::YELLOW;
pub const ERROR: Color = palette::RED;
pub const INFO: Color = palette::BLUE;

/// Accent colors
pub const ACCENT: Color = palette::MAUVE;
pub const SELECTED: Color = palette::GREEN;
pub const CURSOR: Color = palette::ROSEWATER;

/// Type badge colors
pub const TYPE_GIT: Color = palette::PEACH;
pub const TYPE_PATH: Color = palette::SKY;
pub const TYPE_OTHER: Color = palette::OVERLAY1;

/// Misc
pub const KEY_HINT: Color = palette::LAVENDER;
pub const SHA: Color = palette::PEACH;
