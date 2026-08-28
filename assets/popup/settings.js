/**
 * Fast English — Settings IPC Bridge + Form Logic
 *
 * IPC Protocol:
 *   JS → Rust:  window.ipc.postMessage(JSON.stringify({type, payload}))
 *   Rust → JS:  evaluate_script calls window.setSettings(data)
 *               evaluate_script calls window.setTestResult(data)
 *               evaluate_script calls window.setSaveFeedback(data)
 */

'use strict';

// ─── IPC Bridge ────────────────────────────────────────────────────────────

function sendToRust(type, payload) {
  if (window.ipc) {
    window.ipc.postMessage(JSON.stringify({ type, payload: payload || null }));
  } else {
    console.log('[IPC Settings]', type, payload);
  }
}

// ─── Rust → JS API ─────────────────────────────────────────────────────────

/**
 * Called by Rust to populate the form with current settings.
 * @param {object} data
 * @param {string} data.base_url
 * @param {string} data.model
 * @param {string} [data.api_key]  - May be empty/masked
 * @param {boolean} data.auto_popup
 * @param {boolean} data.auto_start
 * @param {string} [data.version]  - App version from Cargo.toml
 */
window.setSettings = function(data) {
  if (!data) return;

  setInputValue('input-base-url', data.base_url || '');
  setInputValue('input-model',    data.model    || '');
  setInputValue('input-api-key',  data.api_key  || '');

  setCheckbox('toggle-auto-popup',  !!data.auto_popup);
  setCheckbox('toggle-auto-start',  !!data.auto_start);
  setInputValue('input-theme', data.theme || 'auto');
  applyTheme(data.theme || 'auto');

  // Update version label if provided
  if (data.version) {
    var versionEl = document.getElementById('version-label');
    if (versionEl) {
      versionEl.textContent = 'Fast English v' + data.version;
    }
  }

  // The settings window is created once and only shown/hidden thereafter, so
  // DOMContentLoaded fires once for the life of the process. Re-render the
  // idle Reset control on every open so an aborted confirm never survives
  // close and reopen.
  renderResetIdle();

  snapshotFormData();
};

/**
 * Called by Rust after TestConnection completes.
 * @param {object} data
 * @param {string} data.status  - 'idle' | 'testing' | 'success' | 'failure'
 * @param {string} data.message
 */
window.setTestResult = function(data) {
  if (!data) return;
  updateTestBadge(data.status || 'idle', data.message || '');
};

/**
 * Called by Rust after SaveSettings completes.
 * @param {object} data
 * @param {string} data.message
 * @param {boolean} data.isOk
 */
window.setSaveFeedback = function(data) {
  if (!data) return;
  showSaveFeedback(data.message || '', !!data.isOk);
};

// ─── Test Connection Badge ──────────────────────────────────────────────────

var TEST_STATUS_CLASSES = ['status-badge--idle', 'status-badge--testing', 'status-badge--success', 'status-badge--failure'];

function updateTestBadge(status, message) {
  var badge = document.getElementById('test-status-badge');
  var text  = document.getElementById('test-status-text');
  if (!badge || !text) return;

  TEST_STATUS_CLASSES.forEach(function(cls) { badge.classList.remove(cls); });
  badge.classList.add('status-badge--' + status);

  switch (status) {
    case 'testing':  text.textContent = 'Testing…'; break;
    case 'success':  text.textContent = message || 'Connected'; break;
    case 'failure':  text.textContent = message || 'Failed'; break;
    default:         text.textContent = 'Not tested'; break;
  }
}

// ─── Save Feedback ─────────────────────────────────────────────────────────

var _saveFeedbackTimer = null;

function showSaveFeedback(message, isOk) {
  var el = document.getElementById('save-feedback');
  if (!el) return;

  el.textContent = message;
  el.classList.remove('ok', 'fail');
  el.classList.add(isOk ? 'ok' : 'fail');
  el.classList.add('visible');

  if (_saveFeedbackTimer) clearTimeout(_saveFeedbackTimer);
  _saveFeedbackTimer = setTimeout(function() {
    el.classList.remove('visible');
  }, 2500);
}

// ─── Collect Form Data ─────────────────────────────────────────────────────

function getFormData() {
  return {
    base_url:   getInputValue('input-base-url'),
    model:      getInputValue('input-model'),
    api_key:    getInputValue('input-api-key'),
    auto_popup: getCheckboxValue('toggle-auto-popup'),
    auto_start: getCheckboxValue('toggle-auto-start'),
    theme:      getInputValue('input-theme') || 'auto',
  };
}

// ─── DOM Helpers ───────────────────────────────────────────────────────────

function setInputValue(id, val) {
  var el = document.getElementById(id);
  if (el) el.value = val;
}

function getInputValue(id) {
  var el = document.getElementById(id);
  return el ? el.value : '';
}

function setCheckbox(id, checked) {
  var el = document.getElementById(id);
  if (el) el.checked = checked;
}

function getCheckboxValue(id) {
  var el = document.getElementById(id);
  return el ? el.checked : false;
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
  html.setAttribute('data-theme', preference || 'auto');
  var resolved = resolveTheme(preference || 'auto');
  html.style.colorScheme = resolved;
}

window.setThemePreference = function(theme) {
  applyTheme(theme || 'auto');
};

// ─── Event Binding ─────────────────────────────────────────────────────────

var initialFormData = null;

function snapshotFormData() {
  initialFormData = getFormData();
  checkDirty();
}

