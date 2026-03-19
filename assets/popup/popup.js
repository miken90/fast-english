/**
 * Fast English — Popup State Machine + IPC Bridge
 *
 * IPC Protocol:
 *   JS → Rust:  window.ipc.postMessage(JSON.stringify({type, payload}))
 *   Rust → JS:  evaluate_script calls window.setState(data)
 */

'use strict';

// ─── IPC Bridge ────────────────────────────────────────────────────────────

/**
 * Send a message to the Rust host.
 * @param {string} type  - IPC message type (matches Rust enum variant)
 * @param {object} [payload] - Optional payload object
 */
function sendToRust(type, payload) {
  if (window.ipc) {
    window.ipc.postMessage(JSON.stringify({ type, payload: payload || null }));
  } else {
    // Dev mode: log to console
    console.log('[IPC]', type, payload);
  }
}

// ─── View Management ───────────────────────────────────────────────────────

const VIEWS = ['options', 'translate-select', 'prompt-editor', 'translate-result', 'processing', 'result', 'error'];

/**
 * Show one view, hide all others.
 * @param {string} viewName
 */
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
}

// ─── Icon Map ──────────────────────────────────────────────────────────────

const TOOL_ICONS = {
  'enhance_writing': {
    colorClass: 'tool-icon--blue',
    svg: '<svg width="20" height="20" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"/></svg>'
  },
  'fix_grammar': {
    colorClass: 'tool-icon--green',
    svg: '<svg width="20" height="20" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/></svg>'
  },
  'convert_prompt': {
    colorClass: 'tool-icon--blue',
    svg: '<svg width="20" height="20" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path d="M4 19.5V4a2 2 0 012-2h8l6 6v11.5a2.5 2.5 0 01-2.5 2.5h-11A2.5 2.5 0 014 19.5z"/><path d="M14 2v6h6"/><path d="M8 13h8M8 17h5"/></svg>'
  },
  'translate': {
    colorClass: 'tool-icon--amber',
    svg: '<svg width="20" height="20" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><path d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10M12.751 5C11.783 10.77 8.07 15.61 3 18.129"/></svg>'
  }
};

/** Get icon config, with fallback for unknown tool ids */
function getToolIcon(id) {
  // Support both snake_case and kebab-case tool ids
  const normalized = id.replace(/-/g, '_');
  return TOOL_ICONS[normalized] || {
    colorClass: 'tool-icon--blue',
    svg: '<svg width="20" height="20" fill="none" stroke="currentColor" stroke-width="2" viewBox="0 0 24 24"><circle cx="12" cy="12" r="10"/><path d="M12 8v4M12 16h.01"/></svg>'
  };
}

// ─── setState — Main Entry Point ───────────────────────────────────────────

/**
 * Called by Rust via evaluate_script("window.setState({...})")
 * @param {object} data
 */
window.setState = function(data) {
  if (!data || !data.view) return;

  switch (data.view) {

    case 'hidden':
      VIEWS.forEach(function(v) {
        const el = document.getElementById('view-' + v);
        if (el) el.classList.remove('active');
      });
      break;

    case 'options':
      renderOptions(data);
      showView('options');
      break;

    case 'translate-select':
      renderTranslateSelect(data);
      showView('translate-select');
      break;

    case 'translate-result':
      renderTranslateResult(data);
      showView('translate-result');
      break;

    case 'prompt-editor':
      renderPromptEditor(data);
      showView('prompt-editor');
      break;

    case 'processing':
      renderProcessing(data);
      showView('processing');
      break;

    case 'result':
      renderResult(data);
      showView('result');
      break;

    case 'error':
      renderError(data);
      showView('error');
      break;

    default:
      console.warn('[setState] Unknown view:', data.view);
  }

  // Auto-resize the window to fit content (skip for hidden)
  if (data.view !== 'hidden') {
    autoResizePopup();
  }
};

