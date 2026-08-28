/**
 * Fast English — Universal Editor Popup
 */
'use strict';

const VIEWS = ['editor', 'processing', 'result', 'error', 'history'];

const appState = {
  view: 'editor',
  tools: [],
  tool: { id: 'enhance-writing', title: 'Enhance Writing' },
  sourceText: '',
  resultOutput: '',
  canReplace: false,
  copyFeedback: false,
  languages: [],
  translateFrom: null,
  translateTo: null,
  /** How the result is being shown: 'raw' | 'rich' | 'diff'. */
  resultMode: 'raw',
  /** Whether the diff markup matches the result currently on screen. */
  diffRendered: false,
  /** Warning shown above the output when the answer is incomplete. */
  resultNotice: '',
  sourceSnippetExpanded: false
};

function sendToRust(type, payload) {
  if (window.ipc) {
    window.ipc.postMessage(JSON.stringify({ type, payload: payload || null }));
  } else {
    console.log('[IPC]', type, payload);
  }
}

function resolveTheme(preference) {
  if (preference === 'light') return 'light';
  if (preference === 'dark') return 'dark';
  return window.matchMedia && window.matchMedia('(prefers-color-scheme: light)').matches
    ? 'light'
    : 'dark';
}

function applyTheme(preference) {
  var html = document.documentElement;
  if (!html) return;
  var pref = preference || 'auto';
  html.setAttribute('data-theme', pref);
  html.style.colorScheme = resolveTheme(pref);
}

window.setThemePreference = function(theme) {
  applyTheme(theme || 'auto');
};

function showView(viewName) {
  VIEWS.forEach(function(v) {
    const el = document.getElementById('view-' + v);
    if (!el) return;
    if (v === viewName) {
      el.classList.add('active');
    } else {
      el.classList.remove('active');
    }
  });

  var activeView = document.getElementById('view-' + viewName);
  if (activeView) {
    var focusable = activeView.querySelector('button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])');
    if (focusable) {
      setTimeout(function() { focusable.focus(); }, 30);
    }
  }
}

var _previousView = 'editor';

var VIEW_RENDERERS = {
  editor: function(data) {
    dismissEditorErrorBanner();
    renderEditor(data);
    showView('editor');
  },
  // Running always gets its own screen. It is the only view with a live region
  // for streamed text, so the answer starts painting the moment the first
  // token lands instead of after the whole completion.
  processing: function(data) {
    renderProcessing(data);
    showView('processing');
  },
  result: function(data) {
    dismissEditorErrorBanner();
    renderResult(data);
    showView('result');
  },
  history: function(data) {
    dismissEditorErrorBanner();
    renderHistory(data);
    showView('history');
  },
  // A failed run returns to the editor with the reason above the text, so the
  // input is still there to retry or edit. A failure arriving from anywhere
  // else has no editor to return to and gets the full error view.
  error: function(data) {
    if (_previousView === 'editor' || _previousView === 'processing') {
      showEditorErrorBanner(data.message || 'An unexpected error occurred.');
      showView('editor');
    } else {
      renderError(data);
      showView('error');
    }
  }
};

window.setState = function(data) {
  if (!data || !data.view) return;

  if (data.theme) applyTheme(data.theme);

  // Every state push ends the stream that was running: a request either
  // finished, failed, or was replaced by this one. Partial raw text never
  // outlives the request that produced it.
  endStream();

  if (data.view === 'hidden') {
    VIEWS.forEach(function(v) {
      const el = document.getElementById('view-' + v);
      if (el) el.classList.remove('active');
    });
    appState.view = 'hidden';
    return;
  }

  var renderer = VIEW_RENDERERS[data.view];
  if (renderer) {
    _previousView = appState.view;
    appState.view = data.view;
    renderer(data);
  }

  requestSnapResizeForCurrentView();
};

function renderEditor(data) {
  appState.tools = data.tools || [];
  appState.tool = data.tool || appState.tool;
  appState.sourceText = unescHtml(data.sourceText || '');
  appState.canReplace = !!data.canReplace;
  appState.resultMode = 'raw';
  appState.diffRendered = false;
  appState.sourceSnippetExpanded = false;

  if (Array.isArray(data.languages)) appState.languages = data.languages;
  if (data.translateFrom) appState.translateFrom = data.translateFrom;
  if (data.translateTo) appState.translateTo = data.translateTo;

  renderToolSwitcher();

  var headerLabel = document.querySelector('#view-editor .label-upper');
  if (headerLabel) {
    headerLabel.textContent = (appState.tool && appState.tool.title) || 'Editor';
  }

  var runBtn = document.getElementById('editor-run-btn');
  if (runBtn) {
    runBtn.textContent = RUN_LABELS[appState.tool.id] || 'Run';
  }

  var sourceEl = document.getElementById('editor-source-text');
  if (sourceEl) {
    sourceEl.value = appState.sourceText;
    autosizeEditorSourceTextarea(sourceEl);
  }
  setCharCount('editor-char-count', appState.sourceText);
  updateRunButtonState();

  var sourceLabel = document.getElementById('editor-source-label');
  if (sourceLabel) {
    sourceLabel.textContent = appState.tool.id === 'translate' ? 'Text to translate' : 'Source';
  }

  var translateControls = document.getElementById('translate-controls');
  var isTranslate = appState.tool.id === 'translate';
  if (translateControls) {
    translateControls.style.display = isTranslate ? '' : 'none';
  }
  if (isTranslate) {
    populateTranslateDropdowns();
  }
}

