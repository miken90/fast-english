/**
 * Fast English — word-level diff
 *
 * Pure logic: takes two strings, returns the edit script between them. No DOM,
 * no escaping, no styling — the caller renders. Exposed as window.diffWords.
 */
'use strict';

(function() {
  /**
   * Largest run of differing words this will diff, per side, after the common
   * prefix and suffix have been trimmed. The LCS table is quadratic, so an
   * unbounded input would freeze the WebView; past this the caller is told to
   * fall back to plain output instead.
   */
  var MAX_DIFF_TOKENS = 800;

  /**
   * Split into words and the whitespace between them, keeping both.
   *
   * Splitting on whitespace runs only is what makes this safe for Vietnamese:
   * `\w` and `\b` are ASCII-only in JavaScript and would cut words like "tiếng"
   * apart, and any per-character split risks separating a combining mark from
   * the letter it belongs to. Whitespace is never part of a grapheme, so this
   * boundary cannot land inside one.
   */
  function tokenize(text) {
    if (!text) return [];
    return text.split(/(\s+)/).filter(function(token) {
      return token.length > 0;
    });
  }

  /**
   * Same text typed two ways — precomposed "ế" versus "e" plus a combining
   * acute — must compare equal, or every accented word shows as a change.
   */
  function normalize(text) {
    var s = String(text == null ? '' : text);
    return s.normalize ? s.normalize('NFC') : s;
  }

  /** Append to the last op when it has the same type, so runs stay whole. */
  function pushOp(ops, type, text) {
    var last = ops.length > 0 ? ops[ops.length - 1] : null;
    if (last && last.type === type) {
      last.text += text;
      return;
    }
    ops.push({ type: type, text: text });
  }

  /**
   * Longest common subsequence over the differing middle, walked back into an
   * edit script.
   */
  function diffMiddle(a, b, ops) {
    var n = a.length;
    var m = b.length;
    var width = m + 1;
    // Uint32Array rather than nested arrays: one flat allocation, and the
    // counts here cannot exceed the token count anyway.
    var table = new Uint32Array((n + 1) * width);

    for (var i = n - 1; i >= 0; i--) {
      for (var j = m - 1; j >= 0; j--) {
        if (a[i] === b[j]) {
          table[i * width + j] = table[(i + 1) * width + (j + 1)] + 1;
        } else {
          var down = table[(i + 1) * width + j];
          var right = table[i * width + (j + 1)];
          table[i * width + j] = down > right ? down : right;
        }
      }
    }

    var x = 0;
    var y = 0;
    while (x < n && y < m) {
      if (a[x] === b[y]) {
        pushOp(ops, 'equal', a[x]);
        x++;
        y++;
      } else if (table[(x + 1) * width + y] >= table[x * width + (y + 1)]) {
        pushOp(ops, 'delete', a[x]);
        x++;
      } else {
        pushOp(ops, 'insert', b[y]);
        y++;
      }
    }
    while (x < n) {
      pushOp(ops, 'delete', a[x]);
      x++;
    }
    while (y < m) {
      pushOp(ops, 'insert', b[y]);
      y++;
    }
  }

  /**
   * Diff `source` against `output` at word level.
   *
   * Returns an array of `{type: 'equal'|'insert'|'delete', text}` covering both
   * strings in order, or null when the change is too large to diff.
   */
  window.diffWords = function(source, output) {
    var a = tokenize(normalize(source));
    var b = tokenize(normalize(output));

    // A grammar fix changes a handful of words in an otherwise identical text,
    // so trimming the shared ends first is what keeps the quadratic step small.
    var start = 0;
    while (start < a.length && start < b.length && a[start] === b[start]) {
      start++;
    }
    var endA = a.length;
    var endB = b.length;
    while (endA > start && endB > start && a[endA - 1] === b[endB - 1]) {
      endA--;
      endB--;
    }

    var midA = a.slice(start, endA);
    var midB = b.slice(start, endB);
    if (midA.length > MAX_DIFF_TOKENS || midB.length > MAX_DIFF_TOKENS) {
      return null;
    }

    var ops = [];
    var k;
    for (k = 0; k < start; k++) {
      pushOp(ops, 'equal', a[k]);
    }
    if (midA.length > 0 || midB.length > 0) {
      diffMiddle(midA, midB, ops);
    }
    for (k = endA; k < a.length; k++) {
      pushOp(ops, 'equal', a[k]);
    }

    return ops;
  };
})();