function checkDirty() {
  var current = getFormData();
  var dirty = !initialFormData ||
    current.base_url   !== initialFormData.base_url   ||
    current.model      !== initialFormData.model      ||
    current.api_key    !== initialFormData.api_key    ||
    current.auto_popup !== initialFormData.auto_popup ||
    current.auto_start !== initialFormData.auto_start ||
    current.theme      !== initialFormData.theme;

  var root = document.getElementById('settings-root');
  if (root) root.classList.toggle('is-dirty', dirty);

  var saveBtn = document.getElementById('btn-save');
  if (saveBtn) saveBtn.disabled = !dirty;
}

function resetFormToDefaults() {
  setInputValue('input-base-url', 'https://api.openai.com/v1');
  setInputValue('input-model', 'gpt-4.1-mini');
  // The API key is deliberately NOT reset: getFormData() feeds this input
  // straight to KeyStore::save, and an empty value would DPAPI-encrypt "" over
  // the stored key. Reset restores preferences, not credentials.
  setCheckbox('toggle-auto-popup', true);
  setCheckbox('toggle-auto-start', false);
  setInputValue('input-theme', 'auto');
  applyTheme('auto');
}

// ─── Reset to Defaults ─────────────────────────────────────────────────────

var RESET_IDLE_HTML =
  '<button class="btn btn--ghost btn--sm" id="btn-reset-settings">Reset to defaults</button>';
var RESET_CONFIRM_HTML =
  '<span class="reset-confirm">Reset preferences?' +
  '<button class="btn btn--ghost btn--sm" id="btn-reset-yes">Yes</button>' +
  '<button class="btn btn--ghost btn--sm" id="btn-reset-no">No</button></span>';

function renderResetIdle() {
  var area = document.getElementById('reset-area');
  if (!area) return;
  area.innerHTML = RESET_IDLE_HTML;
}

// Delegation: any innerHTML write into #reset-area detaches a node a direct
// listener was attached to, so the listener lives on the container.
document.getElementById('reset-area').addEventListener('click', function (e) {
  var t = e.target;
  if (!t || !t.id) return;
  if (t.id === 'btn-reset-settings') {
    document.getElementById('reset-area').innerHTML = RESET_CONFIRM_HTML;
  } else if (t.id === 'btn-reset-yes') {
    resetFormToDefaults();
    checkDirty();
    renderResetIdle();
  } else if (t.id === 'btn-reset-no') {
    renderResetIdle();
  }
});

function restoreInitialFormData() {
  if (!initialFormData) return;
  setInputValue('input-base-url', initialFormData.base_url || '');
  setInputValue('input-model', initialFormData.model || '');
  setInputValue('input-api-key', initialFormData.api_key || '');
  setCheckbox('toggle-auto-popup', !!initialFormData.auto_popup);
  setCheckbox('toggle-auto-start', !!initialFormData.auto_start);
  setInputValue('input-theme', initialFormData.theme || 'auto');
  applyTheme(initialFormData.theme || 'auto');
}

document.addEventListener('DOMContentLoaded', function() {
  if (window.hydrateStaticIcons) {
    window.hydrateStaticIcons();
  }

  renderResetIdle();

  // Close
  var closeBtn = document.getElementById('settings-close');
  if (closeBtn) {
    closeBtn.addEventListener('click', function() {
      restoreInitialFormData();
      sendToRust('CloseSettings');
    });
  }

  // Test Connection
  var testBtn = document.getElementById('btn-test-connection');
  if (testBtn) {
    testBtn.addEventListener('click', function() {
      updateTestBadge('testing', '');
      sendToRust('TestConnection', getFormData());
    });
  }

  // Save Settings
  var saveBtn = document.getElementById('btn-save');
  if (saveBtn) {
    saveBtn.addEventListener('click', function() {
      sendToRust('SaveSettings', getFormData());
      snapshotFormData();
    });
  }

  var cancelBtn = document.getElementById('btn-cancel-settings');
  if (cancelBtn) {
    cancelBtn.addEventListener('click', function() {
      restoreInitialFormData();
      sendToRust('CloseSettings');
    });
  }

  // Live field changes → notify Rust (theme only for live preview)
  var fields = [
    { id: 'input-base-url', field: 'base_url' },
    { id: 'input-model',    field: 'model'    },
    { id: 'input-api-key',  field: 'api_key'  },
    { id: 'input-theme',    field: 'theme'    },
  ];

  fields.forEach(function(f) {
    var el = document.getElementById(f.id);
    if (!el) return;
    var eventType = f.field === 'theme' ? 'change' : 'input';
    el.addEventListener(eventType, function() {
      if (f.field === 'theme') {
        applyTheme(el.value || 'auto');
        sendToRust('SettingsChanged', { field: f.field, value: el.value });
      }
      checkDirty();
    });
  });

  // Toggle changes
  var toggles = [
    { id: 'toggle-auto-popup',  field: 'auto_popup' },
    { id: 'toggle-auto-start',  field: 'auto_start' },
  ];

  toggles.forEach(function(t) {
    var el = document.getElementById(t.id);
    if (!el) return;
    el.addEventListener('change', function() {
      checkDirty();
    });
  });

  // ESC → close (discard unsaved changes)
  document.addEventListener('keydown', function(e) {
    if (e.key === 'Escape') {
      restoreInitialFormData();
      sendToRust('CloseSettings');
    }
  });

  // Enter in inputs (but not in URL field)
  var modelInput = document.getElementById('input-model');
  if (modelInput) {
    modelInput.addEventListener('keydown', function(e) {
      if (e.key === 'Enter') {
        sendToRust('SaveSettings', getFormData());
      }
    });
  }
});