const TOOL_SHORT_LABELS = {
  'enhance-writing': 'Enhance',
  'fix-grammar': 'Grammar',
  'convert-prompt': 'Prompt',
  'translate': 'Translate'
};

const RUN_LABELS = {
  'enhance-writing': 'Enhance',
  'fix-grammar': 'Fix',
  'convert-prompt': 'Convert',
  'translate': 'Translate'
};

function renderToolSwitcher() {
  const row = document.getElementById('tool-switcher');
  if (!row) return;
  row.innerHTML = '';

  appState.tools.forEach(function(tool) {
    var btn = document.createElement('button');
    btn.className = 'tool-pill' + (tool.id === appState.tool.id ? ' active' : '');
    btn.type = 'button';
    btn.setAttribute('aria-pressed', tool.id === appState.tool.id ? 'true' : 'false');
    btn.dataset.toolId = tool.id;

    var iconName = tool.id.replace(/-/g, '_');
    var iconSvg = window.renderIcon ? window.renderIcon(iconName, 13, 2.5) : '';
    btn.innerHTML = (iconSvg ? '<span class="tool-pill__icon">' + iconSvg + '</span>' : '') +
                    '<span class="tool-pill__label">' + (TOOL_SHORT_LABELS[tool.id] || tool.title) + '</span>';

    btn.addEventListener('click', function() {
      sendToRust('ToolSelected', { kind: tool.id });
    });
    row.appendChild(btn);
  });
}

function populateTranslateDropdowns() {
  var fromSel = document.getElementById('translate-from-select');
  var toSel = document.getElementById('translate-to-select');
  if (!fromSel || !toSel) return;

  fromSel.innerHTML = '';
  toSel.innerHTML = '';

  appState.languages.forEach(function(lang) {
    var optFrom = document.createElement('option');
    optFrom.value = lang.id;
    optFrom.textContent = lang.name;
    if (appState.translateFrom && lang.id === appState.translateFrom.id) {
      optFrom.selected = true;
    }
    fromSel.appendChild(optFrom);

    var optTo = document.createElement('option');
    optTo.value = lang.id;
    optTo.textContent = lang.name;
    if (appState.translateTo && lang.id === appState.translateTo.id) {
      optTo.selected = true;
    }
    toSel.appendChild(optTo);
  });
}

/* The pre-stream placeholder. Visibility only — it holds no state and the
   stream itself is unchanged. */
function setProcessingSkeletonVisible(visible) {
  var el = document.getElementById('processing-skeleton');
  if (!el) return;
  if (visible) el.removeAttribute('hidden');
  else el.setAttribute('hidden', '');
}

function renderProcessing(data) {
  if (data.tool) appState.tool = data.tool;
  var toolTitle = appState.tool && appState.tool.title ? appState.tool.title : 'Processing';

  var topLabel = document.getElementById('processing-tool');
  if (topLabel) topLabel.textContent = toolTitle;

  var label = document.getElementById('processing-label');
  if (label) label.textContent = 'Running ' + toolTitle + '…';
  setProcessingSkeletonVisible(true);
}

