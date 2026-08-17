# Fast English 0.5.0

## Highlights
- Answers now appear as they are written, instead of after a spinner.
- Fix Grammar shows you exactly what it changed, word by word.
- The last 20 results are kept, so a result is no longer lost when the popup closes.

## What changed

### Streaming answers
Every tool now shows its answer as the model produces it. The first words
appear in about a second and the text grows as it arrives, rather than the
popup sitting on a spinner until the whole answer is ready. Cancel stops the
request immediately, and a stream that breaks part-way tells you it was cut off
instead of leaving half a sentence looking like a finished result.

### See what changed (diff view)
Fix Grammar now opens showing the corrections themselves: removed words struck
through in red, added words highlighted in green, everything else untouched. If
you would rather read the clean result, the Diff button turns the marks off.

Enhance Writing has the same button, off by default — it rewrites for flow, so
its diff is often a near-total rewrite that reads as noise rather than as a
lesson. Translate and Convert Prompt do not offer a diff, because comparing two
different languages, or a prompt against the text it came from, tells you
nothing.

Copy and Replace always give you the clean text. The marks are for reading, and
never end up in what you paste.

### Recent Results
The last 20 results are now kept. Reach them from the tray icon under **Recent
Results**, or from the clock button in the popup header. Opening one brings it
back into the normal result view, with Copy, Edit output and the diff all
working as they did the first time.

Your text is stored encrypted with Windows DPAPI — the same protection the app
already uses for your API key — so your writing is not sitting in a plain file
for anything on the machine to read. It is readable only by your own Windows
account. There is a **Clear history** button, and each entry keeps at most 8000
characters of the source and of the result.

### Long Vietnamese input no longer gets cut off
Enhance Writing and Fix Grammar used to estimate length with an
English-shaped rule, which quietly left long Vietnamese input without enough
room and cut the answer off mid-sentence. The estimate now accounts for how
non-English text actually behaves, so those two tools have the same headroom in
Vietnamese that they always had in English. English input is unaffected.

### Less work while idle
With auto-popup turned off, the app was still checking your text selection every
half second in the background for a popup it was never going to show. It now
stops that checking entirely while the setting is off, and starts again the
moment you turn it back on. The manual hotkey keeps working either way.

## Known limitations
- **Streaming has never been run against a live provider.** It is verified only
  against synthetic responses matching the OpenAI specification. An endpoint
  that is not OpenAI-compatible, or that streams in a different shape, may fail.
- **The new screens have not been seen running.** The diff marks, the history
  list and the streaming text are verified by code review and automated tests,
  not by watching them render. Expect rough edges in this release.
- Enhance Writing and Fix Grammar still budget output using an estimate rather
  than a real token count, so extreme input can still come up short.
- The Inno Setup installer has still never been compiled; the portable zip is
  the shipping format.

## Notes
- This release targets Windows.
- Portable artifact included in this release: `FastEnglish-0.5.0-portable.zip`
