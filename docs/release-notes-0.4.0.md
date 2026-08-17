# Fast English 0.4.0

## Highlights
- Drag-resize the popup window and its content reflows with it.
- Long results (and Vietnamese translations) no longer get cut off mid-sentence.
- The app's .exe now shows proper version info in Windows Explorer, and the version lives in one place only.

## What changed
- Resize the popup by dragging its frame; content reflows to fit. The size you pick is kept for that popup session and resets to automatic sizing next time the popup opens.
- Fix result truncation: text containing phrases like "I've attached the report", a "Note:" line, a `---` rule, or a code fence used to silently cut off everything after it. Translations also get a larger token budget, so long Vietnamese output no longer runs out mid-sentence.
- The built .exe now carries proper version information (visible in Explorer's Details tab), and the release packaging script can no longer report success when packaging actually failed. Version comes from `Cargo.toml` only — nothing else to keep in sync.
- Minor documentation corrections.

## Also included (from 0.3.2 and 0.3.3)
Versions 0.3.2 and 0.3.3 shipped and were tagged. Since this is the first release note since 0.3.0, their user-visible changes are summarised here too:
- The popup now opens on the monitor holding your text selection, and its width adapts on small displays.
- Dragging the popup tracks the OS cursor more accurately.
- The popup uses the system font stack instead of a downloaded web font.
- Hardened manual hotkey capture to prevent race conditions.

## Known limitations
- Enhance Writing and Fix Grammar still budget output tokens using an English-length estimate, so very long Vietnamese input to those two tools can still truncate.
- No streaming responses yet.
- The Inno Setup installer has never been compiled; the portable zip is the shipping format.

## Notes
- This release targets Windows.
- Portable artifact included in this release: `FastEnglish-0.4.0-portable.zip`