/**
 * Measure the popup container's content height and ask Rust to resize the
 * window so it fits snugly. Runs after a rAF to wait for layout.
 */
function autoResizePopup() {
  requestAnimationFrame(function() {
    var container = document.querySelector('.popup-container');
    if (!container) return;
    var height = container.scrollHeight + 12;
    if (height > 0) {
      sendToRust('ResizePopup', { height: height });
    }
  });
}

// ─── View Renderers ────────────────────────────────────────────────────────

/** Tool IDs that remain enabled even when there is no selected/copied text. */
var ALWAYS_ENABLED_TOOLS = ['convert-prompt', 'translate'];

function renderOptions(data) {
  var hasText = !!data.hasText;

  // Preview text
  const previewEl = document.getElementById('options-preview');
  if (previewEl) {
    const preview = data.preview || '';
    if (preview) {
      previewEl.textContent = preview.length > 70
        ? '"' + preview.slice(0, 67) + '…"'
        : '"' + preview + '"';
      previewEl.style.display = '';
    } else {
      previewEl.textContent = '';
      previewEl.style.display = 'none';
    }
  }

  // Tool list
  const listEl = document.getElementById('tool-list');
  if (!listEl) return;
  listEl.innerHTML = '';

  var arrowSvg = '<svg class="tool-arrow" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M9 5l7 7-7 7"/></svg>';

  const tools = data.tools || [];
  tools.forEach(function(tool) {
    const icon = getToolIcon(tool.id);
    var hasArrow = (tool.id === 'translate' || tool.id === 'convert-prompt');
    var enabled = hasText || ALWAYS_ENABLED_TOOLS.indexOf(tool.id) !== -1;

    const li = document.createElement('li');
    const btn = document.createElement('button');
    btn.className = 'tool-btn' + (enabled ? '' : ' tool-btn--disabled');
    btn.dataset.toolId = tool.id;
    if (!enabled) btn.disabled = true;
    btn.innerHTML =
      '<div class="tool-icon ' + icon.colorClass + '">' + icon.svg + '</div>' +
      '<div class="tool-text">' +
        '<div class="tool-title">' + escHtml(tool.title) + '</div>' +
        '<div class="tool-desc">' + escHtml(tool.description || '') + '</div>' +
      '</div>' +
      (hasArrow ? arrowSvg : '');

    btn.addEventListener('click', function() {
      if (!enabled) return;
      sendToRust('ToolSelected', { kind: tool.id });
    });

    li.appendChild(btn);
    listEl.appendChild(li);
  });
}

/** Stored languages for translate-select so we can rebuild on swap */
let _translateLanguages = [];
let _translateFromId = '';
let _translateToId = '';

function renderTranslateSelect(data) {
  _translateLanguages = data.languages || [];
  _translateFromId = data.from ? data.from.id : (_translateLanguages[0] ? _translateLanguages[0].id : '');
  _translateToId = data.to ? data.to.id : (_translateLanguages[1] ? _translateLanguages[1].id : '');

  populateLangDropdowns();

  // Populate input textarea
  var inputEl = document.getElementById('translate-input-text');
  if (inputEl) {
    inputEl.value = data.original || '';
  }
}

function populateLangDropdowns() {
  const fromSel = document.getElementById('translate-from-select');
  const toSel   = document.getElementById('translate-to-select');
  if (!fromSel || !toSel) return;

  fromSel.innerHTML = '';
  toSel.innerHTML = '';

  _translateLanguages.forEach(function(lang) {
    const optFrom = document.createElement('option');
    optFrom.value = lang.id;
    optFrom.textContent = lang.name;
    if (lang.id === _translateFromId) optFrom.selected = true;
    fromSel.appendChild(optFrom);

    const optTo = document.createElement('option');
    optTo.value = lang.id;
    optTo.textContent = lang.name;
    if (lang.id === _translateToId) optTo.selected = true;
    toSel.appendChild(optTo);
  });
}

