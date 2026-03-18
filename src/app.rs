use crate::selection::SelectionContext;

/// Top-level popup state machine.
#[derive(Debug, Clone)]
pub enum PopupState {
    Hidden,
    Options(SelectionContext),
    ToneSelection(SelectionContext),
    TranslateSelection(SelectionContext),
    PromptEditor(SelectionContext),
    TranslateResult {
        context: SelectionContext,
    },
    Processing {
        context: SelectionContext,
        tool: WritingToolKind,
    },
    Result {
        context: SelectionContext,
        tool: WritingToolKind,
        output: String,
    },
    Error {
        context: SelectionContext,
        message: String,
    },
}

/// Events flowing through the cross-component channel.
///
/// These are events sent from background threads (tray, hotkey, selection monitor)
/// into the iced daemon via tokio::mpsc. Internal UI events use iced `Message` directly.
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
    ChangeTone(ToneOption),
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
            Self::ChangeTone(tone) => write!(f, "Change Tone → {}", tone.name),
            Self::Translate { from, to } => write!(f, "Translate {} → {}", from.name, to.name),
        }
    }
}

/// Tone options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToneOption {
    pub id: &'static str,
    pub name: &'static str,
    pub prompt_instruction: &'static str,
}

pub const TONES: &[ToneOption] = &[
    ToneOption {
        id: "professional",
        name: "Professional",
        prompt_instruction: "Rewrite in a professional and polished tone. Preserve the original meaning and language. Return only the rewritten text.",
    },
    ToneOption {
        id: "casual",
        name: "Casual",
        prompt_instruction: "Rewrite in a casual and conversational tone. Preserve the original meaning and language. Return only the rewritten text.",
    },
    ToneOption {
        id: "formal",
        name: "Formal",
        prompt_instruction: "Rewrite in a formal and official tone. Preserve the original meaning and language. Return only the rewritten text.",
    },
    ToneOption {
        id: "friendly",
        name: "Friendly",
        prompt_instruction: "Rewrite in a warm and friendly tone. Preserve the original meaning and language. Return only the rewritten text.",
    },
    ToneOption {
        id: "concise",
        name: "Concise",
        prompt_instruction: "Rewrite in a concise and brief style, removing unnecessary words. Preserve the original meaning and language. Return only the rewritten text.",
    },
    ToneOption {
        id: "confident",
        name: "Confident",
        prompt_instruction: "Rewrite in a confident and assertive tone. Preserve the original meaning and language. Return only the rewritten text.",
    },
];

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
        prompt_instruction: "Improve the writing quality, clarity, flow, and readability. Preserve the original meaning and language. Return only the improved text.",
    },
    WritingTool {
        id: "change-tone",
        title: "Change Tone",
        short_description: "Make the writing more professional",
        prompt_instruction: "Rewrite in a more professional and polished tone. Preserve the original meaning. Return only the rewritten text.",
    },
    WritingTool {
        id: "fix-grammar",
        title: "Fix Grammar",
        short_description: "Correct grammar and spelling",
        prompt_instruction: "Correct grammar, spelling, and punctuation. Do not explain changes. Return only the corrected text.",
    },
    WritingTool {
        id: "convert-prompt",
        title: "Convert Prompt",
        short_description: "Convert text into an AI prompt",
        prompt_instruction: "Turn this text into a clear, structured prompt that can be used with an AI assistant. Preserve intent, add useful context, constraints, and output format where appropriate. Return only the final prompt in clean markdown format.",
    },
    WritingTool {
        id: "translate",
        title: "Translate",
        short_description: "Translate text between languages",
        prompt_instruction: "Translate the text. Return only the translated text.",
    },
];
