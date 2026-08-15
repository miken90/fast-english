/**
 * Fast English — Universal Editor Popup
 */
'use strict';

const VIEWS = ['editor', 'processing', 'result', 'error'];

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
  markdownPreviewEnabled: false,
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
    exitInlineProcessing();
    dismissEditorErrorBanner();
    renderEditor(data);
    showView('editor');
  },
  processing: function(data) {
    if (data.tool) appState.tool = data.tool;
    if (_previousView === 'editor' || _previousView === 'processing') {
      enterInlineProcessing();
      showView('editor');
    } else {
      renderProcessing(data);
      showView('processing');
    }
  },
  result: function(data) {
    exitInlineProcessing();
    dismissEditorErrorBanner();
    renderResult(data);
    showView('result');
  },
  error: function(data) {
    exitInlineProcessing();
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
  appState.markdownPreviewEnabled = false;
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

function renderProcessing(data) {
  if (data.tool) appState.tool = data.tool;
  var toolTitle = appState.tool && appState.tool.title ? appState.tool.title : 'Processing';

  var topLabel = document.getElementById('processing-tool');
  if (topLabel) topLabel.textContent = toolTitle;

  var label = document.getElementById('processing-label');
  if (label) label.textContent = 'Running ' + toolTitle + '…';
}

function renderResult(data) {
  if (data.tool) appState.tool = data.tool;
  appState.sourceText = unescHtml(data.sourceText || appState.sourceText || '');
  appState.resultOutput = unescHtml(data.output || '');
  appState.canReplace = !!data.canReplace;
  appState.copyFeedback = !!data.copyFeedback;

  var label = document.getElementById('result-tool-label');
  if (label) label.textContent = (appState.tool && appState.tool.title) || 'Result';

  var outputEl = document.getElementById('result-output');
  var richEl = document.getElementById('result-rich-preview');
  var toggleBtn = document.getElementById('result-toggle-md');
  if (outputEl) outputEl.textContent = appState.resultOutput;
  if (richEl) richEl.innerHTML = simpleMarkdown(appState.resultOutput);
  setCharCount('result-char-count', appState.resultOutput);

  setMarkdownPreviewEnabled(appState.markdownPreviewEnabled);

  var snippetEl = document.getElementById('result-source-snippet');
  if (snippetEl) snippetEl.value = appState.sourceText;
  setSourceSnippetExpanded(appState.sourceSnippetExpanded);

  var replaceBtn = document.getElementById('result-replace-btn');
  if (replaceBtn) {
    replaceBtn.disabled = !appState.canReplace;
    replaceBtn.setAttribute('aria-disabled', appState.canReplace ? 'false' : 'true');
  }

  if (toggleBtn) {
    toggleBtn.disabled = appState.resultOutput.trim().length === 0;
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

function renderError(data) {
  if (data.tool) appState.tool = data.tool;
  if (typeof data.sourceText === 'string') appState.sourceText = unescHtml(data.sourceText);

  var msgEl = document.getElementById('error-message');
  if (msgEl) msgEl.textContent = unescHtml(data.message || '') || 'An unexpected error occurred.';
}

function enterInlineProcessing() {
  var editorView = document.getElementById('view-editor');
  if (!editorView) return;
  editorView.classList.add('is-processing');
  editorView.setAttribute('aria-busy', 'true');

  var toolTitle = (appState.tool && appState.tool.title) || 'Processing';
  var headerLabel = editorView.querySelector('.label-upper');
  if (headerLabel) headerLabel.textContent = toolTitle + ' — Running\u2026';

  var spinnerInline = document.getElementById('editor-inline-spinner');
  if (spinnerInline) spinnerInline.style.display = '';

  var runBtn = document.getElementById('editor-run-btn');
  if (runBtn) runBtn.style.display = 'none';
  var cancelBtn = document.getElementById('editor-cancel-btn');
  if (cancelBtn) cancelBtn.style.display = '';
}

function exitInlineProcessing() {
  var editorView = document.getElementById('view-editor');
  if (!editorView) return;
  editorView.classList.remove('is-processing');
  editorView.removeAttribute('aria-busy');

  var headerLabel = editorView.querySelector('.label-upper');
  if (headerLabel) {
    headerLabel.textContent = (appState.tool && appState.tool.title) || 'Editor';
  }

  var spinnerInline = document.getElementById('editor-inline-spinner');
  if (spinnerInline) spinnerInline.style.display = 'none';

  var runBtn = document.getElementById('editor-run-btn');
  if (runBtn) runBtn.style.display = '';
  var cancelBtn = document.getElementById('editor-cancel-btn');
  if (cancelBtn) cancelBtn.style.display = 'none';
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

var _resizeTimer = null;
var _imeComposing = false;

function maxPopupHeight() {
  var avail = (window.screen && window.screen.availHeight) || 800;
  return Math.round(avail * MAX_HEIGHT_RATIO);
}

/** The one region allowed to scroll once the popup hits its height cap. */
function scrollRegionForCurrentView() {
  if (appState.view === 'result') {
    return document.getElementById(
      appState.markdownPreviewEnabled ? 'result-rich-preview' : 'result-output'
    );
  }
  if (appState.view === 'editor') return document.getElementById('editor-source-text');
  if (appState.view === 'error') return document.getElementById('error-message');
  return null;
}

function releaseScrollRegion(el) {
  if (!el) return;
  el.style.maxHeight = '';
  el.classList.remove('scroll-region--at-max');
}

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

function setMarkdownPreviewEnabled(enabled) {
  appState.markdownPreviewEnabled = !!enabled;
  var outputEl = document.getElementById('result-output');
  var richEl = document.getElementById('result-rich-preview');
  var btn = document.getElementById('result-toggle-md');
  if (!outputEl || !richEl || !btn) return;

  if (appState.markdownPreviewEnabled) {
    richEl.innerHTML = simpleMarkdown(appState.resultOutput || '');
    richEl.style.display = '';
    outputEl.style.display = 'none';
    btn.textContent = 'Raw';
  } else {
    richEl.style.display = 'none';
    outputEl.style.display = '';
    btn.textContent = 'Preview';
  }
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
  bindClick('editor-run-btn', runCurrentTool);
  bindClick('editor-cancel-btn', function() { sendToRust('CancelProcessing'); });

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
    setMarkdownPreviewEnabled(!appState.markdownPreviewEnabled);
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