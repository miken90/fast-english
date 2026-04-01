/**
 * Fast English — Shared icon renderer
 */
'use strict';

(function() {
  var ICON_PATHS = {
    close: '<path d="M18 6L6 18M6 6l12 12"/>',
    back: '<path d="M15 19l-7-7 7-7"/>',
    swap: '<path d="M7 16V4m0 0L3 8m4-4l4 4"/><path d="M17 8v12m0 0l4-4m-4 4l-4-4"/>',
    arrow_right: '<path d="M9 5l7 7-7 7"/>',
    chevron_down: '<path d="M6 9l6 6 6-6"/>',
    divider_down: '<path d="M12 5v14M5 12l7 7 7-7"/>',
    enhance_writing: '<path d="M13 2L3 14h9l-1 8 10-12h-9l1-8z"/>',
    fix_grammar: '<path d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"/>',
    convert_prompt: '<path d="M4 19.5V4a2 2 0 012-2h8l6 6v11.5a2.5 2.5 0 01-2.5 2.5h-11A2.5 2.5 0 014 19.5z"/><path d="M14 2v6h6"/><path d="M8 13h8M8 17h5"/>',
    translate: '<path d="M3 5h12M9 3v2m1.048 9.5A18.022 18.022 0 016.412 9m6.088 9h7M11 21l5-10 5 10M12.751 5C11.783 10.77 8.07 15.61 3 18.129"/>',
    info: '<circle cx="12" cy="12" r="10"/><path d="M12 8v4M12 16h.01"/>',
    refresh: '<path d="M1 4v6h6"/><path d="M3.51 15a9 9 0 102.13-9.36L1 10"/>'
  };

  function renderIcon(name, size, strokeWidth) {
    var iconName = name || 'info';
    var iconSize = size || 16;
    var width = strokeWidth || 2;
    var path = ICON_PATHS[iconName] || ICON_PATHS.info;
    return '<svg width="' + iconSize + '" height="' + iconSize + '" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="' + width + '">' + path + '</svg>';
  }

  function hydrateStaticIcons() {
    var nodes = document.querySelectorAll('[data-icon-name]');
    nodes.forEach(function(node) {
      var name = node.getAttribute('data-icon-name') || 'info';
      var size = Number(node.getAttribute('data-icon-size') || '16');
      var stroke = Number(node.getAttribute('data-icon-stroke') || '2.5');
      node.innerHTML = renderIcon(name, size, stroke);
    });
  }

  window.renderIcon = renderIcon;
  window.hydrateStaticIcons = hydrateStaticIcons;
})();
