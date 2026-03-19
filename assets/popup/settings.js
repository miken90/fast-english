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
  
  // Update version label if provided
  if (data.version) {
    var versionEl = document.getElementById('version-label');
    if (versionEl) {
      versionEl.textContent = 'Fast English v' + data.version;
    }
  }
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

// ─── Event Binding ─────────────────────────────────────────────────────────

document.addEventListener('DOMContentLoaded', function() {

  // Close
  var closeBtn = document.getElementById('settings-close');
  if (closeBtn) {
    closeBtn.addEventListener('click', function() {
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
    });
  }

  // Live field changes → notify Rust
  var fields = [
    { id: 'input-base-url', field: 'base_url' },
    { id: 'input-model',    field: 'model'    },
    { id: 'input-api-key',  field: 'api_key'  },
  ];

  fields.forEach(function(f) {
    var el = document.getElementById(f.id);
    if (!el) return;
    el.addEventListener('input', function() {
      sendToRust('SettingsChanged', { field: f.field, value: el.value });
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
      sendToRust('SettingsChanged', { field: t.field, value: el.checked });
    });
  });

  // ESC → close
  document.addEventListener('keydown', function(e) {
    if (e.key === 'Escape') {
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
