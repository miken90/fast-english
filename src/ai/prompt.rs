/// Prompt builder for constructing AI request prompts.
pub struct PromptBuilder;

impl PromptBuilder {
    pub fn new() -> Self {
        Self
    }

    pub fn system_prompt(&self) -> &'static str {
        "Text editor. Output ONLY the edited text. No explanations, notes, commentary, \
         headers, bullets, or markdown formatting (**, ```). Any extra content = failure."
    }

    pub fn convert_prompt_system_prompt(&self) -> &'static str {
        "Prompt engineer. Rewrite user input into a production-quality AI prompt. Rules:\n\
         1. Output ONLY the final prompt — no preamble, commentary, or postscript.\n\
         2. Clear English only, regardless of input language.\n\
         3. Use markdown structure: headings, bullets, code blocks.\n\
         4. Concise — remove filler, use direct instructions.\n\
         5. Include role/context, task, constraints, output format as needed.\n\
         6. Never wrap in code fences or quote blocks — output IS the prompt."
    }

    pub fn user_prompt(&self, input: &str, instruction: &str) -> String {
        format!("[{}]\n\n{}", instruction, input)
    }

    pub fn max_tokens(&self, input: &str) -> usize {
        let estimated_tokens = input.len() / 4; // ~4 chars per token for English
        let budget = (estimated_tokens * 2).max(200);
        budget.min(4096)
    }

    /// Convert prompt output is often longer than input — give more room.
    pub fn max_tokens_for_convert(&self, input: &str) -> usize {
        let estimated_tokens = input.len() / 4;
        let budget = (estimated_tokens * 3).max(400);
        budget.min(4096)
    }

    /// Strip AI commentary appended after the actual result.
    pub fn clean_output(raw: &str) -> String {
        let text = raw.trim();
        if text.is_empty() {
            return String::new();
        }

        let cut_patterns: &[&str] = &[
            "**changes",
            "**change",
            "**edit",
            "**note",
            "**correction",
            "changes made",
            "changes:",
            "edits made",
            "edits:",
            "here's what",
            "here is what",
            "i made the following",
            "i changed",
            "i corrected",
            "i fixed",
            "i improved",
            "i rewrote",
            "i've ",
            "note:",
            "notes:",
            "explanation:",
            "summary of changes",
            "---",
            "```",
        ];

        let mut clean_lines: Vec<&str> = Vec::new();

        for line in text.lines() {
            let lower = line.trim().to_lowercase();

            let should_cut = cut_patterns
                .iter()
                .any(|p| lower.starts_with(p) || lower.contains(p));

            let is_bullet_after_blank = lower.starts_with("- ")
                && !clean_lines.is_empty()
                && clean_lines.last().is_some_and(|l| l.trim().is_empty());

            if should_cut || is_bullet_after_blank {
                break;
            }

            clean_lines.push(line);
        }

        while clean_lines.last().is_some_and(|l| l.trim().is_empty()) {
            clean_lines.pop();
        }

        clean_lines.join("\n").trim().to_string()
    }

    pub fn clean_convert_prompt_output(raw: &str) -> String {
        let cleaned = raw.trim();
        if cleaned.is_empty() {
            return String::new();
        }

        let mut lines: Vec<&str> = cleaned.lines().collect();

        while lines.first().is_some_and(|line| {
            let l = line.trim().to_lowercase();
            l.starts_with("here") || l.starts_with("sure") || l.starts_with("i rewrote")
        }) {
            lines.remove(0);
        }

        while lines.last().is_some_and(|line| {
            let l = line.trim().to_lowercase();
            l.starts_with("note:") || l.starts_with("explanation:")
        }) {
            lines.pop();
        }

        lines.join("\n").trim().to_string()
    }
}
