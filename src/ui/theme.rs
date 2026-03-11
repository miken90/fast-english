use iced::Color;

// Primary
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

// Surface
pub const SURFACE: Color = Color {
    r: 0.118,
    g: 0.118,
    b: 0.180,
    a: 1.0,
};
pub const SURFACE_ELEVATED: Color = Color {
    r: 0.145,
    g: 0.145,
    b: 0.212,
    a: 1.0,
};
pub const SURFACE_BORDER: Color = Color {
    r: 0.227,
    g: 0.227,
    b: 0.314,
    a: 0.6,
};

// Text
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

// Semantic
pub const SUCCESS: Color = Color {
    r: 0.133,
    g: 0.773,
    b: 0.369,
    a: 1.0,
};
pub const ERROR: Color = Color {
    r: 0.937,
    g: 0.267,
    b: 0.267,
    a: 1.0,
};

// Spacing
pub const RADIUS_MD: f32 = 10.0;
pub const RADIUS_LG: f32 = 14.0;

// Spinner frames
pub const SPINNER_FRAMES: &[&str] = &["|", "/", "-", "\\"];