function renderPromptEditor(data) {
  var inputEl = document.getElementById('prompt-editor-input');
  if (inputEl) {
    inputEl.value = unescHtml(data.original || '');
  }
}

function renderTranslateResult(data) {
  // Store languages in case re-translate is needed
  if (data.languages) _translateLanguages = data.languages;
  if (data.from) _translateFromId = data.from.id;
  if (data.to)   _translateToId   = data.to.id;

  const fromLabel = document.getElementById('tr-from-label');
  const toLabel   = document.getElementById('tr-to-label');
  if (fromLabel) fromLabel.textContent = data.from ? data.from.name : 'Original';
  if (toLabel)   toLabel.textContent   = data.to   ? data.to.name   : 'Translated';

  const originalText = unescHtml(data.original || '');
  const translatedText = unescHtml(data.translated || '');

  const origEl  = document.getElementById('tr-original-text');
  const transEl = document.getElementById('tr-translated-text');
  if (origEl)  origEl.value = originalText;
  if (transEl) transEl.textContent = translatedText;

  const feedback = document.getElementById('translate-copy-feedback');
  if (feedback) {
    feedback.textContent = '';
    feedback.classList.remove('visible');
  }

  if (data.copyFeedback) {
    showCopyFeedback('translate-copy-feedback', 'Copied!');
  }
}

function renderProcessing(data) {
  const label = document.getElementById('processing-label');
  if (label) {
    label.textContent = data.tool
      ? 'Running ' + data.tool + '…'
      : 'Processing…';
  }
}

function renderResult(data) {
  var outputEl = document.getElementById('result-output');
  var richEl = document.getElementById('result-rich-preview');
  if (outputEl) outputEl.value = data.output || '';
  if (richEl) {
    richEl.innerHTML = simpleMarkdown(data.output || '');
    richEl.style.display = 'none';
  }
  // Reset toggle state
  var toggleBtn = document.getElementById('result-toggle-md');
  if (toggleBtn) toggleBtn.textContent = 'Rich';
  if (outputEl) outputEl.style.display = '';

  var feedback = document.getElementById('result-copy-feedback');
  if (feedback) {
    feedback.textContent = '';
    feedback.classList.remove('visible');
  }

  if (data.copyFeedback) {
    showCopyFeedback('result-copy-feedback', 'Copied!');
  }
}

function renderError(data) {
  const msgEl = document.getElementById('error-message');
  if (msgEl) msgEl.textContent = data.message || 'An unexpected error occurred.';
}

// ─── Copy Feedback ─────────────────────────────────────────────────────────

function showCopyFeedback(elementId, text) {
  const el = document.getElementById(elementId);
  if (!el) return;
  el.textContent = text || 'Copied!';
  el.classList.add('visible');
  setTimeout(function() {
    el.classList.remove('visible');
  }, 1500);
}

function copyToClipboard(text, feedbackId) {
  if (navigator.clipboard && navigator.clipboard.writeText) {
    navigator.clipboard.writeText(text).then(function() {
      showCopyFeedback(feedbackId, 'Copied!');
    }).catch(function() {
      fallbackCopy(text, feedbackId);
    });
  } else {
    fallbackCopy(text, feedbackId);
  }
}

function fallbackCopy(text, feedbackId) {
  try {
    const ta = document.createElement('textarea');
    ta.value = text;
    ta.style.cssText = 'position:fixed;opacity:0;pointer-events:none';
    document.body.appendChild(ta);
    ta.select();
    document.execCommand('copy');
    document.body.removeChild(ta);
    showCopyFeedback(feedbackId, 'Copied!');
  } catch(e) {
    showCopyFeedback(feedbackId, 'Failed');
  }
}

// ─── HTML Escaping ─────────────────────────────────────────────────────────

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

// ─── Event Binding ─────────────────────────────────────────────────────────