function renderResult(data) {
  if (data.tool) appState.tool = data.tool;
  appState.sourceText = unescHtml(data.sourceText || appState.sourceText || '');
  appState.resultOutput = unescHtml(data.output || '');
  appState.canReplace = !!data.canReplace;
  appState.copyFeedback = !!data.copyFeedback;
  appState.resultNotice = data.notice ? unescHtml(data.notice) : '';

  // The answer is on screen either way; the notice only says it stops short.
  // It belongs to this result alone, so it is cleared whenever one arrives
  // without it rather than lingering over the next answer.
  var noticeEl = document.getElementById('result-notice');
  if (noticeEl) {
    var noticeMsg = noticeEl.querySelector('.result-notice__msg');
    if (noticeMsg) noticeMsg.textContent = appState.resultNotice;
    if (appState.resultNotice) {
      noticeEl.removeAttribute('hidden');
    } else {
      noticeEl.setAttribute('hidden', '');
    }
  }

  var label = document.getElementById('result-tool-label');
  if (label) label.textContent = (appState.tool && appState.tool.title) || 'Result';

  var outputEl = document.getElementById('result-output');
  var richEl = document.getElementById('result-rich-preview');
  var toggleBtn = document.getElementById('result-toggle-md');
  if (outputEl) outputEl.textContent = appState.resultOutput;
  if (richEl) richEl.innerHTML = simpleMarkdown(appState.resultOutput);
  setCharCount('result-char-count', appState.resultOutput);

  // The diff belongs to this result and this result only — a stale one would
  // mark up the previous answer's changes.
  appState.diffRendered = false;
  setResultMode(defaultResultMode(appState.tool && appState.tool.id));

  var snippetEl = document.getElementById('result-source-snippet');
  if (snippetEl) snippetEl.value = appState.sourceText;
  setSourceSnippetExpanded(appState.sourceSnippetExpanded);

  var replaceBtn = document.getElementById('result-replace-btn');
  if (replaceBtn) {
    replaceBtn.disabled = !appState.canReplace;
    replaceBtn.setAttribute('aria-disabled', appState.canReplace ? 'false' : 'true');
  }

  var empty = appState.resultOutput.trim().length === 0;
  if (toggleBtn) {
    toggleBtn.disabled = empty;
  }

  var diffBtn = document.getElementById('result-toggle-diff');
  if (diffBtn) {
    var available = diffAppliesTo(appState.tool && appState.tool.id) && !empty;
    diffBtn.disabled = !available;
    diffBtn.style.display = diffAppliesTo(appState.tool && appState.tool.id) ? '' : 'none';
  }

  var feedback = document.getElementById('result-copy-feedback');
  if (feedback) {
    feedback.textContent = '';
    feedback.classList.remove('visible');
  }
  if (appState.copyFeedback) {
    showCopyFeedback('result-copy-feedback', 'Copied!');
  }
}

/* ---------- Streaming ----------
 * Chunks arrive as raw model text and are appended straight into a region that
 * scrolls on its own. Nothing here measures or resizes the window per chunk:
 * the region absorbs the growth instead, so a long answer cannot turn into a
 * window that jitters on every token. The cleaned result replaces this region
 * once, at completion, through the normal setState path.
 */

var _streaming = false;

/** The regions streamed text is appended into. */
var STREAM_OUTPUT_IDS = ['processing-stream-output'];

/** Where streamed text goes, when a view that can host it is on screen. */
function streamHostForCurrentView() {
  var active = document.querySelector('.view.active');
  if (!active || active.id !== 'view-processing') return null;
  var el = document.getElementById('processing-stream-output');
  return { reveal: el, output: el };
}

/** The height reserved for streamed text, from the screen — never the window. */
function streamRegionHeight() {
  return Math.max(MIN_SCROLL_REGION, Math.round(maxPopupHeight() * STREAM_REGION_RATIO));
}

function beginStream(host) {
  setProcessingSkeletonVisible(false);
  _streaming = true;
  host.output.textContent = '';
  if (host.reveal) host.reveal.removeAttribute('hidden');

  // The region is given its full height before the first chunk lands, so the
  // document stops growing here: everything after this scrolls inside a box
  // that is already the size it will stay. That is what keeps the window from
  // being resized token by token. In manual-size mode the flex rules own the
  // height instead, and an inline one would fight them.
  if (!_manualWindowSize) {
    host.output.style.height = streamRegionHeight() + 'px';
  }
  // The one measurement of a stream — and it is skipped outright when the user
  // owns the window size.
  measureAndResize(true);
}

function endStream() {
  setProcessingSkeletonVisible(false);
  if (!_streaming) return;
  _streaming = false;
  STREAM_OUTPUT_IDS.forEach(function(id) {
    var el = document.getElementById(id);
    if (el) el.setAttribute('hidden', '');
  });
  STREAM_OUTPUT_IDS.forEach(function(id) {
    var el = document.getElementById(id);
    if (!el) return;
    el.textContent = '';
    el.style.height = '';
    releaseScrollRegion(el);
  });
}

window.streamChunk = function(data) {
  if (!data) return;
  var text = unescHtml(data.text || '');
  if (!text) return;

  var host = streamHostForCurrentView();
  if (!host || !host.output) return;
  if (!_streaming) beginStream(host);

  // Whether to keep following the tail has to be decided before the append,
  // while the old scroll position still means something.
  var following = isScrolledToBottom(host.output);
  host.output.textContent += text;
  if (following) host.output.scrollTop = host.output.scrollHeight;
};

function isScrolledToBottom(el) {
  return el.scrollHeight - el.scrollTop - el.clientHeight < 40;
}

/* ---------- History ----------
 * The list is display-only: entries carry snippets and an index, and opening
 * one asks Rust for it by index. The full text stays on the Rust side, which is
 * also what Copy and Edit output read from once an entry is open.
 */

