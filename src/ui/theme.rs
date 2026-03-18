use iced::Color;

// ─── Brand / Primary ────────────────────────────────────────────────────────

pub const PRIMARY: Color = Color {
    r: 0.231,
    g: 0.510,
    b: 0.965,
    a: 1.0,
};
#[allow(dead_code)]
pub const PRIMARY_DARK: Color = Color {
    r: 0.114,
    g: 0.306,
    b: 0.847,
    a: 1.0,
};
pub const PRIMARY_LIGHT: Color = Color {
    r: 0.376,
    g: 0.647,
    b: 0.980,
    a: 1.0,
};
/// Subtle primary tint — 12 % opacity fill for active/hover states
#[allow(dead_code)]
pub const PRIMARY_TINT: Color = Color {
    r: 0.231,
    g: 0.510,
    b: 0.965,
    a: 0.12,
};
/// Stronger primary tint — 20 % opacity fill for pressed states
#[allow(dead_code)]
pub const PRIMARY_TINT_STRONG: Color = Color {
    r: 0.231,
    g: 0.510,
    b: 0.965,
    a: 0.20,
};

// ─── Surface ────────────────────────────────────────────────────────────────

/// Base popup background
pub const SURFACE: Color = Color {
    r: 0.118,
    g: 0.118,
    b: 0.180,
    a: 1.0,
};
/// Slightly lifted layer (inputs, cards)
pub const SURFACE_ELEVATED: Color = Color {
    r: 0.145,
    g: 0.145,
    b: 0.212,
    a: 1.0,
};
/// Highest layer — tooltips, dropdowns
#[allow(dead_code)]
pub const SURFACE_OVERLAY: Color = Color {
    r: 0.173,
    g: 0.173,
    b: 0.247,
    a: 1.0,
};
/// Divider / border
pub const SURFACE_BORDER: Color = Color {
    r: 0.227,
    g: 0.227,
    b: 0.314,
    a: 0.6,
};
/// Subtle hairline — less prominent separators
#[allow(dead_code)]
pub const SURFACE_DIVIDER: Color = Color {
    r: 0.227,
    g: 0.227,
    b: 0.314,
    a: 0.3,
};

// ─── Text ───────────────────────────────────────────────────────────────────

pub const TEXT_PRIMARY: Color = Color {
    r: 0.941,
    g: 0.941,
    b: 0.941,
    a: 1.0,
};
pub const TEXT_SECONDARY: Color = Color {
    r: 0.627,
    g: 0.627,
    b: 0.690,
    a: 1.0,
};
pub const TEXT_MUTED: Color = Color {
    r: 0.420,
    g: 0.420,
    b: 0.502,
    a: 1.0,
};
/// Placeholder or disabled text
#[allow(dead_code)]
pub const TEXT_DISABLED: Color = Color {
    r: 0.300,
    g: 0.300,
    b: 0.380,
    a: 1.0,
};

// ─── Semantic ───────────────────────────────────────────────────────────────

pub const SUCCESS: Color = Color {
    r: 0.133,
    g: 0.773,
    b: 0.369,
    a: 1.0,
};
#[allow(dead_code)]
pub const SUCCESS_MUTED: Color = Color {
    r: 0.133,
    g: 0.773,
    b: 0.369,
    a: 0.15,
};
pub const ERROR: Color = Color {
    r: 0.937,
    g: 0.267,
    b: 0.267,
    a: 1.0,
};
#[allow(dead_code)]
pub const ERROR_MUTED: Color = Color {
    r: 0.937,
    g: 0.267,
    b: 0.267,
    a: 0.15,
};
#[allow(dead_code)]
pub const WARNING: Color = Color {
    r: 0.984,
    g: 0.737,
    b: 0.180,
    a: 1.0,
};
#[allow(dead_code)]
pub const WARNING_MUTED: Color = Color {
    r: 0.984,
    g: 0.737,
    b: 0.180,
    a: 0.15,
};

// ─── Spacing ────────────────────────────────────────────────────────────────

#[allow(dead_code)]
pub const SPACE_XS: f32 = 4.0;
#[allow(dead_code)]
pub const SPACE_SM: f32 = 8.0;
#[allow(dead_code)]
pub const SPACE_MD: f32 = 12.0;
#[allow(dead_code)]
pub const SPACE_LG: f32 = 16.0;
#[allow(dead_code)]
pub const SPACE_XL: f32 = 24.0;
#[allow(dead_code)]
pub const SPACE_2XL: f32 = 32.0;

// ─── Border radius ──────────────────────────────────────────────────────────

#[allow(dead_code)]
pub const RADIUS_SM: f32 = 6.0;
pub const RADIUS_MD: f32 = 10.0;
#[allow(dead_code)]
pub const RADIUS_LG: f32 = 14.0;
#[allow(dead_code)]
pub const RADIUS_XL: f32 = 20.0;
#[allow(dead_code)]
pub const RADIUS_PILL: f32 = 999.0;

// ─── Typography — font families ─────────────────────────────────────────────

/// Display / heading font (load via iced font bytes or system font name)
#[allow(dead_code)]
pub const FONT_DISPLAY: &str = "Plus Jakarta Sans";
/// Body / UI font
#[allow(dead_code)]
pub const FONT_BODY: &str = "Inter";
/// Monospace — code snippets, API key fields
#[allow(dead_code)]
pub const FONT_MONO: &str = "JetBrains Mono";

// ─── Typography — sizes (logical pixels) ────────────────────────────────────

#[allow(dead_code)]
pub const TEXT_XS: f32 = 10.0;
#[allow(dead_code)]
pub const TEXT_SM: f32 = 12.0;
#[allow(dead_code)]
pub const TEXT_BASE: f32 = 14.0;
#[allow(dead_code)]
pub const TEXT_MD: f32 = 16.0;
#[allow(dead_code)]
pub const TEXT_LG: f32 = 18.0;
#[allow(dead_code)]
pub const TEXT_XL: f32 = 22.0;

// ─── Typography — line heights (multiplier) ──────────────────────────────────

#[allow(dead_code)]
pub const LINE_HEIGHT_TIGHT: f32 = 1.2;
#[allow(dead_code)]
pub const LINE_HEIGHT_NORMAL: f32 = 1.5;
#[allow(dead_code)]
pub const LINE_HEIGHT_RELAXED: f32 = 1.7;

// ─── Opacity helpers ────────────────────────────────────────────────────────

#[allow(dead_code)]
pub const OPACITY_DISABLED: f32 = 0.38;
#[allow(dead_code)]
pub const OPACITY_HOVER: f32 = 0.08;

// ─── Animation / spinner ────────────────────────────────────────────────────

/// Smooth braille-dot spinner (unicode, 8 frames)
#[allow(dead_code)]
pub const SPINNER_FRAMES: &[&str] = &["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"];
/// Fallback ASCII spinner — use when font rendering is uncertain
#[allow(dead_code)]
pub const SPINNER_FRAMES_ASCII: &[&str] = &["|", "/", "-", "\\"];