document.addEventListener('DOMContentLoaded', function() {

  // ── Options View ──
  bindClick('options-close', function() { sendToRust('Dismiss'); });
  bindClick('options-settings-btn', function() { sendToRust('OpenSettings'); });

  // ── Translate Select ──
  bindClick('translate-select-back',  function() { sendToRust('BackToOptions'); });
  bindClick('translate-select-close', function() { sendToRust('Dismiss'); });

  bindChange('translate-from-select', function(val) {
    _translateFromId = val;
    sendToRust('TranslateFromChanged', { lang_id: val });
  });

  bindChange('translate-to-select', function(val) {
    _translateToId = val;
    sendToRust('TranslateToChanged', { lang_id: val });
  });

  bindClick('translate-swap-btn', function() {
    var tmp = _translateFromId;
    _translateFromId = _translateToId;
    _translateToId = tmp;
    populateLangDropdowns();
    sendToRust('TranslateSwapLangs');
  });

  bindClick('translate-confirm-btn', function() {
    var inputText = getInputValue('translate-input-text');
    sendToRust('TranslateConfirm', {
      from: _translateFromId,
      to:   _translateToId,
      text: inputText || null
    });
  });

  // ── Prompt Editor ──
  bindClick('prompt-editor-back', function() { sendToRust('BackToOptions'); });
  bindClick('prompt-editor-close', function() { sendToRust('Dismiss'); });
  bindClick('prompt-editor-confirm-btn', function() {
    var text = getInputValue('prompt-editor-input');
    sendToRust('PromptEditorConfirm', { text: text });
  });

  // ── Translate Result ──
  bindClick('translate-result-back',  function() { sendToRust('BackToOptions'); });
  bindClick('translate-result-close', function() { sendToRust('Dismiss'); });

  bindClick('translate-copy-btn', function() {
    var text = getTextContent('tr-translated-text');
    copyToClipboard(text, 'translate-copy-feedback');
    sendToRust('TranslateCopyResult');
  });

  bindClick('translate-retranslate-btn', function() {
    sendToRust('TranslateConfirm', {
      from: _translateFromId,
      to:   _translateToId,
      text: getInputValue('tr-original-text')
    });
  });

  bindClick('translate-replace-btn', function() {
    sendToRust('TranslateReplaceResult');
  });

  // ── Result View ──
  bindClick('result-back', function() { sendToRust('BackToOptions'); });
  bindClick('result-close', function() { sendToRust('Dismiss'); });

  bindClick('result-copy-btn', function() {
    var text = getInputValue('result-output');
    copyToClipboard(text, 'result-copy-feedback');
    sendToRust('CopyResult');
  });

  bindClick('result-replace-btn', function() {
    sendToRust('ReplaceResult');
  });

  bindClick('result-toggle-md', function() {
    var outputEl = document.getElementById('result-output');
    var richEl = document.getElementById('result-rich-preview');
    var btn = document.getElementById('result-toggle-md');
    if (!outputEl || !richEl || !btn) return;
    if (richEl.style.display === 'none') {
      richEl.innerHTML = simpleMarkdown(outputEl.value);
      richEl.style.display = '';
      outputEl.style.display = 'none';
      btn.textContent = 'Raw';
    } else {
      richEl.style.display = 'none';
      outputEl.style.display = '';
      btn.textContent = 'Rich';
    }
    autoResizePopup();
  });

  // ── Error View ──
  bindClick('error-back',        function() { sendToRust('BackToOptions'); });
  bindClick('error-close',       function() { sendToRust('Dismiss'); });
  bindClick('error-dismiss-btn', function() { sendToRust('Dismiss'); });
  bindClick('error-retry-btn',   function() { sendToRust('Retry'); });

  // ── ESC key global ──
  document.addEventListener('keydown', function(e) {
    if (e.key === 'Escape') {
      sendToRust('Dismiss');
    }
  });

  // ── Draggable popup ──
  initPopupDrag();
});