function renderHistory(data) {
  var list = document.getElementById('history-list');
  var empty = document.getElementById('history-empty');
  var count = document.getElementById('history-count');
  if (!list) return;

  var entries = Array.isArray(data.entries) ? data.entries : [];
  list.innerHTML = '';

  if (empty) {
    if (entries.length === 0) {
      empty.removeAttribute('hidden');
    } else {
      empty.setAttribute('hidden', '');
    }
  }
  if (count) {
    count.textContent = entries.length === 1 ? '1 result' : entries.length + ' results';
  }

  entries.forEach(function(entry) {
    var item = document.createElement('button');
    item.className = 'history-item';
    item.type = 'button';
    item.setAttribute('role', 'listitem');

    var head = document.createElement('div');
    head.className = 'history-item__head';

    var tool = document.createElement('span');
    tool.className = 'history-item__tool';
    tool.textContent = unescHtml(entry.tool || '');

    var when = document.createElement('span');
    when.className = 'history-item__time';
    when.textContent = formatTimestamp(entry.timestamp);

    head.appendChild(tool);
    head.appendChild(when);

    var output = document.createElement('div');
    output.className = 'history-item__output';
    output.textContent = unescHtml(entry.outputSnippet || '');

    var source = document.createElement('div');
    source.className = 'history-item__source';
    source.textContent = unescHtml(entry.sourceSnippet || '');

    item.appendChild(head);
    item.appendChild(output);
    item.appendChild(source);

    item.addEventListener('click', function() {
      sendToRust('OpenHistoryEntry', { index: entry.index });
    });

    list.appendChild(item);
  });
}

/** Unix seconds to the user's own locale format. */
function formatTimestamp(seconds) {
  if (!seconds) return '';
  var date = new Date(seconds * 1000);
  if (isNaN(date.getTime())) return '';
  return date.toLocaleString();
}

function renderError(data) {
  if (data.tool) appState.tool = data.tool;
  if (typeof data.sourceText === 'string') appState.sourceText = unescHtml(data.sourceText);

  var msgEl = document.getElementById('error-message');
  if (msgEl) msgEl.textContent = unescHtml(data.message || '') || 'An unexpected error occurred.';
}

function showEditorErrorBanner(message) {
  var banner = document.getElementById('editor-error-banner');
  if (!banner) return;
  var msgSpan = banner.querySelector('.error-banner__msg');
  if (msgSpan) msgSpan.textContent = unescHtml(message || '');
  banner.removeAttribute('hidden');
}

function dismissEditorErrorBanner() {
  var banner = document.getElementById('editor-error-banner');
  if (banner) banner.setAttribute('hidden', '');
}

function runCurrentTool() {
  var sourceEl = document.getElementById('editor-source-text');
  var text = sourceEl ? sourceEl.value : appState.sourceText;
  appState.sourceText = text || '';
  if (!appState.sourceText.trim()) return;
  sendToRust('RunTool', { text: appState.sourceText });
}

function updateRunButtonState() {
  var sourceEl = document.getElementById('editor-source-text');
  var runBtn = document.getElementById('editor-run-btn');
  if (!runBtn) return;
  var empty = !sourceEl || !sourceEl.value.trim();
  runBtn.disabled = empty;
  runBtn.setAttribute('aria-disabled', empty ? 'true' : 'false');
}

function rerunFromResult() {
  sendToRust('RunTool', { text: appState.sourceText || '' });
}

function setCharCount(countId, text) {
  var cc = document.getElementById(countId);
  if (cc) cc.textContent = (text || '').length + ' chars';
}

function autosizeEditorSourceTextarea(textarea) {
  if (!textarea) return;
  // Once the user owns the window size the textarea is sized by the flex
  // layout instead, and an inline height would fight it.
  if (_manualWindowSize) {
    textarea.style.height = '';
    textarea.classList.remove('editor-textarea--scrolling');
    return;
  }

  var style = window.getComputedStyle(textarea);
  var lineHeight = parseFloat(style.lineHeight) || 19.375;
  var paddingTop = parseFloat(style.paddingTop) || 0;
  var paddingBottom = parseFloat(style.paddingBottom) || 0;
  var borderTop = parseFloat(style.borderTopWidth) || 0;
  var borderBottom = parseFloat(style.borderBottomWidth) || 0;
  var verticalExtras = paddingTop + paddingBottom + borderTop + borderBottom;
  var minHeight = lineHeight * 4 + verticalExtras;
  var maxHeight = Math.max(minHeight, Math.round(maxPopupHeight() * 0.6));

  textarea.style.height = 'auto';
  var nextHeight = Math.min(Math.max(textarea.scrollHeight, minHeight), maxHeight);
  textarea.style.height = nextHeight + 'px';
  // A class rather than an inline style, so it cannot outrank the scroll
  // region's own overflow rule when both apply to this textarea.
  textarea.classList.toggle('editor-textarea--scrolling', textarea.scrollHeight > maxHeight);
}

