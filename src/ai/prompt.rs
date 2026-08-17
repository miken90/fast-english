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

    /// The 4-chars-per-token estimate is an English one. Vietnamese and other
    /// non-Latin-script targets tokenize roughly 1.5-2x worse, so a translation
    /// budgeted like English runs out mid-sentence.
    pub fn max_tokens_for_translate(&self, input: &str) -> usize {
        let estimated_tokens = input.len() / 4;
        let budget = (estimated_tokens * 3).max(200);
        budget.min(4096)
    }

    /// Strip AI commentary appended after the actual result.
    ///
    /// A marker only ends the result when it opens a line *and* that line
    /// starts a new paragraph. Matching anywhere in a line truncated real
    /// prose — "I've attached the report" ended the output at that sentence.
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
        // The result itself may open with any marker, so the first line of
        // content can never be a cut point.
        let mut seen_content = false;
        // A result that is already a bulleted list keeps its own bullets: only
        // a list that appears out of nowhere reads as an appended summary.
        let mut seen_bullet = false;
        let mut previous_is_blank = false;

        for line in strip_wrapping_fence(text) {
            let trimmed = line.trim();
            let lower = trimmed.to_lowercase();
            let is_blank = trimmed.is_empty();

            if seen_content && previous_is_blank {
                let opens_commentary = cut_patterns.iter().any(|p| lower.starts_with(p));
                let opens_bullet_appendix = !seen_bullet && lower.starts_with("- ");
                if opens_commentary || opens_bullet_appendix {
                    break;
                }
            }

            if !is_blank {
                seen_content = true;
                seen_bullet |= lower.starts_with("- ");
            }
            previous_is_blank = is_blank;
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

/// Unwrap a result the model put inside a single code fence.
///
/// Only a fence that opens the first line and closes the last one is a
/// wrapper. An unbalanced fence is left alone: it is more likely to belong to
/// the result than to be a wrapper someone forgot to close.
fn strip_wrapping_fence(text: &str) -> Vec<&str> {
    let lines: Vec<&str> = text.lines().collect();
    if lines.len() < 2 {
        return lines;
    }
    let opens = lines[0].trim().starts_with("```");
    let closes = lines[lines.len() - 1].trim() == "```";
    if opens && closes {
        return lines[1..lines.len() - 1].to_vec();
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prose_containing_a_marker_is_not_truncated() {
        let raw = "The deadline moved to Friday.\nI've attached the report for review.\nLet me know if anything looks off.";
        assert_eq!(PromptBuilder::clean_output(raw), raw);
    }

    #[test]
    fn a_marker_mid_paragraph_is_not_a_cut_point() {
        let raw = "Please read the brief.\nNote: the second section is still a draft.\nThanks.";
        assert_eq!(PromptBuilder::clean_output(raw), raw);
    }

    #[test]
    fn a_table_separator_row_survives() {
        let raw = "| Term | Meaning |\n|---|---|\n| SLA | Service level agreement |";
        assert_eq!(PromptBuilder::clean_output(raw), raw);
    }

    #[test]
    fn a_result_opening_with_a_marker_survives() {
        let raw = "Note: bring your badge to the front desk.";
        assert_eq!(PromptBuilder::clean_output(raw), raw);
    }

    #[test]
    fn trailing_commentary_after_a_blank_line_is_cut() {
        let raw = "The deadline moved to Friday.\n\n**Changes:**\n- Fixed the tense\n- Shortened the second sentence";
        assert_eq!(
            PromptBuilder::clean_output(raw),
            "The deadline moved to Friday."
        );
    }

    #[test]
    fn a_separator_before_commentary_is_cut() {
        let raw = "The deadline moved to Friday.\n\n---\n\nI fixed the tense throughout.";
        assert_eq!(
            PromptBuilder::clean_output(raw),
            "The deadline moved to Friday."
        );
    }

    #[test]
    fn a_bulleted_result_survives_its_own_blank_lines() {
        let raw = "- Draft the agenda\n\n- Book the room\n\n- Send the invites";
        assert_eq!(PromptBuilder::clean_output(raw), raw);
    }

    #[test]
    fn a_bullet_appendix_after_prose_is_cut() {
        let raw = "The deadline moved to Friday.\n\n- Fixed the tense\n- Shortened the sentence";
        assert_eq!(
            PromptBuilder::clean_output(raw),
            "The deadline moved to Friday."
        );
    }

    #[test]
    fn a_wrapping_code_fence_is_stripped() {
        let raw = "```\nThe deadline moved to Friday.\n```";
        assert_eq!(
            PromptBuilder::clean_output(raw),
            "The deadline moved to Friday."
        );
    }

    #[test]
    fn a_fenced_commentary_block_after_the_result_is_cut() {
        let raw = "The deadline moved to Friday.\n\n```diff\n- Friday\n+ Friday morning\n```";
        assert_eq!(
            PromptBuilder::clean_output(raw),
            "The deadline moved to Friday."
        );
    }

    #[test]
    fn trailing_blank_lines_are_trimmed() {
        assert_eq!(PromptBuilder::clean_output("Done.\n\n\n"), "Done.");
    }

    #[test]
    fn empty_input_stays_empty() {
        assert_eq!(PromptBuilder::clean_output("   \n  "), "");
    }

    #[test]
    fn translation_gets_a_larger_budget_than_the_english_estimate() {
        let builder = PromptBuilder::new();
        let input = "x".repeat(4000); // 1000 estimated tokens

        assert_eq!(builder.max_tokens(&input), 2000);
        assert_eq!(builder.max_tokens_for_translate(&input), 3000);
        // The convert path keeps its own budget and its own floor.
        assert_eq!(builder.max_tokens_for_convert(&input), 3000);
        assert_eq!(builder.max_tokens_for_convert(""), 400);
    }

    #[test]
    fn the_translate_budget_keeps_the_floor_and_the_cap() {
        let builder = PromptBuilder::new();
        assert_eq!(builder.max_tokens_for_translate(""), 200);
        assert_eq!(builder.max_tokens_for_translate(&"x".repeat(100_000)), 4096);
    }
}