// ─── Popup Drag ───────────────────────────────────────────────────────────

function initPopupDrag() {
  var dragging = false;
  var startX = 0;
  var startY = 0;

  function isDragHandle(el) {
    while (el && el !== document.body) {
      if (el.classList && (
        el.classList.contains('popup-header') ||
        el.classList.contains('popup-header--tight') ||
        el.classList.contains('result-header')
      )) return true;
      if (el.tagName === 'BUTTON' || el.tagName === 'SELECT' || el.tagName === 'TEXTAREA' || el.tagName === 'INPUT') return false;
      el = el.parentElement;
    }
    return false;
  }

  document.addEventListener('mousedown', function(e) {
    if (e.button !== 0) return;
    if (!isDragHandle(e.target)) return;
    if (e.target.tagName === 'BUTTON' || e.target.tagName === 'SELECT') return;
    dragging = true;
    startX = e.screenX;
    startY = e.screenY;
    e.preventDefault();
  });

  document.addEventListener('mousemove', function(e) {
    if (!dragging) return;
    var dx = e.screenX - startX;
    var dy = e.screenY - startY;
    if (dx === 0 && dy === 0) return;
    startX = e.screenX;
    startY = e.screenY;
    sendToRust('MovePopup', { dx: dx, dy: dy });
  });

  document.addEventListener('mouseup', function() {
    dragging = false;
  });
}

// ─── DOM Helpers ───────────────────────────────────────────────────────────

function bindClick(id, fn) {
  var el = document.getElementById(id);
  if (el) el.addEventListener('click', fn);
}

function bindChange(id, fn) {
  var el = document.getElementById(id);
  if (el) el.addEventListener('change', function() { fn(el.value); });
}

function getTextContent(id) {
  var el = document.getElementById(id);
  return el ? el.textContent : '';
}

function getInputValue(id) {
  var el = document.getElementById(id);
  return el ? el.value : '';
}

/**
 * Minimal markdown → HTML converter for rich preview.
 * Handles: headings, bold, italic, inline code, code blocks, lists, paragraphs.
 */
function simpleMarkdown(text) {
  if (!text) return '';
  var html = escHtml(text);
  // Code blocks (``` ... ```)
  html = html.replace(/```(\w*)\n([\s\S]*?)```/g, function(_, lang, code) {
    return '<pre class="md-code-block"><code>' + code.trim() + '</code></pre>';
  });
  // Headings
  html = html.replace(/^#### (.+)$/gm, '<h4 class="md-h4">$1</h4>');
  html = html.replace(/^### (.+)$/gm, '<h3 class="md-h3">$1</h3>');
  html = html.replace(/^## (.+)$/gm, '<h2 class="md-h2">$1</h2>');
  html = html.replace(/^# (.+)$/gm, '<h1 class="md-h1">$1</h1>');
  // Bold + italic
  html = html.replace(/\*\*\*(.+?)\*\*\*/g, '<strong><em>$1</em></strong>');
  html = html.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');
  html = html.replace(/\*(.+?)\*/g, '<em>$1</em>');
  // Inline code
  html = html.replace(/`([^`]+)`/g, '<code class="md-inline-code">$1</code>');
  // Unordered list items (group consecutive bullets into one <ul>)
  html = html.replace(/^[\-\*] (.+)$/gm, '<li>$1</li>');
  html = html.replace(/(?:^|\n)(<li>.*<\/li>(?:\n<li>.*<\/li>)*)/gm, function(_, items) {
    return '\n<ul>' + items + '</ul>';
  });
  // Line breaks → paragraphs (double newline)
  html = html.replace(/\n\n/g, '</p><p>');
  html = html.replace(/\n/g, '<br>');
  html = '<p>' + html + '</p>';
  // Clean empty tags
  html = html.replace(/<p>\s*<\/p>/g, '');
  return html;
}