/* ---------- Window sizing ----------
 * The document lays out at its intrinsic height and reports that height to
 * Rust. Nothing here may read a value that depends on the window's own size.
 * Above the cap, exactly one region scrolls; below it, nothing does.
 */

var MAX_HEIGHT_RATIO = 0.7;
var MIN_SCROLL_REGION = 80;
var RESIZE_DEBOUNCE_MS = 150;
/** Share of the height cap handed to streamed text while it is arriving. */
var STREAM_REGION_RATIO = 0.45;

var _resizeTimer = null;
var _imeComposing = false;
/** True once the user has dragged the window frame; Rust decides this. */
var _manualWindowSize = false;

/** Every region the measurement pass may have capped, across all views. */
var SCROLL_REGION_IDS = [
  'editor-source-text',
  'result-output',
  'result-rich-preview',
  'result-diff',
  'history-list',
  'error-message',
  'processing-stream-output'
];

function maxPopupHeight() {
  var avail = (window.screen && window.screen.availHeight) || 800;
  return Math.round(avail * MAX_HEIGHT_RATIO);
}

/** The one region allowed to scroll once the popup hits its height cap. */
function scrollRegionForCurrentView() {
  // Read the view that is actually on screen: processing and errors are shown
  // inside the editor view when they follow one, so the state name alone would
  // point at the wrong region.
  // While a stream is running the growing text is the region that has to be
  // capped, whichever view is hosting it.
  if (_streaming) {
    var host = streamHostForCurrentView();
    if (host && host.output) return host.output;
  }
  var active = document.querySelector('.view.active');
  if (!active) return null;
  if (active.id === 'view-result') {
    return document.getElementById(
      RESULT_MODE_ELEMENT_IDS[appState.resultMode] || RESULT_MODE_ELEMENT_IDS.raw
    );
  }
  if (active.id === 'view-editor') return document.getElementById('editor-source-text');
  if (active.id === 'view-error') return document.getElementById('error-message');
  if (active.id === 'view-history') return document.getElementById('history-list');
  return null;
}

function releaseScrollRegion(el) {
  if (!el) return;
  el.style.maxHeight = '';
  el.classList.remove('scroll-region--at-max');
}

/**
 * Switches between the two sizing directions.
 *
 * Off (the default): the document lays out at its intrinsic height and the
 * window follows it. On (after the user drags the frame): the document fills
 * the window the user chose and measurement stops, so nothing snaps the window
 * back. Called from Rust — `setManualWindowSize(false)` on every show.
 */
window.setManualWindowSize = function(on) {
  var next = !!on;
  if (next === _manualWindowSize) return;
  _manualWindowSize = next;

  var html = document.documentElement;
  if (html) html.classList.toggle('popup-fill-window', next);

  if (next) {
    // A pending measurement would resize the window out from under the user.
    if (_resizeTimer) {
      clearTimeout(_resizeTimer);
      _resizeTimer = null;
    }
    // Inline caps and heights left by the measurement pass outrank the fill
    // rules, so they have to go before the layout can stretch.
    SCROLL_REGION_IDS.forEach(function(id) {
      releaseScrollRegion(document.getElementById(id));
    });
    var textarea = document.getElementById('editor-source-text');
    if (textarea) {
      textarea.style.height = '';
      textarea.classList.remove('editor-textarea--scrolling');
    }
    // Same for the height reserved for a stream in progress: the window the
    // user just chose decides the region's size from here on.
    STREAM_OUTPUT_IDS.forEach(function(id) {
      var el = document.getElementById(id);
      if (el) el.style.height = '';
    });
  } else {
    autosizeEditorSourceTextarea(document.getElementById('editor-source-text'));
  }
};

function applyMeasuredHeight() {
  _resizeTimer = null;
  var root = document.getElementById('popup-root');
  if (!root) return;

  // Always measure at intrinsic height: a cap left over from a previous state
  // would be measured instead of the content.
  var region = scrollRegionForCurrentView();
  releaseScrollRegion(region);

  var maxHeight = maxPopupHeight();
  var needed = root.offsetHeight;

  if (needed > maxHeight && region) {
    var excess = needed - maxHeight;
    var regionHeight = region.getBoundingClientRect().height;
    region.style.maxHeight = Math.max(MIN_SCROLL_REGION, regionHeight - excess) + 'px';
    region.classList.add('scroll-region--at-max');
    needed = root.offsetHeight;
  }

  sendToRust('ResizePopup', { height: Math.min(needed, maxHeight) });
}

/**
 * `immediate` measures on the next frame instead of after the debounce — used
 * for state changes, where a delayed resize shows as the window snapping to a
 * new size after it is already on screen.
 */
