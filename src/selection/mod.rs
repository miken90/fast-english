pub mod monitor;

pub(crate) mod clipboard_fallback;
pub(crate) mod uia_text;
pub(crate) mod uia_value;

/// Represents detected text selection with positioning info.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectionContext {
    pub text: String,
    pub bounds: Option<Rect>,
    pub source_app: String,
    pub detection_tier: Tier,
    pub is_editable: bool,
}

/// Which detection method found the selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    TextPattern,
    ValuePattern,
    ClipboardFallback,
}

impl std::fmt::Display for Tier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TextPattern => write!(f, "UIA TextPattern"),
            Self::ValuePattern => write!(f, "UIA ValuePattern"),
            Self::ClipboardFallback => write!(f, "Clipboard"),
        }
    }
}

/// Screen rectangle for popup positioning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn from_point(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self { x, y, width, height }
    }
}
