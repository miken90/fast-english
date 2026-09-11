# Fast English 0.6.0

## Highlights
- The popup and settings windows were rebuilt to match the approved design: brand header, rounded tool pills with full names, a compact Run button, a grouped result-mode control, coloured history tags, and a reworked settings layout.
- A skeleton placeholder now fills the processing screen while the first token is on its way, instead of a bare spinner.
- The editor text box can finally scroll. Long input used to be clipped with no way to reach it.
- The editor box no longer opens too short and stay that way until you type.
- Reset to Defaults no longer wipes your saved API key.

## What changed

### The interface was rebuilt, not just restyled
An earlier pass restyled the popup in place — new tokens, focus rings, diff
colours, keyboard-hint chips — but deliberately kept the existing layout. Opening
the app afterwards, almost nothing looked different, because everything that
changed was either invisible at rest or only appeared while interacting.

This release changes the layout itself, across all six screens: the editor, the
processing view, the result view, history, the error view and settings. The
header now carries the product name rather than a context label; the tool pills
are rounded, unlabelled by icon, and show their full names; the primary action
is a compact button on the right instead of a full-width uppercase bar; the
result view groups its display modes into one control and puts the original text
behind a disclosure; history rows lead with a colour-coded tool tag and a
timestamp; settings gained help text under each field and a cleaner footer.

Everything the previous pass delivered is kept — tokens, focus rings, diff
colours, the skeleton, disabled states and Windows High Contrast support.

### The editor text box could never scroll
`#editor-source-text` carried `overflow-y: hidden` set through an id selector,
which outranks the class the app toggles when content passes the height cap. The
box was therefore clamped to a maximum height and then clipped: text past that
point existed but could not be reached by any means.

The declaration is gone, so the class can do its job. Long input scrolls.

### The editor box opened too short
The textarea sizes itself to its content, but that calculation was running while
its view was still hidden. Layout metrics are not available inside a hidden
subtree, so the computation fell back to its four-line minimum and stayed there
until something forced a recalculation — typically the user typing. With text
captured from a selection, the box could open visibly smaller than the text it
already held. The sizing now runs once the view is actually visible.

### Reset to Defaults destroyed the stored API key
Reset blanked every field including the API key box, and an empty key passed the
save guard, so the encrypted key file was overwritten with an empty value. The
confirmation prompt said only "Reset settings?" — nothing indicated a credential
was about to be lost, and it could not be recovered from the app.

Reset now leaves the API key alone. It restores preference defaults, which is
what the control claims to do.

Two related fixes shipped alongside it: an abandoned reset confirmation no longer
survives closing and reopening settings (it used to sit there with a live "Yes"
button over an untouched form), and the settings help text now describes the key
storage accurately — it is encrypted to a file with your Windows account, not
held in Windows Credential Manager.

## Under the hood
- `POPUP_INITIAL_HEIGHT` moved 296 → 326 to match the rebuilt chrome, so the
  first frame is right before the measurement pass lands.
- The design package — spec, screen inventory, tokens and six mockups — is
  tracked at `docs/design/ui-restyle/` and is now the design authority.
  `docs/design-guidelines.md` points at it rather than restating values.

## Known limitations
- **This release has not had a human visual QA pass.** Verification was
  structural and geometric: 129 automated tests, contract checks, and a
  screenshot matrix compared against the mockups at three screen heights. What
  no one has looked at with their own eyes is focus-ring rendering during real
  keyboard navigation and live-window resize behaviour.
- The `auto` theme setting does not apply the light palette. `auto` is written
  straight into the theme attribute while the stylesheet only themes an explicit
  `light`, so an auto user on a light Windows setup still gets the dark
  interface. Choosing Light explicitly works. Pre-existing, not introduced here.
- No automated test talks to a live provider; all 129 run against a local server.
- The Inno Setup installer has still never been compiled; the portable zip
  remains the shipping format.