function measureAndResize(immediate) {
  // The user's own window size wins over the content's; measuring would snap
  // it back on the next tab switch or result.
  if (_manualWindowSize) return;
  // Resizing the native window mid-composition displaces the IME candidate
  // window, so measurement waits for the composition to finish.
  if (_imeComposing) return;
  if (_resizeTimer) {
    clearTimeout(_resizeTimer);
    _resizeTimer = null;
  }
  if (immediate) {
    requestAnimationFrame(applyMeasuredHeight);
    return;
  }
  _resizeTimer = setTimeout(applyMeasuredHeight, RESIZE_DEBOUNCE_MS);
}

function requestSnapResizeForCurrentView() {
  measureAndResize(true);
}

/* ---------- Result display modes ----------
 * The result is shown one of three ways, and only one at a time: the raw text,
 * a markdown preview, or a word-level diff against what the user wrote. The
 * modes are exclusive because they are three renderings of the same output —
 * and because only one region may scroll (see applyMeasuredHeight).
 *
 * None of this touches the output itself. Copy, Replace and Edit output all
 * read the clean text Rust holds in PopupState::Result, so what the user takes
 * away is never the marked-up version.
 */

var RESULT_MODE_ELEMENT_IDS = {
  raw: 'result-output',
  rich: 'result-rich-preview',
  diff: 'result-diff'
};

/** Tools whose output is a corrected version of the input, so a diff means something. */
function diffAppliesTo(toolId) {
  // Translate and Convert Prompt produce a different text in a different
  // shape — diffing those against the source is noise, not information.
  return toolId === 'fix-grammar' || toolId === 'enhance-writing';
}

/**
 * Fix Grammar opens on the diff: corrections are the entire point of the tool,
 * and its diffs are sparse enough to read at a glance. Enhance Writing opens on
 * the text, because it rewrites for flow — its diff is often a near-total
 * replacement, which reads as noise and buries the result the user came for.
 * Both keep the toggle.
 */
function defaultResultMode(toolId) {
  return toolId === 'fix-grammar' ? 'diff' : 'raw';
}

function setResultMode(mode) {
  var next = RESULT_MODE_ELEMENT_IDS[mode] ? mode : 'raw';
  if (next === 'diff' && !diffAppliesTo(appState.tool && appState.tool.id)) {
    next = 'raw';
  }
  // An empty answer would diff as the whole source struck through, which reads
  // as a deletion the model never made.
  if (next === 'diff' && !(appState.resultOutput || '').trim()) {
    next = 'raw';
  }
  // Only a diff the user asked for and did not get needs explaining.
  var diffTooLarge = false;
  if (next === 'diff' && !renderResultDiff()) {
    // Too large to diff — the plain text is the honest fallback, but silently
    // showing it looks like the toggle did nothing.
    next = 'raw';
    diffTooLarge = true;
  }
  setDiffNoteVisible(diffTooLarge);

  appState.resultMode = next;

  Object.keys(RESULT_MODE_ELEMENT_IDS).forEach(function(name) {
    var el = document.getElementById(RESULT_MODE_ELEMENT_IDS[name]);
    if (el) el.style.display = name === next ? '' : 'none';
  });

  if (next === 'rich') {
    var richEl = document.getElementById(RESULT_MODE_ELEMENT_IDS.rich);
    if (richEl) richEl.innerHTML = simpleMarkdown(appState.resultOutput || '');
  }

  var mdBtn = document.getElementById('result-toggle-md');
  if (mdBtn) {
    mdBtn.textContent = next === 'rich' ? 'Raw' : 'Preview';
    mdBtn.setAttribute('aria-pressed', next === 'rich' ? 'true' : 'false');
  }

  var diffBtn = document.getElementById('result-toggle-diff');
  if (diffBtn) {
    diffBtn.setAttribute('aria-pressed', next === 'diff' ? 'true' : 'false');
    diffBtn.classList.toggle('active', next === 'diff');
  }
}

function setDiffNoteVisible(visible) {
  var note = document.getElementById('result-diff-note');
  if (!note) return;
  if (visible) {
    note.removeAttribute('hidden');
  } else {
    note.setAttribute('hidden', '');
  }
}

/**
 * Build the marked-up diff, once per result. Returns false when the change is
 * too large to diff, so the caller can fall back instead of showing nothing.
 *
 * This runs at completion only. During a stream the popup is still in the
 * processing state, showing raw text as it arrives — diffing a half-finished
 * answer would re-mark the whole text on every chunk.
 */
