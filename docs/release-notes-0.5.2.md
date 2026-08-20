# Fast English 0.5.2

## Highlights
- Answers longer than 30 seconds no longer die halfway — an overall HTTP timeout was killing them.
- The empty Output box on the editor screen is gone.
- When something goes wrong you now get a sentence saying what and what to change, instead of a status code or a hang.
- An answer that stops short is handed to you with a warning instead of being thrown away.
- Every tool's prompt was rewritten: select a question and it gets edited, not answered.

## What changed

### Long answers were being cut off at 30 seconds
The HTTP client carried a single 30 second timeout covering the whole request —
including reading the streamed answer. Any completion that took longer than that
was killed partway through, which looked like the app hanging or failing for no
reason on exactly the longer texts where it mattered most.

That timeout is gone. In its place are two that bound the right things: 10
seconds to establish a connection, and 60 seconds without a single byte arriving.
A stream that keeps producing text now runs as long as it needs; a server that
accepts the request and then goes quiet fails with a message instead of hanging
forever. Nothing can hang indefinitely any more, and nothing is cut off for
taking its time.

### The dead Output box on the editor screen
The editor screen had a second card labelled Output that was never useful — it
was marked hidden in the markup, yet it showed up empty before you had run
anything.

The cause: the `hidden` attribute only carries the browser's own `display: none`,
which any stylesheet rule outranks. A layout rule that sets `display: flex` on
every editor card — active once you have dragged the popup to your own size —
was quietly overriding it, so the hidden card appeared. The same class of bug
had been patched once before for one specific element; it is now fixed for every
hidden element at once.

The card is removed. Running a tool goes to its own screen, which shows the
answer as it streams in, so the first words appear as soon as the model produces
them.

### Failures now say what happened
Previously a failure surfaced as a raw status code, a dump of whatever the
server sent back, or nothing at all. Each of these now arrives as a sentence
naming the cause and, where relevant, the setting to change:

- the endpoint is unreachable, the name does not resolve, or the connection was refused
- the connection timed out, or the answer stalled partway through
- the API key was rejected (401/403)
- the base URL or the model name is wrong (404 — both are named in the message)
- rate limited (429), including how long the provider asked you to wait
- the provider failed on its own side (5xx)
- the server returned a web page instead of an API response — described, never pasted into the popup
- a success response that carried an error, no completion, or empty text
- the text is too long for the model to process in one request
- there is no text to work on — no request is sent at all

This matters more than usual if you go through a proxy rather than straight to
OpenAI: proxies buffer, return non-JSON errors, and sometimes report failures
with a success status. All of those are handled explicitly.

### An incomplete answer is kept, not discarded
If the model hits its output limit mid-sentence, or the connection breaks after
a usable amount of text has already arrived, you now get that text as a normal
result with a warning above it saying it stops short. Previously the whole thing
was thrown away and you were left with nothing after waiting for it.

A stream that breaks having delivered almost nothing is still an error — a
fragment of a few characters helps nobody, and the reason it failed is the more
useful thing to show.

### Faster first response
The connection to your AI endpoint is now opened while the popup is on screen,
before you press anything, so the first run of a session does not wait for a TLS
handshake. Connections are also kept alive between runs for five minutes instead
of ninety seconds, so rerunning a few minutes later reuses one.

### Prompts rewritten for every tab
The biggest failure these tools had was answering your text instead of editing
it — select "Should I go?", ask for a grammar fix, and get advice back. Every
prompt now states that the input is data to transform, never a request, and the
request is framed so the two halves cannot be confused.

Per tab:

- **Enhance** and **Fix Grammar** now answer in the same language as the input.
  Vietnamese in, Vietnamese out — they will not translate.
- **Fix Grammar** is explicitly a minimal edit: it corrects errors and leaves
  your word choice, sentence order, tone and formatting alone.
- **Enhance** will not shift how formal you sound, and will not add facts,
  examples or sign-offs.
- **Translate** keeps proper nouns, code, URLs, numbers and formatting, returns
  text already in the target language unchanged rather than rewriting it, and
  translates questions instead of answering them.
- **Convert to Prompt** keeps its contract — the output is the prompt itself, in
  English, in markdown — and no longer answers a question it was asked to turn
  into a prompt.

All four preserve names, numbers, dates, URLs, file paths, code and emoji
exactly, and none of them will refuse, apologise, or ask you for clarification.

## Verification
129 automated tests pass on Windows, covering every failure class above against
a local server that deliberately sends malformed, truncated, and silent
responses.

**What is not covered:** there is no automated test against a live AI provider.
The end-to-end behaviour of this release — the new run screen, the warning
banner, and the failure messages as they actually appear against a real
endpoint — was verified by hand on a test build before release, not by the test
suite.

## Install
Download `FastEnglish-0.5.2-portable.zip`, extract it, and run `FastEnglish.exe`.
It appears in your system tray. Right-click the tray icon for Settings.
