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

const appState = {
  translateLanguages: [],
  translateFromId: '',
  translateToId: '',
  resizeRafId: null,
  replaceConfirmVisible: false
};

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

/**
 * Show one view, hide all others.
 * @param {string} viewName
 */
function showView(viewName) {
  if (viewName !== 'result') {
    hideReplaceConfirmBar();
  }

  VIEWS.forEach(function(v) {
    const el = document.getElementById('view-' + v);
    if (!el) return;
    if (v === viewName) {
      el.classList.add('active');
    } else {
      el.classList.remove('active');
    }
  });

  // Focus management: move focus to first focusable element in new view
  var activeView = document.getElementById('view-' + viewName);
  if (activeView) {
    var focusable = activeView.querySelector('button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])');
    if (focusable) {
      setTimeout(function() { focusable.focus(); }, 50);
    }
  }
}

// ─── Icon Map ──────────────────────────────────────────────────────────────

const TOOL_ICONS = {
  'enhance_writing': {
    colorClass: 'tool-icon--blue',
    iconName: 'enhance_writing'
  },
  'fix_grammar': {
    colorClass: 'tool-icon--green',
    iconName: 'fix_grammar'
  },
  'convert_prompt': {
    colorClass: 'tool-icon--blue',
    iconName: 'convert_prompt'
  },
  'translate': {
    colorClass: 'tool-icon--amber',
    iconName: 'translate'
  }
};

/** Get icon config, with fallback for unknown tool ids */
function getToolIcon(id) {
  const normalized = id.replace(/-/g, '_');
  return TOOL_ICONS[normalized] || {
    colorClass: 'tool-icon--blue',
    iconName: 'info'
  };
}

// ─── setState — Main Entry Point ───────────────────────────────────────────

/**
 * Called by Rust via evaluate_script("window.setState({...})")
 * @param {object} data
 */