function renderResultDiff() {
  var el = document.getElementById(RESULT_MODE_ELEMENT_IDS.diff);
  if (!el || !window.diffWords) return false;
  if (appState.diffRendered) return true;

  var ops = window.diffWords(appState.sourceText || '', appState.resultOutput || '');
  if (!ops) return false;

  var html = '';
  for (var i = 0; i < ops.length; i++) {
    var text = escHtml(ops[i].text);
    if (ops[i].type === 'insert') {
      html += '<ins class="diff-ins">' + text + '</ins>';
    } else if (ops[i].type === 'delete') {
      html += '<del class="diff-del">' + text + '</del>';
    } else {
      html += text;
    }
  }

  el.innerHTML = html;
  appState.diffRendered = true;
  return true;
}

function setSourceSnippetExpanded(expanded) {
  appState.sourceSnippetExpanded = !!expanded;
  var toggle = document.getElementById('result-source-toggle');
  var wrap = document.getElementById('result-source-wrap');
  if (!toggle || !wrap) return;

  toggle.setAttribute('aria-expanded', appState.sourceSnippetExpanded ? 'true' : 'false');
  toggle.textContent = appState.sourceSnippetExpanded ? 'Hide source snippet' : 'Source snippet';
  if (appState.sourceSnippetExpanded) {
    wrap.removeAttribute('hidden');
  } else {
    wrap.setAttribute('hidden', '');
  }
}

function showCopyFeedback(elementId, text) {
  const el = document.getElementById(elementId);
  if (!el) return;
  el.textContent = text || 'Copied!';
  el.classList.add('visible');
  setTimeout(function() {
    el.classList.remove('visible');
  }, 1500);
}

function bindClick(id, fn) {
  var el = document.getElementById(id);
  if (el) el.addEventListener('click', fn);
}

function bindChange(id, fn) {
  var el = document.getElementById(id);
  if (el) el.addEventListener('change', function() { fn(el.value); });
}

function getInputValue(id) {
  var el = document.getElementById(id);
  return el ? el.value : '';
}

function escHtml(str) {
  return String(str)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;');
}

