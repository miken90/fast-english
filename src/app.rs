use crate::selection::SelectionContext;

/// Top-level popup state machine.
#[derive(Debug, Clone)]
pub enum PopupState {
    Hidden,
    Editor {
        context: SelectionContext,
        tool: WritingToolKind,
        source_text: String,
    },
    Processing {
        context: SelectionContext,
        tool: WritingToolKind,
        source_text: String,
    },
    Result {
        context: SelectionContext,
        tool: WritingToolKind,
        source_text: String,
        output: String,
        can_replace: bool,
    },
    Error {
        context: SelectionContext,
        tool: WritingToolKind,
        source_text: String,
        message: String,
    },
}

/// Events flowing through the cross-component channel.
///
/// These are events sent from background threads (tray, hotkey, selection monitor)
/// into the popup runtime via a tokio::sync::mpsc channel, which drives the
/// WebView2 popup/settings windows (wry + a single shared tao EventLoop).
#[derive(Debug, Clone)]
pub enum AppEvent {
    // Selection
    TextSelected(SelectionContext),
    SelectionCleared,

    // System
    HotkeyPressed,
    OpenPopupRequested,
    SettingsRequested,
    QuitRequested,
}

/// Writing tool variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WritingToolKind {
    EnhanceWriting,
    FixGrammar,
    ConvertPrompt,
    Translate {
        from: TranslateLanguage,
        to: TranslateLanguage,
    },
}

/// Language option for translation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranslateLanguage {
    pub id: &'static str,
    pub name: &'static str,
}

impl std::fmt::Display for TranslateLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

pub const TRANSLATE_LANGS: &[TranslateLanguage] = &[
    TranslateLanguage {
        id: "auto",
        name: "Auto Detect",
    },
    TranslateLanguage {
        id: "en",
        name: "English",
    },
    TranslateLanguage {
        id: "vi",
        name: "Vietnamese",
    },
    TranslateLanguage {
        id: "es",
        name: "Spanish",
    },
    TranslateLanguage {
        id: "fr",
        name: "French",
    },
    TranslateLanguage {
        id: "de",
        name: "German",
    },
    TranslateLanguage {
        id: "it",
        name: "Italian",
    },
    TranslateLanguage {
        id: "pt",
        name: "Portuguese",
    },
    TranslateLanguage {
        id: "ru",
        name: "Russian",
    },
    TranslateLanguage {
        id: "ja",
        name: "Japanese",
    },
    TranslateLanguage {
        id: "ko",
        name: "Korean",
    },
    TranslateLanguage {
        id: "zh",
        name: "Chinese (Simplified)",
    },
    TranslateLanguage {
        id: "zh-TW",
        name: "Chinese (Traditional)",
    },
    TranslateLanguage {
        id: "th",
        name: "Thai",
    },
    TranslateLanguage {
        id: "id",
        name: "Indonesian",
    },
];

impl std::fmt::Display for WritingToolKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EnhanceWriting => write!(f, "Enhance Writing"),
            Self::FixGrammar => write!(f, "Fix Grammar"),
            Self::ConvertPrompt => write!(f, "Convert Prompt"),

            Self::Translate { from, to } => write!(f, "Translate {} → {}", from.name, to.name),
        }
    }
}

/// Default writing tools.
pub struct WritingTool {
    pub id: &'static str,
    pub title: &'static str,
    #[allow(dead_code)] // reserved for future tooltips
    pub short_description: &'static str,
    pub prompt_instruction: &'static str,
}

pub const DEFAULT_TOOLS: &[WritingTool] = &[
    WritingTool {
        id: "enhance-writing",
        title: "Enhance Writing",
        short_description: "Improve clarity and flow",
        prompt_instruction: "Improve clarity, flow, and readability. Preserve meaning and language. Return only the improved text.",
    },
    WritingTool {
        id: "fix-grammar",
        title: "Fix Grammar",
        short_description: "Correct grammar and spelling",
        prompt_instruction: "Fix grammar, spelling, and punctuation. Return only the corrected text.",
    },
    WritingTool {
        id: "convert-prompt",
        title: "Convert Prompt",
        short_description: "Convert text into an AI prompt",
        prompt_instruction: "Convert into a clear, structured AI prompt. Preserve intent, add context, constraints, and output format. Return only the final prompt in markdown.",
    },
    WritingTool {
        id: "translate",
        title: "Translate",
        short_description: "Translate text between languages",
        prompt_instruction: "Translate text. Return only the translation.",
    },
];