window.setState = function(data) {
  if (!data || !data.view) return;

  if (data.theme) {
    applyTheme(data.theme);
  }

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
 * Debounced to prevent excessive IPC messages (max 1 per frame).
 */
function autoResizePopup() {
  if (appState.resizeRafId) {
    cancelAnimationFrame(appState.resizeRafId);
  }
  appState.resizeRafId = requestAnimationFrame(function() {
    appState.resizeRafId = null;
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

  var arrowSvg = '<span class="tool-arrow">' + renderIcon('arrow_right', 12, 2.5) + '</span>';

  const tools = data.tools || [];
  var firstEnabledAssigned = false;
  tools.forEach(function(tool) {
    const icon = getToolIcon(tool.id);
    var hasArrow = (tool.id === 'translate' || tool.id === 'convert-prompt');
    var enabled = hasText || ALWAYS_ENABLED_TOOLS.indexOf(tool.id) !== -1;

    const li = document.createElement('li');
    const btn = document.createElement('button');
    btn.className = 'tool-btn' + (enabled ? '' : ' tool-btn--disabled');
    btn.dataset.toolId = tool.id;
    if (!enabled) btn.disabled = true;
    
    // Roving tabindex: first enabled tool gets tabindex="0", rest get "-1"
    if (enabled) {
      btn.setAttribute('tabindex', firstEnabledAssigned ? '-1' : '0');
      firstEnabledAssigned = true;
    }
    
    btn.innerHTML =
      '<div class="tool-icon ' + icon.colorClass + '">' + renderIcon(icon.iconName, 20, 2) + '</div>' +
      '<div class="tool-text">' +
        '<div class="tool-title">' + escHtml(tool.title) + '</div>' +
        '<div class="tool-desc">' + escHtml(tool.description || '') + '</div>' +
      '</div>' +
      (hasArrow ? arrowSvg : '');

    btn.setAttribute('role', 'menuitem');
    btn.setAttribute('aria-label', tool.title + (tool.description ? ': ' + tool.description : ''));

    btn.addEventListener('click', function() {
      if (!enabled) return;
      sendToRust('ToolSelected', { kind: tool.id });
    });

    li.appendChild(btn);
    listEl.appendChild(li);
  });
  
  // Enable keyboard navigation for tool list
  setupToolListKeyboardNav();
}

/** Stored translate state for dropdown rebuilding and re-translate */

function renderTranslateSelect(data) {
  appState.translateLanguages = data.languages || [];
  appState.translateFromId = data.from ? data.from.id : (appState.translateLanguages[0] ? appState.translateLanguages[0].id : '');
  appState.translateToId = data.to ? data.to.id : (appState.translateLanguages[1] ? appState.translateLanguages[1].id : '');

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

  appState.translateLanguages.forEach(function(lang) {
    const optFrom = document.createElement('option');
    optFrom.value = lang.id;
    optFrom.textContent = lang.name;
    if (lang.id === appState.translateFromId) optFrom.selected = true;
    fromSel.appendChild(optFrom);

    const optTo = document.createElement('option');
    optTo.value = lang.id;
    optTo.textContent = lang.name;
    if (lang.id === appState.translateToId) optTo.selected = true;
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
  if (data.languages) appState.translateLanguages = data.languages;
  if (data.from) appState.translateFromId = data.from.id;
  if (data.to)   appState.translateToId   = data.to.id;

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

function showReplaceConfirmBar() {
  if (appState.replaceConfirmVisible) return;
  var actionRow = document.querySelector('#view-result .action-row');
  if (!actionRow) {
    sendToRust('ReplaceResult');
    return;
  }

  var existing = document.getElementById('replace-confirm-bar');
  if (existing) existing.remove();

  var bar = document.createElement('div');
  bar.id = 'replace-confirm-bar';
  bar.className = 'confirm-bar';
  bar.setAttribute('aria-live', 'assertive');
  bar.innerHTML =
    '<span class="confirm-bar__text">Replace selection?</span>' +
    '<div class="confirm-bar__actions">' +
      '<button class="btn btn--primary btn--confirm" id="replace-confirm-yes">Yes</button>' +
      '<button class="btn btn--secondary btn--confirm" id="replace-confirm-no">No</button>' +
    '</div>';

  actionRow.parentNode.insertBefore(bar, actionRow);
  appState.replaceConfirmVisible = true;

  bindClick('replace-confirm-yes', function() {
    hideReplaceConfirmBar();
    sendToRust('ReplaceResult');
  });
  bindClick('replace-confirm-no', function() {
    hideReplaceConfirmBar();
  });

  var yesBtn = document.getElementById('replace-confirm-yes');
  if (yesBtn) {
    yesBtn.focus();
  }

  bar.addEventListener('keydown', function(e) {
    if (e.key === 'Enter') {
      e.preventDefault();
      hideReplaceConfirmBar();
      sendToRust('ReplaceResult');
    }
    if (e.key === 'Escape') {
      e.preventDefault();
      hideReplaceConfirmBar();
    }
  });

  autoResizePopup();
}

function hideReplaceConfirmBar() {
  var bar = document.getElementById('replace-confirm-bar');
  if (bar) bar.remove();
  appState.replaceConfirmVisible = false;
  autoResizePopup();
}

function animateCopyButton(buttonId, defaultLabel) {
  var btn = document.getElementById(buttonId);
  if (!btn) return;
  var original = btn.innerHTML;
  btn.innerHTML = '<span class="copy-check">' + renderIcon('fix_grammar', 14, 2) + '</span>';
  btn.classList.add('copy-animated');
  setTimeout(function() {
    btn.classList.remove('copy-animated');
    btn.innerHTML = defaultLabel || original;
  }, 900);
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
    appState.translateFromId = val;
    sendToRust('TranslateFromChanged', { lang_id: val });
  });

  bindChange('translate-to-select', function(val) {
    appState.translateToId = val;
    sendToRust('TranslateToChanged', { lang_id: val });
  });

  bindClick('translate-swap-btn', function() {
    var tmp = appState.translateFromId;
    appState.translateFromId = appState.translateToId;
    appState.translateToId = tmp;
    populateLangDropdowns();
    sendToRust('TranslateSwapLangs');
  });

  bindClick('translate-confirm-btn', function() {
    var inputText = getInputValue('translate-input-text');
    sendToRust('TranslateConfirm', {
      from: appState.translateFromId,
      to:   appState.translateToId,
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
    animateCopyButton('translate-copy-btn', 'Copy');
    sendToRust('TranslateCopyResult');
  });

  bindClick('translate-retranslate-btn', function() {
    sendToRust('TranslateConfirm', {
      from: appState.translateFromId,
      to:   appState.translateToId,
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
    animateCopyButton('result-copy-btn', 'Copy');
    sendToRust('CopyResult');
  });

  bindClick('result-replace-btn', function() {
    showReplaceConfirmBar();
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

  // ── Processing View ──
  bindClick('processing-cancel-btn', function() { sendToRust('CancelProcessing'); });

  // ── ESC key global ──
  document.addEventListener('keydown', function(e) {
    if (e.key === 'Escape') {
      if (appState.replaceConfirmVisible) {
        e.preventDefault();
        hideReplaceConfirmBar();
        return;
      }
      sendToRust('Dismiss');
    }
  });

  // ── Draggable popup ──
  initPopupDrag();

  // Initialize theme from system preference
  applyTheme('auto');

  if (window.matchMedia) {
    var media = window.matchMedia('(prefers-color-scheme: light)');
    var onThemeMediaChange = function() {
      var current = document.documentElement.getAttribute('data-theme') || 'auto';
      if (current === 'auto') {
        applyTheme('auto');
      }
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

  // Set aria-live on feedback elements (backup for HTML attrs)
  ['result-copy-feedback', 'translate-copy-feedback', 'processing-label'].forEach(function(id) {
    var el = document.getElementById(id);
    if (el && !el.hasAttribute('aria-live')) {
      el.setAttribute('aria-live', 'polite');
    }
  });
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

function setupToolListKeyboardNav() {
  const listEl = document.getElementById('tool-list');
  if (!listEl) return;
  
  // Remove old listener if exists
  listEl.removeEventListener('keydown', handleToolListKeydown);
  listEl.addEventListener('keydown', handleToolListKeydown);
}

function handleToolListKeydown(e) {
  const toolButtons = Array.from(document.querySelectorAll('.tool-btn:not(.tool-btn--disabled)'));
  if (toolButtons.length === 0) return;
  
  const currentIndex = toolButtons.findIndex(function(btn) {
    return btn.getAttribute('tabindex') === '0';
  });
  
  var nextIndex = currentIndex;
  
  switch (e.key) {
    case 'ArrowDown':
      e.preventDefault();
      nextIndex = (currentIndex + 1) % toolButtons.length;
      break;
    case 'ArrowUp':
      e.preventDefault();
      nextIndex = (currentIndex - 1 + toolButtons.length) % toolButtons.length;
      break;
    case 'Home':
      e.preventDefault();
      nextIndex = 0;
      break;
    case 'End':
      e.preventDefault();
      nextIndex = toolButtons.length - 1;
      break;
    case 'Enter':
    case ' ':
      e.preventDefault();
      if (currentIndex >= 0 && toolButtons[currentIndex]) {
        toolButtons[currentIndex].click();
      }
      return;
    default:
      return;
  }
  
  // Move focus
  if (nextIndex !== currentIndex && nextIndex >= 0) {
    toolButtons.forEach(function(btn) {
      btn.setAttribute('tabindex', '-1');
    });
    toolButtons[nextIndex].setAttribute('tabindex', '0');
    toolButtons[nextIndex].focus();
  }
}

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