function unescHtml(str) {
  return String(str)
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"')
    .replace(/&#039;/g, "'")
    .replace(/&amp;/g, '&');
}

function simpleMarkdown(text) {
  if (!text) return '';
  var html = escHtml(text);
  html = html.replace(/```(\w*)\n([\s\S]*?)```/g, function(_, lang, code) {
    return '<pre class="md-code-block"><code>' + code.trim() + '</code></pre>';
  });
  html = html.replace(/^#### (.+)$/gm, '<h4 class="md-h4">$1</h4>');
  html = html.replace(/^### (.+)$/gm, '<h3 class="md-h3">$1</h3>');
  html = html.replace(/^## (.+)$/gm, '<h2 class="md-h2">$1</h2>');
  html = html.replace(/^# (.+)$/gm, '<h1 class="md-h1">$1</h1>');
  html = html.replace(/\*\*\*(.+?)\*\*\*/g, '<strong><em>$1</em></strong>');
  html = html.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');
  html = html.replace(/\*(.+?)\*/g, '<em>$1</em>');
  html = html.replace(/`([^`]+)`/g, '<code class="md-inline-code">$1</code>');
  html = html.replace(/^[\-\*] (.+)$/gm, '<li>$1</li>');
  html = html.replace(/(?:^|\n)(<li>.*<\/li>(?:\n<li>.*<\/li>)*)/gm, function(_, items) {
    return '\n<ul>' + items + '</ul>';
  });
  html = html.replace(/\n\n/g, '</p><p>');
  html = html.replace(/\n/g, '<br>');
  html = '<p>' + html + '</p>';
  html = html.replace(/<p>\s*<\/p>/g, '');
  return html;
}

function initPopupDrag() {
  // The drag only reports that it started and that the mouse moved. Rust reads
  // the OS cursor to work out the offset, because mouse coordinates here are
  // CSS pixels and would need a per-monitor scale conversion to be usable.
  var dragging = false;
  var lastX = 0;
  var lastY = 0;

  function isDragHandle(el) {
    while (el && el !== document.body) {
      if (el.classList && (el.classList.contains('popup-header') || el.classList.contains('popup-header--tight'))) {
        return true;
      }
      if (el.tagName === 'BUTTON' || el.tagName === 'SELECT' || el.tagName === 'TEXTAREA' || el.tagName === 'INPUT') {
        return false;
      }
      el = el.parentElement;
    }
    return false;
  }

  document.addEventListener('mousedown', function(e) {
    if (e.button !== 0) return;
    if (!isDragHandle(e.target)) return;
    if (e.target.tagName === 'BUTTON' || e.target.tagName === 'SELECT') return;

    dragging = true;
    lastX = e.screenX;
    lastY = e.screenY;
    sendToRust('PopupDragStart');
    e.preventDefault();
  });

  document.addEventListener('mousemove', function(e) {
    if (!dragging) return;
    if (e.screenX === lastX && e.screenY === lastY) return;
    lastX = e.screenX;
    lastY = e.screenY;
    sendToRust('PopupDragMove');
  });

  document.addEventListener('mouseup', function() {
    dragging = false;
  });
}

document.addEventListener('DOMContentLoaded', function() {
  bindClick('editor-close', function() { sendToRust('Dismiss'); });
  bindClick('editor-settings-btn', function() { sendToRust('OpenSettings'); });
  bindClick('editor-history-btn', function() { sendToRust('OpenHistory'); });

  bindClick('history-back', function() { sendToRust('BackToEditor'); });
  bindClick('history-close', function() { sendToRust('Dismiss'); });
  bindClick('history-clear-btn', function() {
    // Stored text is the user's own writing, so clearing is deliberate.
    if (window.confirm('Delete all saved results?')) {
      sendToRust('ClearHistory');
    }
  });
  bindClick('editor-run-btn', runCurrentTool);

  bindClick('editor-error-retry', function() {
    dismissEditorErrorBanner();
    sendToRust('Retry');
  });
  bindClick('editor-error-dismiss', function() {
    dismissEditorErrorBanner();
    requestSnapResizeForCurrentView();
  });

  bindClick('processing-close', function() { sendToRust('Dismiss'); });
  bindClick('processing-cancel-btn', function() { sendToRust('CancelProcessing'); });

  bindClick('result-back', function() { sendToRust('BackToEditor'); });
  bindClick('result-close', function() { sendToRust('Dismiss'); });
  bindClick('result-rerun-btn', rerunFromResult);
  bindClick('result-use-output-btn', function() { sendToRust('UseOutputAsInput'); });
  bindClick('result-replace-btn', function() { sendToRust('ReplaceResult'); });
  bindClick('result-copy-btn', function() { sendToRust('CopyResult'); });

  bindClick('result-toggle-md', function() {
    setResultMode(appState.resultMode === 'rich' ? 'raw' : 'rich');
    requestSnapResizeForCurrentView();
  });

  bindClick('result-toggle-diff', function() {
    setResultMode(appState.resultMode === 'diff' ? 'raw' : 'diff');
    // The mode change alters the content height. This goes through
    // measureAndResize, so a window the user sized by hand is left alone.
    requestSnapResizeForCurrentView();
  });

  bindClick('result-source-toggle', function() {
    setSourceSnippetExpanded(!appState.sourceSnippetExpanded);
    requestSnapResizeForCurrentView();
  });

  bindClick('error-back', function() { sendToRust('BackToEditor'); });
  bindClick('error-close', function() { sendToRust('Dismiss'); });
  bindClick('error-dismiss-btn', function() { sendToRust('Dismiss'); });
  bindClick('error-retry-btn', function() { sendToRust('Retry'); });

  bindChange('translate-from-select', function(val) {
    sendToRust('TranslateFromChanged', { lang_id: val });
  });

  bindChange('translate-to-select', function(val) {
    sendToRust('TranslateToChanged', { lang_id: val });
  });

  bindClick('translate-swap-btn', function() {
    sendToRust('TranslateSwapLangs');
  });

  var editorSource = document.getElementById('editor-source-text');
  if (editorSource) {
    autosizeEditorSourceTextarea(editorSource);
    editorSource.addEventListener('input', function() {
      appState.sourceText = editorSource.value;
      sendToRust('UpdateSourceText', { text: appState.sourceText });
      setCharCount('editor-char-count', appState.sourceText);
      updateRunButtonState();
      // Growing the textarea mid-composition would move the IME candidate
      // window, so the whole layout pass waits for compositionend.
      if (_imeComposing) return;
      autosizeEditorSourceTextarea(editorSource);
      measureAndResize(false);
    });

    editorSource.addEventListener('compositionstart', function() {
      _imeComposing = true;
    });

    editorSource.addEventListener('compositionend', function() {
      _imeComposing = false;
      autosizeEditorSourceTextarea(editorSource);
      measureAndResize(false);
    });
  }

  document.addEventListener('keydown', function(e) {
    if (e.key === 'Escape') {
      e.preventDefault();
      sendToRust('Dismiss');
      return;
    }

    if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
      e.preventDefault();
      if (appState.view === 'editor') {
        runCurrentTool();
      } else if (appState.view === 'result') {
        rerunFromResult();
      }
    }
  });

  initPopupDrag();
  applyTheme('auto');

  if (window.matchMedia) {
    var media = window.matchMedia('(prefers-color-scheme: light)');
    var onThemeMediaChange = function() {
      var current = document.documentElement.getAttribute('data-theme') || 'auto';
      if (current === 'auto') applyTheme('auto');
    };
    if (media.addEventListener) {
      media.addEventListener('change', onThemeMediaChange);
    } else if (media.addListener) {
      media.addListener(onThemeMediaChange);
    }
  }

  if (window.hydrateStaticIcons) {
    window.hydrateStaticIcons();
  }
}
);