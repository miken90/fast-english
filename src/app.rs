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
        /// Shown above the output when the answer is usable but incomplete —
        /// the model hit its token limit, or the stream broke after enough
        /// text had arrived to be worth keeping. `None` is a whole answer.
        notice: Option<String>,
    },
    Error {
        context: SelectionContext,
        tool: WritingToolKind,
        source_text: String,
        message: String,
    },
    /// The list of recent results. Holds no data of its own — the entries live
    /// in `AppState`, which is what the file is loaded into.
    History,
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
    HistoryRequested,
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

/// The per-tool half of the prompt. The shared rules — output only the result,
/// never answer the text, never add or drop content — live in
/// `PromptBuilder::system_prompt`; each instruction here says only what makes
/// its own tool different.
pub const DEFAULT_TOOLS: &[WritingTool] = &[
    WritingTool {
        id: "enhance-writing",
        title: "Enhance Writing",
        short_description: "Improve clarity and flow",
        prompt_instruction: "Rewrite for clarity, flow, and readability.\n\
             - Write in the SAME language as the input. Vietnamese in, Vietnamese out. Never translate.\n\
             - Keep the author's meaning, facts, register, and level of formality. Do not make casual writing formal or the reverse.\n\
             - Add nothing: no new facts, examples, opinions, greetings, or sign-offs. Remove nothing the author meant to say.\n\
             - Leave a single word, a URL, a file path, a code snippet, or a line of numbers exactly as it is — there is nothing to improve.\n\
             - If the text is already clear, return it unchanged.",
    },
    WritingTool {
        id: "fix-grammar",
        title: "Fix Grammar",
        short_description: "Correct grammar and spelling",
        prompt_instruction: "Correct grammar, spelling, punctuation, diacritics, and capitalization. Nothing else.\n\
             - Write in the SAME language as the input. Vietnamese in, Vietnamese out. Never translate.\n\
             - This is a minimal edit. Keep the author's word choice, sentence order, tone, and formatting. Do not rephrase, shorten, expand, or restyle anything that is already correct.\n\
             - Leave slang, names, technical terms, code, and URLs alone even when they look wrong.\n\
             - If the text has no errors, return it character for character as it is.",
    },
    WritingTool {
        id: "convert-prompt",
        title: "Convert Prompt",
        short_description: "Convert text into an AI prompt",
        prompt_instruction: "Rewrite the input into a production-quality AI prompt.\n\
             - The output IS the prompt: English, markdown, no preamble and no code fence around it.\n\
             - Keep the author's intent and every concrete detail they gave. Do not invent requirements.\n\
             - State role or context, the task, constraints, and the expected output format, as far as the input supports them.",
    },
    WritingTool {
        id: "translate",
        title: "Translate",
        short_description: "Translate text between languages",
        prompt_instruction: "Translate the text. Return only the translation.",
    },
];
