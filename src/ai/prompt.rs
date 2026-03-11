/// Prompt builder for constructing AI request prompts.
pub struct PromptBuilder;

impl PromptBuilder {
    pub fn new() -> Self {
        Self
    }

    pub fn system_prompt(&self) -> &'static str {
        "You are a text editor. Rules: reply with ONLY the edited text. \
         NEVER explain your changes. NEVER add notes, bullet points, headers, \
         or commentary after the text. NEVER use markdown formatting like ** or ```. \
         If you add anything beyond the edited text, you have failed."
    }

    pub fn user_prompt(&self, input: &str, instruction: &str) -> String {
        format!("[{}]\n\n{}", instruction, input)
    }

    pub fn max_tokens(&self, input: &str) -> usize {
        let estimated_tokens = input.len() / 4; // ~4 chars per token for English
        let budget = (estimated_tokens * 2).max(200);
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
                && clean_lines.last().map_or(false, |l| l.trim().is_empty());

            if should_cut || is_bullet_after_blank {
                break;
            }

            clean_lines.push(line);
        }

        // Trim trailing empty lines
        while clean_lines.last().map_or(false, |l| l.trim().is_empty()) {
            clean_lines.pop();
        }

        clean_lines.join("\n").trim().to_string()
    }
}
