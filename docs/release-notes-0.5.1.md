# Fast English 0.5.1

## Highlights
- The popup is wider, so the four tool tabs no longer run edge to edge.
- The Enhance button no longer ends up drawn on top of the output box.
- The app no longer mistakes its own resize for you dragging the window.

## What changed

### A popup sized for its own contents
The popup opened at 440x340. The tool tabs need 406 points of that width just to
fit — the four pills come to 358, plus the gaps between them, the row's side
padding and the window border — which left only 34 points of slack, all of it
bunched after the last tab. The row looked like it had been squeezed in.

The default is now 520x400. That leaves 114 points clear after the last tab, so
the row reads as inset rather than edge to edge, and the extra height keeps the
first frame clear of the range where sections started colliding.

The popup still narrows on a display too small for the full width, and a size
you drag the window to still wins over anything the app would have chosen.

### The run button no longer covers the output
Once you had dragged the popup to your own size, the layout switched to filling
that window. If the window was shorter than the editor needs — under about 379
points — the box holding the source and output cards was squeezed while the
cards themselves kept their minimum height, so they spilled out of the bottom of
it. The action row sits at that bottom edge and paints last, which put the
Enhance button directly on top of the output box.

Those cards are now kept inside their box, which scrolls instead of overflowing.
The buttons stay reachable and nothing is drawn over anything else, at any
window size.

### Resizing no longer confuses the app about who resized
The app tells a window you dragged apart from one it sized itself by remembering
the size it last asked for. It recorded that size by truncating it, while
applying the exact fractional value to the window — so on a display where the
window rounded up instead, the size reported back was one the app had never
asked for. It read its own resize as you dragging the frame, and switched to
fill-the-window sizing on its own, at whatever height the window happened to be.
That was one way into the overlapping layout above without touching the window
at all. The recorded size and the applied size are now rounded the same way,
once.

## Known limitations
- **The new popup size is confirmed on screen.** 520x400 was checked in the
  running app before this release.
- **Streaming, the diff marks and the history list have still never been seen
  rendered.** They remain verified by code review and automated tests only, as
  in 0.5.0. Nothing in this release changed them, and nothing in this release
  has made them any more observed.
- **Streaming has still never been run against a live provider.** It is verified
  only against synthetic responses matching the OpenAI specification.
- Enhance Writing and Fix Grammar still budget output using an estimate rather
  than a real token count, so extreme input can still come up short.
- The Inno Setup installer has still never been compiled; the portable zip is
  the shipping format.

## Notes
- This release targets Windows.
- Portable artifact included in this release: `FastEnglish-0.5.1-portable.zip`
