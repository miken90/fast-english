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
  resizeRafId: null
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

window.setState = function(data) {
  if (!data || !data.view) return;

  if (data.theme) applyTheme(data.theme);

  appState.view = data.view;

  if (data.view === 'hidden') {
    VIEWS.forEach(function(v) {
      const el = document.getElementById('view-' + v);
      if (el) el.classList.remove('active');
    });
    return;
  }

  if (data.view === 'editor') {
    renderEditor(data);
    showView('editor');
  } else if (data.view === 'processing') {
    renderProcessing(data);
    showView('processing');
  } else if (data.view === 'result') {
    renderResult(data);
    showView('result');
  } else if (data.view === 'error') {
    renderError(data);
    showView('error');
  }

  autoResizePopup();
};

function renderEditor(data) {
  appState.tools = data.tools || [];
  appState.tool = data.tool || appState.tool;
  appState.sourceText = unescHtml(data.sourceText || '');
  appState.canReplace = !!data.canReplace;

  if (Array.isArray(data.languages)) appState.languages = data.languages;
  if (data.translateFrom) appState.translateFrom = data.translateFrom;
  if (data.translateTo) appState.translateTo = data.translateTo;

  renderToolSwitcher();

  var sourceEl = document.getElementById('editor-source-text');
  if (sourceEl) sourceEl.value = appState.sourceText;

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

function renderToolSwitcher() {
  const row = document.getElementById('tool-switcher');
  if (!row) return;
  row.innerHTML = '';

  appState.tools.forEach(function(tool) {
    var btn = document.createElement('button');
    btn.className = 'tool-pill' + (tool.id === appState.tool.id ? ' active' : '');
    btn.type = 'button';
    btn.textContent = tool.title;
    btn.setAttribute('role', 'tab');
    btn.setAttribute('aria-selected', tool.id === appState.tool.id ? 'true' : 'false');
    btn.dataset.toolId = tool.id;
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
  if (outputEl) outputEl.value = appState.resultOutput;

  var richEl = document.getElementById('result-rich-preview');
  if (richEl) {
    richEl.innerHTML = simpleMarkdown(appState.resultOutput);
    richEl.style.display = 'none';
  }

  var toggleBtn = document.getElementById('result-toggle-md');
  if (toggleBtn) toggleBtn.textContent = 'Rich';
  if (outputEl) outputEl.style.display = '';

  var replaceBtn = document.getElementById('result-replace-btn');
  if (replaceBtn) {
    replaceBtn.disabled = !appState.canReplace;
    replaceBtn.setAttribute('aria-disabled', appState.canReplace ? 'false' : 'true');
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
  if (msgEl) msgEl.textContent = data.message || 'An unexpected error occurred.';
}

function runCurrentTool() {
  var sourceEl = document.getElementById('editor-source-text');
  var text = sourceEl ? sourceEl.value : appState.sourceText;
  appState.sourceText = text || '';
  sendToRust('RunTool', { text: appState.sourceText });
}

function rerunFromResult() {
  sendToRust('RunTool', { text: appState.sourceText || '' });
}

function autoResizePopup() {
  if (appState.resizeRafId) cancelAnimationFrame(appState.resizeRafId);

  appState.resizeRafId = requestAnimationFrame(function() {
    appState.resizeRafId = null;
    var container = document.querySelector('.popup-container');
    if (!container) return;
    var height = container.scrollHeight + 12;
    if (height > 0) sendToRust('ResizePopup', { height: height });
  });
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
  var dragging = false;
  var startX = 0;
  var startY = 0;

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

document.addEventListener('DOMContentLoaded', function() {
  bindClick('editor-close', function() { sendToRust('Dismiss'); });
  bindClick('editor-settings-btn', function() { sendToRust('OpenSettings'); });
  bindClick('editor-run-btn', runCurrentTool);

  bindClick('processing-close', function() { sendToRust('Dismiss'); });
  bindClick('processing-cancel-btn', function() { sendToRust('CancelProcessing'); });

  bindClick('result-back', function() { sendToRust('BackToEditor'); });
  bindClick('result-close', function() { sendToRust('Dismiss'); });
  bindClick('result-rerun-btn', rerunFromResult);
  bindClick('result-use-output-btn', function() { sendToRust('UseOutputAsInput'); });
  bindClick('result-replace-btn', function() { sendToRust('ReplaceResult'); });
  bindClick('result-copy-btn', function() { sendToRust('CopyResult'); });

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
    editorSource.addEventListener('input', function() {
      appState.sourceText = editorSource.value;
      sendToRust('UpdateSourceText', { text: appState.sourceText });
      autoResizePopup();
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