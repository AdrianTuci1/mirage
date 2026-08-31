/*
 * Mirage design -> Penpot builder (local source of truth).
 *
 * Builds the spotlight and clipboard windows on the Penpot page
 * "Mirage . Spotlight" (file driven through the Penpot MCP plugin). Values
 * mirror src/client-kmp/src/jvmMain/kotlin/ui/theme/MirageTokens.kt,
 * SearchScreen.kt and ClipboardHistoryScreen.kt.
 *
 * Workflow: the file is split into `// ==== CHUNK <name>` sections. Each
 * section only redefines keys on `storage.mirageLib`, so after an edit only the
 * changed section is sent through `penpot__execute_code`; `storage` keeps the
 * helpers, the token maps and the ids of the generated boards between calls.
 *
 *   build  L.buildBoard(L.spotlightNode(SPOT), x, y, 'dark')
 *   swap   L.replaceBoard(storage.winClipDark, L.clipboardNode(CLIP), 1600, 0, 'dark')
 *
 * Penpot quirks encoded here:
 *  - token names use dots ("color.bg"); "/" is rejected, and a name may not be
 *    both a leaf and a prefix ("color.border" blocks "color.border.input").
 *  - token set names may not contain "/".
 *  - shadow tokens cannot carry alpha, so window shadows are set per shape.
 *  - flex recomputes only once the children exist: fixLayout() re-applies the
 *    flex config and the per-child sizing in a second pass.
 *  - a text created through the plugin keeps a bogus frame height (line box
 *    times ~12 lines) which inflates flex parents and pushes content outside
 *    the board, where exports clip it. finalize() pins verticalAlign "top" and
 *    re-measures the texts twice.
 *  - assigning `characters` back to a text wipes its rendering: never touch it.
 *  - a board with no explicit fill inherits white, so nodes default to fillNone.
 *  - `lineHeight` is a unitless multiplier of the font size, not px: "18" makes
 *    an 18x line box, the glyphs land outside the frame and vanish from exports.
 *    FixLH repairs boards built before that was known.
 *  - a flex board only picks up its height when the node also carries a width
 *    (`if (node.w && node.h) resize`), so fill-width rows state an explicit w.
 *  - path data may not use shorthand arc flags ("a.996.996 0 00-1.41 0"): the
 *    shape is created with an empty path and renders as nothing.
 *  - `storage` is per plugin session: a reload or a detached file wipes it, so
 *    re-send bootstrap + palette + lib + specs + settings + settings2 + data and
 *    restore the board ids from the page before building anything.
 */

// ========================================================= CHUNK bootstrap
// Prefix for every fragment sent through penpot__execute_code; `storage` keeps
// L across calls, so a fragment can assume the chunks before it are loaded.
const L = storage.mirageLib || (storage.mirageLib = {});
L.errors = L.errors || [];
L.created = L.created || [];

// ============================================================== CHUNK palette
// Light values are the MirageTokens colors; the dark set is a neutral gray ramp
// (no purple accent) until the real dark palette is tuned. Boards built with a
// `variant` get explicit hex, so light and dark can sit on one canvas; the
// light board is the token-bound one.
L.HEX = {
  light: {
    'color.bg': '#FFFFFF',
    'color.text.primary': '#000000',
    'color.text.secondary': '#6B7280',
    'color.border': '#E5E7EB',
    'color.input.border': '#000000',
    'color.selected.bg': '#EDE9FE',
    'color.selected.bgStrong': '#DDD6FE',
    'color.key.bg': '#F3F4F6',
    'color.key.text': '#374151',
    'color.hover.bg': '#F9FAFB',
    'color.progress.idle': '#9CA3AF',
    'color.progress.active': '#EAB308',
    'color.progress.done': '#22C55E'
  },
  dark: {
    'color.bg': '#18181A',
    'color.text.primary': '#FFFFFF',
    'color.text.secondary': '#98989D',
    'color.border': '#2E2E32',
    'color.input.border': '#48484D',
    'color.selected.bg': '#38383D',
    'color.selected.bgStrong': '#4A4A50',
    'color.key.bg': '#262626',
    'color.key.text': '#C8C8CD',
    'color.hover.bg': '#202024',
    'color.progress.idle': '#6E6E73',
    'color.progress.active': '#EAB308',
    'color.progress.done': '#22C55E'
  }
};
L.HEX.dark['color.key.bg'] = '#26262A';

// Material icon outlines on a 24x24 grid, scaled by L.scaleD.
L.ICONS = {
  folder: 'M10 4H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2h-8l-2-2z',
  document: 'M6 2c-1.1 0-1.99.9-1.99 2L4 20c0 1.1.89 2 1.99 2H18c1.1 0 2-.9 2-2V8l-6-6H6zm2 12H8v-2h2v2zm0-4H8v-2h2v2zm8 8H8v-2h8v2zm0-4H8v-2h8v2zm-3-5V3.5L18.5 9H13z',
  file: 'M6 2c-1.1 0-1.99.9-1.99 2L4 20c0 1.1.89 2 1.99 2H18c1.1 0 2-.9 2-2V8l-6-6H6zm7 7V3.5L18.5 9H13z',
  image: 'M21 19V5c0-1.1-.9-2-2-2H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2zM8.5 13.5l2.5 3.01L14.5 12l4.5 6H5l3.5-4.5z',
  movie: 'M18 4l2 4h-3l-2-4h-2l2 4h-3l-2-4H8l2 4H7L5 4H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V4h-4z',
  cloud: 'M19.35 10.04C18.67 6.59 15.64 4 12 4 9.11 4 6.6 5.64 5.35 8.04 2.34 8.36 0 10.91 0 14c0 3.31 2.69 6 6 6h13c2.76 0 5-2.24 5-5 0-2.56-2.04-4.65-4.65-4.96z',
  storage: 'M2 20h20v-4H2v4zm2-3h2v2H4v-2zM2 4v4h20V4H2zm4 3H4V5h2v2zm-4 7h20v-4H2v4zm2-3h2v2H4v-2z',
  close: 'M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z',
  add: 'M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z',
  // Penpot's path parser rejects the shorthand arc flags in the stock Material
  // pencil glyph, so this variant spells the corners out with cubic curves.
  edit: 'M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04c.39-.39.39-1.02 0-1.41l-2.34-2.34c-.39-.39-1.02-.39-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z',
  delete: 'M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z',
  settings: 'M19.43 12.98c.04-.32.07-.64.07-.98s-.03-.66-.07-.98l2.11-1.65c.19-.15.24-.42.12-.64l-2-3.46c-.12-.22-.39-.3-.61-.22l-2.49 1c-.52-.4-1.08-.73-1.69-.98l-.38-2.65C18.43 2.18 18.19 2 17.92 2h-4c-.27 0-.51.18-.53.46l-.38 2.65c-.61.25-1.17.59-1.69.98l-2.49-1c-.23-.09-.49 0-.61.22l-2 3.46c-.13.22-.07.49.12.64l2.11 1.65c-.04.32-.07.65-.07.98s.03.66.07.98l-2.11 1.65c-.19.15-.24.42-.12.64l2 3.46c.12.22.39.3.61.22l2.49-1c.52.4 1.08.73 1.69.98l.38 2.65c.02.28.26.46.53.46h4c.27 0 .51-.18.53-.46l.38-2.65c.61-.25 1.17-.59 1.69-.98l2.49 1c.23.09.49 0 .61-.22l2-3.46c.12-.22.07-.49-.12-.64l-2.11-1.65zM12 15.5c-1.93 0-3.5-1.57-3.5-3.5s1.57-3.5 3.5-3.5 3.5 1.57 3.5 3.5-1.57 3.5-3.5 3.5z',
  tune: 'M3 17v2h6v-2H3zM3 5v2h10V5H3zm10 16v-2h8v-2h-8v-2h-2v6h2zM7 9v2H3v2h4v2h2V9H7zm14 4v-2H11v2h10zm-6-4h2V7h4V5h-4V3h-2v6z'
};

// ================================================================ CHUNK lib
// Node schema: { n:name, t:'board'|'rect'|'text'|'icon'|'path'|'ellipse',
//   w,h, hs/vs:'fix'|'auto'|'fill', r:{tok,px}, fill:'color.x', fillNone,
//   stroke:{tok,w}, shadows:[{ox,oy,blur,spread,opacity}], sizeToks:{w,h},
//   flex:{dir,gap,colGap,align,justify,hs,vs,pad,padH,padV,padT,padB},
//   padTok,gapTok,colGapTok, mb,mt,absolute, text:{chars,size,weight,lh,typo,color},
//   icon:{name,size,color}, children:[...] }

L.tok = function (name) { return storage.T[name]; };
L.colorOf = function (name, variant) { return (L.HEX[variant] || L.HEX.light)[name]; };
L.sleep = function (ms) { return new Promise(function (r) { setTimeout(r, ms); }); };
L.scaleD = function (d, s) {
  return d.replace(/-?\d*\.?\d+(e[-+]?\d+)?/gi, function (m) { return String(Math.round(parseFloat(m) * s * 1000) / 1000); });
};
L.arcD = function (cx, cy, r, a0, a1) {
  const p = function (a) { return [Math.round((cx + r * Math.cos(a)) * 1000) / 1000, Math.round((cy + r * Math.sin(a)) * 1000) / 1000]; };
  const s = p(a0), e = p(a1);
  const large = a1 - a0 > Math.PI ? 1 : 0;
  return 'M' + s[0] + ',' + s[1] + 'A' + r + ',' + r + ' 0 ' + large + ' 1 ' + e[0] + ',' + e[1];
};

L.setFill = function (shape, tokName, variant) {
  const hex = this.colorOf(tokName, variant);
  shape.fills = hex ? [{ fillColor: hex, fillOpacity: 1 }] : [];
  if (!variant && storage.T[tokName]) shape.applyToken(storage.T[tokName], ['fill']);
};
L.setStroke = function (shape, tokName, width, variant) {
  const hex = this.colorOf(tokName, variant);
  shape.strokes = [{ strokeColor: hex, strokeOpacity: 1, strokeWidth: width, strokeAlignment: 'inner' }];
  if (!variant && storage.T[tokName]) shape.applyToken(storage.T[tokName], ['strokeColor']);
};
L.setRadius = function (shape, tokName, px, variant) {
  shape.borderRadius = px;
  if (!variant && storage.T[tokName]) {
    shape.applyToken(storage.T[tokName], ['borderRadiusTopLeft', 'borderRadiusTopRight', 'borderRadiusBottomLeft', 'borderRadiusBottomRight']);
  }
};

L.makeFlex = function (board, f) {
  const fl = penpotUtils.addFlexLayout(board, f.dir || 'row');
  if (f.gap !== undefined) fl.rowGap = f.gap;
  if (f.colGap !== undefined) fl.columnGap = f.colGap;
  if (f.align) fl.alignItems = f.align;
  if (f.justify) fl.justifyContent = f.justify;
  if (f.pad !== undefined) { fl.topPadding = f.pad; fl.rightPadding = f.pad; fl.bottomPadding = f.pad; fl.leftPadding = f.pad; }
  if (f.padH !== undefined) { fl.rightPadding = f.padH; fl.leftPadding = f.padH; }
  if (f.padV !== undefined) { fl.topPadding = f.padV; fl.bottomPadding = f.padV; }
  if (f.padT !== undefined) fl.topPadding = f.padT;
  if (f.padB !== undefined) fl.bottomPadding = f.padB;
  if (f.padL !== undefined) fl.leftPadding = f.padL;
  if (f.padR !== undefined) fl.rightPadding = f.padR;
  fl.horizontalSizing = f.hs || 'fix';
  fl.verticalSizing = f.vs || 'auto';
  return fl;
};

// Recursive node -> shape. `variant` ('dark') opts out of token bindings.
L.build = async function (node, parent, variant) {
  let s;
  try {
    if (node.t === 'text') {
      s = penpot.createText(node.text.chars);
      s.name = node.n;
      s.fontFamily = 'Inter';
      s.fontStyle = 'normal';
      s.fontSize = node.text.size;
      s.fontWeight = String(node.text.weight || 400);
      // Penpot keeps line-height as a unitless multiplier of the font size, while
      // the Compose spec is in px. Passing "18" builds an 18x line box, which
      // pushes the glyphs out of the frame and makes the text invisible.
      const lhPx = node.text.lh || Math.round(node.text.size * 1.35);
      s.lineHeight = (lhPx / node.text.size).toFixed(4);
      this.setFill(s, node.text.color, variant);
      if (!variant) {
        if (node.text.typo) s.applyToken(this.tok(node.text.typo), ['typography']);
        if (node.text.color) s.applyToken(this.tok(node.text.color), ['fill']);
      }
    } else if (node.t === 'icon') {
      s = penpot.createPath();
      s.name = node.n;
      s.d = this.scaleD(this.ICONS[node.icon.name], node.icon.size / 24);
      this.setFill(s, node.icon.color, variant);
      if (!variant && node.icon.color) s.applyToken(this.tok(node.icon.color), ['fill']);
    } else if (node.t === 'path') {
      s = penpot.createPath();
      s.name = node.n;
      s.d = node.path.d;
      s.fills = [];
      if (node.path.stroke) s.strokes = [{ strokeColor: this.colorOf(node.path.stroke.tok, variant), strokeOpacity: 1, strokeWidth: node.path.stroke.w, strokeAlignment: 'center' }];
    } else {
      if (!node.fill && !node.fillNone) node.fillNone = true;
      s = node.t === 'board' ? penpot.createBoard() : (node.t === 'ellipse' ? penpot.createEllipse() : penpot.createRectangle());
      s.name = node.n;
      if (node.w && node.h) s.resize(node.w, node.h);
      if (node.fillNone) s.fills = []; else if (node.fill) this.setFill(s, node.fill, variant);
      if (node.stroke) this.setStroke(s, node.stroke.tok, node.stroke.w, variant);
      if (node.r) this.setRadius(s, node.r.tok, node.r.px, variant);
      if (node.shadows) s.shadows = node.shadows.map(function (sh) { return { shadowColor: sh.color || '#000000', shadowOpacity: sh.opacity, offsetX: sh.ox, offsetY: sh.oy, blur: sh.blur, spread: sh.spread || 0, shadowStyle: 'drop-shadow' }; });
      if (node.flex) {
        this.makeFlex(s, node.flex);
        if (!variant && node.padTok) s.applyToken(this.tok(node.padTok), ['paddingTop', 'paddingRight', 'paddingBottom', 'paddingLeft']);
        if (!variant && node.padTopTok) s.applyToken(this.tok(node.padTopTok), ['paddingTop']);
        if (!variant && node.gapTok && node.flex.gap !== undefined) s.applyToken(this.tok(node.gapTok), ['rowGap']);
        if (!variant && node.colGapTok && node.flex.colGap !== undefined) s.applyToken(this.tok(node.colGapTok), ['columnGap']);
      }
    }
    if (node.sizeToks && !variant) {
      if (node.sizeToks.w) s.applyToken(this.tok(node.sizeToks.w), ['width']);
      if (node.sizeToks.h) s.applyToken(this.tok(node.sizeToks.h), ['height']);
    }
    if (parent) {
      parent.appendChild(s);
      const lc = s.layoutChild;
      if (lc) {
        lc.horizontalSizing = node.hs || 'auto';
        lc.verticalSizing = node.vs || 'auto';
        if (node.absolute) lc.absolute = true;
        if (node.mt !== undefined) lc.topMargin = node.mt;
        if (node.mb !== undefined) lc.bottomMargin = node.mb;
      }
    }
    if (node.children) { for (const c of node.children) await this.build(c, s, variant); }
    if (s) L.created.push({ node: node, id: s.id });
    return s;
  } catch (e) {
    L.errors.push(node.n + ': ' + e.message.slice(0, 140));
    return null;
  }
};

// Second pass: flex is only computed once children exist.
L.fixLayout = async function (root) {
  const errs = [];
  for (const c of L.created) {
    try {
      const s = penpotUtils.findShapeById(c.id);
      if (!s || !c.node.flex || !s.flex) continue;
      const f = c.node.flex, fl = s.flex;
      fl.dir = f.dir || 'row';
      if (f.gap !== undefined) fl.rowGap = f.gap;
      if (f.colGap !== undefined) fl.columnGap = f.colGap;
      fl.alignItems = f.align || 'start';
      fl.justifyContent = f.justify || 'start';
      if (f.pad !== undefined) { fl.topPadding = f.pad; fl.rightPadding = f.pad; fl.bottomPadding = f.pad; fl.leftPadding = f.pad; }
      if (f.padH !== undefined) { fl.rightPadding = f.padH; fl.leftPadding = f.padH; }
      if (f.padV !== undefined) { fl.topPadding = f.padV; fl.bottomPadding = f.padV; }
      if (f.padT !== undefined) fl.topPadding = f.padT;
      fl.horizontalSizing = f.hs || 'fix';
      fl.verticalSizing = f.vs || 'auto';
    } catch (e) { errs.push('flex ' + c.node.n + ': ' + e.message.slice(0, 80)); }
  }
  await L.sleep(400);
  const tx = await L.fixTexts(root);
  errs.push.apply(errs, tx.errs);
  await L.sleep(300);
  for (const c of L.created) {
    try {
      const s = penpotUtils.findShapeById(c.id);
      if (!s || !s.parent || !s.layoutChild) continue;
      s.layoutChild.horizontalSizing = c.node.hs || 'auto';
      s.layoutChild.verticalSizing = c.node.vs || 'auto';
    } catch (e) { errs.push('child ' + c.node.n + ': ' + e.message.slice(0, 80)); }
  }
  await L.sleep(700);
  return errs;
};

// Pin every text to the top of its frame and re-measure. Two traps:
//  - a text is created with a bogus frame height (line box x ~12 lines), which
//    inflates flex parents and pushes rows outside the board, where exports
//    clip them;
//  - textBounds collapses to ~1px while the Inter metrics are still loading,
//    and a text pinned at that width keeps reporting it, so every pass has to
//    release the frame back to auto-width before measuring.
L.fixTexts = async function (root) {
  const errs = [];
  const texts = penpotUtils.findShapes(function (s) { return s.type === 'text'; }, root);
  for (const t of texts) {
    try { t.growType = 'auto-width'; } catch (e) { errs.push('grow ' + t.name + ': ' + e.message.slice(0, 60)); }
  }
  await L.sleep(600);
  for (const t of texts) {
    try {
      let tb = t.textBounds;
      for (let i = 0; i < 6 && tb.width <= 1 && (t.characters || '').length; i++) {
        await L.sleep(500);
        tb = t.textBounds;
      }
      const lh = parseFloat(t.fontSize) * (parseFloat(t.lineHeight) || 1.35);
      t.verticalAlign = 'top';
      t.growType = 'fixed';
      t.resize(Math.ceil(tb.width) + 1, Math.max(Math.ceil(tb.height), Math.ceil(lh)));
    } catch (e) { errs.push(t.name + ': ' + e.message.slice(0, 60)); }
  }
  await L.sleep(400);
  return { count: texts.length, errs: errs };
};

L.finalize = async function (root) {
  let res = await L.fixTexts(root);
  await L.sleep(700);
  res = await L.fixTexts(root);
  await L.sleep(700);
  return res;
};

L.buildBoard = async function (node, x, y, variant) {
  L.errors = []; L.created = [];
  const board = await L.build(node, null, variant);
  if (!board) return { failed: true, errors: L.errors };
  const fixErrs = await L.fixLayout(board);
  board.x = x; board.y = y;
  await L.sleep(400);
  const f = await L.finalize(board);
  return { id: board.id, name: board.name, w: Math.round(board.width), h: Math.round(board.height), errors: L.errors.concat(fixErrs).concat(f.errs), texts: f.count };
};

// Split build/finalize so one execute_code call stays under the MCP gateway
// timeout; a retry of buildFast drops the board it just made instead of leaving
// a duplicate behind.
L.buildFast = async function (node, x, y, variant) {
  L.errors = []; L.created = [];
  penpot.currentPage.root.children
    .filter(function (c) { return c.name === node.n; })
    .forEach(function (c) { try { c.remove(); } catch (e) {} });
  const board = await L.build(node, null, variant);
  if (!board) return { failed: true, errors: L.errors };
  const errs = await L.fixLayout(board);
  board.x = x; board.y = y;
  storage.pending = { id: board.id, name: board.name };
  return { id: board.id, name: board.name, w: Math.round(board.width), h: Math.round(board.height), errors: errs.slice(0, 8) };
};

L.finishPending = async function () {
  var p = storage.pending;
  if (!p) return { err: 'nothing pending' };
  var b = penpotUtils.findShapeById(p.id);
  var f = await L.finalize(b);
  storage.pending = null;
  var narrow = penpotUtils.findShapes(function (s) { return s.type === 'text' && s.width <= 2; }, b).length;
  return { id: p.id, name: p.name, w: Math.round(b.width), h: Math.round(b.height), texts: f.count, narrow: narrow, errs: f.errs.slice(0, 8) };
};

// Penpot only computes text metrics inside the render loop, so a hidden browser
// tab leaves every fresh text at 1x1 with an empty textBounds. The work is
// therefore split into micro-steps that each return in well under the ~10s the
// MCP gateway allows: build -> stepFlex -> stepRelease -> stepMeasure.
L.stepFlex = function (id) {
  var b = penpotUtils.findShapeById(id);
  var done = 0;
  for (const c of (L.created || [])) {
    try {
      const s = penpotUtils.findShapeById(c.id);
      if (!s || !c.node.flex || !s.flex) continue;
      const f = c.node.flex, fl = s.flex;
      fl.dir = f.dir || 'row';
      if (f.gap !== undefined) fl.rowGap = f.gap;
      if (f.colGap !== undefined) fl.columnGap = f.colGap;
      fl.alignItems = f.align || 'start';
      fl.justifyContent = f.justify || 'start';
      if (f.pad !== undefined) { fl.topPadding = f.pad; fl.rightPadding = f.pad; fl.bottomPadding = f.pad; fl.leftPadding = f.pad; }
      if (f.padH !== undefined) { fl.rightPadding = f.padH; fl.leftPadding = f.padH; }
      if (f.padV !== undefined) { fl.topPadding = f.padV; fl.bottomPadding = f.padV; }
      if (f.padT !== undefined) fl.topPadding = f.padT;
      fl.horizontalSizing = f.hs || 'fix';
      fl.verticalSizing = f.vs || 'auto';
      done++;
    } catch (e) {}
  }
  for (const c of (L.created || [])) {
    try {
      const s = penpotUtils.findShapeById(c.id);
      if (!s || !s.parent || !s.layoutChild) continue;
      s.layoutChild.horizontalSizing = c.node.hs || 'auto';
      s.layoutChild.verticalSizing = c.node.vs || 'auto';
    } catch (e) {}
  }
  return 'flex applied on ' + done + ' boards';
};

L.stepRelease = function (id) {
  var b = penpotUtils.findShapeById(id);
  var texts = penpotUtils.findShapes(function (s) { return s.type === 'text'; }, b);
  texts.forEach(function (t) { try { t.growType = 'auto-width'; } catch (e) {} });
  return 'released ' + texts.length;
};

// Pins the frames the render loop has already measured; texts whose metrics are
// not there yet are left alone and picked up by the next call.
L.stepMeasure = function (id) {
  var b = penpotUtils.findShapeById(id);
  var texts = penpotUtils.findShapes(function (s) { return s.type === 'text'; }, b);
  var narrow = 0;
  texts.forEach(function (t) {
    try {
      const tb = t.textBounds;
      if (!tb || tb.width <= 1) { narrow++; return; }
      const lh = parseFloat(t.fontSize) * (parseFloat(t.lineHeight) || 1.35);
      t.verticalAlign = 'top';
      t.growType = 'fixed';
      t.resize(Math.ceil(tb.width) + 1, Math.max(Math.ceil(tb.height), Math.ceil(lh)));
    } catch (e) {}
  });
  return JSON.stringify({ texts: texts.length, waiting: narrow, h: Math.round(b.height) });
};

// Repairs boards built before the line-height fix: the shape-level lineHeight
// held px values, which Penpot reads as multipliers, so the glyphs sat far below
// the frame. Rewrites it as the px/fontSize factor and re-pins the frames.
L.fixLH = function (id) {
  var b = penpotUtils.findShapeById(id);
  var texts = penpotUtils.findShapes(function (s) { return s.type === 'text'; }, b);
  var n = 0;
  texts.forEach(function (t) {
    try {
      const fs = parseFloat(t.fontSize);
      const cur = parseFloat(t.lineHeight);
      if (!fs || cur < 4) return;
      const px = L.LH[fs] || Math.round(fs * 1.35);
      t.lineHeight = (px / fs).toFixed(4);
      n++;
    } catch (e) {}
  });
  return 'lineHeight fixed on ' + n + ' texts';
};

// Compose line heights (px) per font size used in the Mirage specs.
L.LH = { 10: 14, 11: 15, 12: 16, 13: 18, 14: 18, 16: 22, 18: 24, 20: 26, 24: 30 };

// Builds one of the L.SETTINGS boards into the current page at its slot.
L.buildSettings = async function (i) {
  L.errors = []; L.created = [];
  var d = Object.assign({}, L.SETTINGS[i], { tabs: L.SETTINGS_TABS });
  var board = await L.build(L.settingsNode(d), null, 'dark');
  board.x = i * 960; board.y = 0;
  storage['set' + d.tab] = board.id;
  return { id: board.id, name: board.name, created: L.created.length, errors: L.errors.slice(0, 5) };
};

// Delete the previous version of a board and rebuild it in place.
L.replaceBoard = async function (id, node, x, y, variant) {
  try {
    const old = penpotUtils.findShapeById(id);
    if (old) old.remove();
  } catch (e) { /* already gone */ }
  return L.buildBoard(node, x, y, variant);
};

// ============================================================== CHUNK specs
//
// Spotlight window (SearchScreen.kt), after the 2026-08 review:
//   - the module status row and the indexing progress bar are dropped; both
//     belong to the onboarding flow instead
//   - the search input keeps a single gray 1px border, no underline, 48dp tall
//   - an empty result list is withdrawn: no divider, no list, no spacer, and
//     the window height hugs its content instead of staying at 480dp
//
// Clipboard window (ClipboardHistoryScreen.kt): the same search input stays on
// top, then a two-column row (history list | 1px divider | preview). Media
// previews are placeholders.

L.CW = 688; // 720 - 2 * 16 window padding

L.keyChip = function (key, padH) {
  return {
    n: 'Key: ' + key, t: 'board', hs: 'auto', vs: 'auto', r: { tok: 'radius.sm', px: 4 }, fill: 'color.key.bg',
    flex: { dir: 'row', align: 'center', justify: 'center', hs: 'auto', vs: 'auto', padH: padH === undefined ? 4 : padH, padV: 2 },
    children: [{ n: 'Key Label', t: 'text', hs: 'auto', vs: 'auto', text: { chars: key, size: 12, weight: 400, lh: 16, typo: 'typography.footer', color: 'color.key.text' } }]
  };
};

L.hint = function (label, key) {
  return {
    n: 'Shortcut Hint: ' + label, t: 'board', hs: 'auto', vs: 'auto',
    flex: { dir: 'row', colGap: 4, align: 'center', hs: 'auto', vs: 'auto' }, colGapTok: 'space.xs',
    children: [
      { n: 'Hint Label', t: 'text', hs: 'auto', vs: 'auto', text: { chars: label, size: 12, weight: 400, lh: 16, typo: 'typography.footer', color: 'color.text.secondary' } },
      this.keyChip(key)
    ]
  };
};

// 48dp tall, gray 1px border on all four sides, no underline.
L.searchInput = function (d) {
  return {
    n: 'Search Input', t: 'board', hs: 'fill', vs: 'fix', w: L.CW, h: 48, r: { tok: 'radius.md', px: 8 },
    fillNone: true, stroke: { tok: 'color.border', w: 1 },
    flex: { dir: 'row', align: 'center', justify: 'start', hs: 'fill', vs: 'fix', padH: 12 },
    children: [{
      n: d.query ? 'Query Text' : 'Placeholder Text', t: 'text', hs: 'auto', vs: 'auto',
      text: { chars: d.query || d.placeholder, size: 18, weight: 400, lh: 24, typo: 'typography.input', color: d.query ? 'color.text.primary' : 'color.text.secondary' }
    }]
  };
};

L.resultRow = function (r, i) {
  const kids = [
    { n: 'File Icon (' + r.icon + ')', t: 'icon', hs: 'fix', vs: 'fix', icon: { name: r.icon, size: 32, color: 'color.text.secondary' } },
    {
      n: 'Result Text', t: 'board', hs: 'fill', vs: 'auto',
      flex: { dir: 'column', gap: 2, align: 'start', hs: 'auto', vs: 'auto' },
      children: [
        { n: 'Result Title', t: 'text', hs: 'auto', vs: 'auto', text: { chars: r.title, size: 14, weight: 500, lh: 18, typo: 'typography.result.title', color: 'color.text.primary' } },
        { n: 'Result Path', t: 'text', hs: 'auto', vs: 'auto', text: { chars: r.path, size: 12, weight: 400, lh: 16, typo: 'typography.result.meta', color: 'color.text.secondary' } }
      ]
    }
  ];
  if (r.cloud) kids.push({ n: 'Source Icon (cloud)', t: 'icon', hs: 'fix', vs: 'fix', icon: { name: 'cloud', size: 16, color: 'color.text.secondary' } });
  kids.push(L.hint('open', '\u21B5'));
  return {
    n: 'Result Row ' + (i + 1) + (r.selected ? ' (selected)' : ''), t: 'board', hs: 'fill', vs: 'fix', w: L.CW, h: 44,
    sizeToks: { h: 'size.result.row' }, r: { tok: 'radius.md', px: 8 },
    fill: r.selected ? 'color.selected.bg' : undefined, fillNone: !r.selected,
    flex: { dir: 'row', colGap: 12, align: 'center', hs: 'fill', vs: 'fix', padH: 12 }, colGapTok: 'space.md',
    children: kids
  };
};

L.sourceFilter = function (f) {
  return {
    n: 'Source Filter: ' + f.kind + (f.active ? ' (on)' : ' (off)'), t: 'board', hs: 'fix', vs: 'fix', w: 20, h: 20,
    r: { px: 10 }, fill: f.active ? 'color.selected.bg' : undefined, fillNone: !f.active,
    stroke: f.active ? null : { tok: 'color.border', w: 1.5 },
    flex: { dir: 'row', align: 'center', justify: 'center', hs: 'fix', vs: 'fix' },
    children: [{ n: 'Source Icon', t: 'icon', hs: 'fix', vs: 'fix', icon: { name: f.icon, size: 12, color: f.active ? 'color.text.primary' : 'color.text.secondary' } }]
  };
};

L.footer = function (d) {
  const right = [L.hint('open', '\u21B5')];
  if (d.showDownload) right.push(L.hint('download', 'shift+\u21B5'));
  right.push(L.hint('clipboard', 'tab'));
  right.push({ n: 'Settings Link', t: 'text', hs: 'auto', vs: 'auto', text: { chars: 'settings', size: 12, weight: 400, lh: 16, typo: 'typography.footer', color: 'color.text.secondary' } });
  right.push(L.keyChip('\u2318,', 6));
  return {
    n: 'Search Footer', t: 'board', hs: 'fill', vs: 'auto',
    flex: { dir: 'column', gap: 0, align: 'stretch', hs: 'auto', vs: 'auto' },
    children: [
      { n: 'Divider (footer)', t: 'rect', hs: 'fill', vs: 'fix', w: L.CW, h: 1, fill: 'color.border' },
      {
        n: 'Footer Row', t: 'board', hs: 'fill', vs: 'auto',
        flex: { dir: 'row', justify: 'space-between', align: 'center', hs: 'fill', vs: 'auto', padT: 12 }, padTopTok: 'space.md',
        children: [
          { n: 'Source Type Filters', t: 'board', hs: 'auto', vs: 'auto', flex: { dir: 'row', colGap: 4, align: 'center', hs: 'auto', vs: 'auto' }, colGapTok: 'space.xs', children: d.sources.map(L.sourceFilter) },
          { n: 'Footer Right', t: 'board', hs: 'auto', vs: 'auto', flex: { dir: 'row', colGap: 12, align: 'center', hs: 'auto', vs: 'auto' }, colGapTok: 'space.md', children: right }
        ]
      }
    ]
  };
};

// Outer window board. `fixed` keeps the 480dp height token; the empty state
// hugs its content instead.
L.windowBoard = function (node, fixed) {
  return Object.assign({
    t: 'board', w: 720, h: 480,
    sizeToks: fixed ? { w: 'size.window.width', h: 'size.window.height' } : { w: 'size.window.width' },
    fill: 'color.bg', r: { tok: 'radius.window', px: 16 },
    shadows: [
      { ox: 0, oy: 5, blur: 5, spread: 0, opacity: 0.2 },
      { ox: 0, oy: 8, blur: 10, spread: 1, opacity: 0.14 },
      { ox: 0, oy: 3, blur: 14, spread: 2, opacity: 0.12 }
    ]
  }, node);
};

L.spotlightNode = function (d) {
  const kids = [L.searchInput(d)];
  const fixed = d.results.length > 0;
  if (fixed) {
    kids.push({ n: 'Divider (results)', t: 'rect', hs: 'fill', vs: 'fix', w: L.CW, h: 1, fill: 'color.border' });
    kids.push({ n: 'Results List', t: 'board', hs: 'fill', vs: 'auto', flex: { dir: 'column', gap: 4, align: 'stretch', hs: 'auto', vs: 'auto' }, gapTok: 'space.xs', children: d.results.map(L.resultRow) });
    kids.push({ n: 'Spacer (weight 1f)', t: 'rect', hs: 'fill', vs: 'fill', w: L.CW, h: 40, fillNone: true });
  }
  kids.push(L.footer(d));
  return L.windowBoard({
    n: d.name,
    flex: { dir: 'column', gap: 12, align: 'stretch', hs: 'fix', vs: fixed ? 'fix' : 'auto', pad: 16 },
    padTok: 'space.lg', gapTok: 'space.md',
    h: fixed ? 480 : 128,
    children: kids
  }, fixed);
};

// Clipboard: the search input stays on top, then list | 1px divider | preview.
L.clipboardNode = function (d) {
  const row = function (e, i) {
    return {
      n: 'Clipboard Row ' + (i + 1) + (e.selected ? ' (selected)' : ''), t: 'board', hs: 'fill', vs: 'fix', w: 335, h: 44,
      sizeToks: { h: 'size.result.row' }, r: { tok: 'radius.md', px: 8 },
      fill: e.selected ? 'color.selected.bg' : undefined, fillNone: !e.selected,
      stroke: e.selected ? { tok: 'color.selected.bgStrong', w: 1 } : null,
      flex: { dir: 'row', colGap: 12, align: 'center', hs: 'fill', vs: 'fix', padH: 12 }, colGapTok: 'space.md',
      children: [
        { n: 'Entry Icon (' + e.icon + ')', t: 'icon', hs: 'fix', vs: 'fix', icon: { name: e.icon, size: 32, color: 'color.text.secondary' } },
        {
          n: 'Entry Text', t: 'board', hs: 'fill', vs: 'auto',
          flex: { dir: 'column', gap: 2, align: 'start', hs: 'auto', vs: 'auto' },
          children: [
            { n: 'Entry Label', t: 'text', hs: 'auto', vs: 'auto', text: { chars: e.label, size: 14, weight: 500, lh: 18, typo: 'typography.result.title', color: 'color.text.primary' } },
            { n: 'Entry Time', t: 'text', hs: 'auto', vs: 'auto', text: { chars: e.time, size: 12, weight: 400, lh: 16, typo: 'typography.result.meta', color: 'color.text.secondary' } }
          ]
        }
      ]
    };
  };
  const meta = function (label, value) {
    return {
      n: 'Metadata: ' + label, t: 'board', hs: 'fill', vs: 'auto',
      flex: { dir: 'row', justify: 'space-between', align: 'center', hs: 'fill', vs: 'auto' },
      children: [
        { n: 'Metadata Label', t: 'text', hs: 'auto', vs: 'auto', text: { chars: label, size: 12, weight: 400, lh: 16, typo: 'typography.result.meta', color: 'color.text.secondary' } },
        { n: 'Metadata Value', t: 'text', hs: 'auto', vs: 'auto', text: { chars: value, size: 12, weight: 400, lh: 16, typo: 'typography.result.meta', color: 'color.text.primary' } }
      ]
    };
  };
  return L.windowBoard({
    n: d.name, h: 480,
    flex: { dir: 'column', gap: 12, align: 'stretch', hs: 'fix', vs: 'fix', pad: 16 }, padTok: 'space.lg', gapTok: 'space.md',
    children: [
      L.searchInput(d),
      {
        n: 'Clipboard Body', t: 'board', hs: 'fill', vs: 'fill', w: L.CW, h: 380, fillNone: true,
        flex: { dir: 'row', colGap: 12, align: 'stretch', hs: 'auto', vs: 'fill' }, colGapTok: 'space.md',
        children: [
          {
            n: 'Clipboard List Column', t: 'board', hs: 'fill', vs: 'fill', w: 335, h: 380, fillNone: true,
            flex: { dir: 'column', gap: 4, align: 'stretch', hs: 'auto', vs: 'fill' }, gapTok: 'space.xs',
            children: [{ n: 'Section Title (history)', t: 'text', hs: 'auto', vs: 'auto', mb: 8, text: { chars: 'Clipboard history', size: 18, weight: 500, lh: 24, typo: 'typography.section.title', color: 'color.text.primary' } }]
              .concat(d.entries.map(row))
          },
          { n: 'Divider (vertical)', t: 'rect', hs: 'fix', vs: 'fill', w: 1, h: 380, fill: 'color.border' },
          {
            n: 'Preview Column', t: 'board', hs: 'fill', vs: 'fill', w: 335, h: 380, fillNone: true,
            flex: { dir: 'column', gap: 12, align: 'stretch', hs: 'auto', vs: 'fill' }, gapTok: 'space.md',
            children: [
              { n: 'Section Title (preview)', t: 'text', hs: 'auto', vs: 'auto', mb: 8, text: { chars: 'Preview', size: 18, weight: 500, lh: 24, typo: 'typography.section.title', color: 'color.text.primary' } },
              {
                n: 'Media Preview (placeholder)', t: 'board', hs: 'fill', vs: 'fill', w: 335, h: 200,
                r: { tok: 'radius.md', px: 8 }, stroke: { tok: 'color.border', w: 1 }, fillNone: true,
                flex: { dir: 'column', gap: 8, align: 'center', justify: 'center', hs: 'auto', vs: 'fill', pad: 12 },
                children: [
                  { n: 'Media Placeholder Icon', t: 'icon', hs: 'fix', vs: 'fix', icon: { name: d.previewIcon, size: 32, color: 'color.text.secondary' } },
                  { n: 'Media Placeholder Label', t: 'text', hs: 'auto', vs: 'auto', text: { chars: d.previewLabel, size: 12, weight: 400, lh: 16, typo: 'typography.result.meta', color: 'color.text.secondary' } }
                ]
              },
              meta('Type', d.previewType),
              meta('Size', d.previewSize),
              meta('Copied at', d.copiedAt)
            ]
          }
        ]
      }
    ]
  }, true);
};

// =============================================================== CHUNK tokens
// ============================================================= CHUNK settings
// Settings window (SettingsWindow.kt): padding 16, header with the four
// undecorated tabs (the selected one gets a 2px underline in
// color.selected.bgStrong) and a close button, then a column of rows spaced by
// 12 with 1px dividers. The code opens it at 720x560; as a standalone window it
// is designed larger (880x640) so the rows and the connectors list breathe.

// Settings window: a standalone app window, so it is allowed to be larger than
// the 720x480 spotlight. Content width = 880 - 2 * 16 padding.
L.SW = 880;
L.SH = 640;
L.SWC = 848;

// title + description, stacked with the 2dp gap used across the window
L.settingLabel = function (title, desc) {
  return {
    n: 'Setting Label: ' + title, t: 'board', hs: 'fill', vs: 'auto',
    flex: { dir: 'column', gap: 2, align: 'start', hs: 'auto', vs: 'auto' },
    children: [
      { n: 'Setting Title', t: 'text', hs: 'auto', vs: 'auto', text: { chars: title, size: 14, weight: 500, lh: 18, color: 'color.text.primary' } },
      { n: 'Setting Description', t: 'text', hs: 'auto', vs: 'auto', text: { chars: desc, size: 12, weight: 400, lh: 16, color: 'color.text.secondary' } }
    ]
  };
};

// Material 3 switch: 52x32 track, 24dp thumb when on, 20dp when off.
L.settingSwitch = function (on) {
  const thumb = on ? 24 : 20;
  return {
    n: 'Switch' + (on ? ' (on)' : ' (off)'), t: 'board', hs: 'fix', vs: 'fix', w: 52, h: 32, r: { px: 16 },
    fill: on ? 'color.selected.bgStrong' : 'color.key.bg',
    stroke: on ? null : { tok: 'color.border', w: 1 },
    flex: { dir: 'row', align: 'center', justify: on ? 'end' : 'start', hs: 'fix', vs: 'fix', padH: on ? 0 : 6 },
    children: [{ n: 'Thumb', t: 'ellipse', hs: 'fix', vs: 'fix', w: thumb, h: thumb, fill: on ? 'color.bg' : 'color.text.secondary' }]
  };
};

L.smallButton = function (label, fillTok, padH, padV, icon) {
  const kids = [];
  if (icon) kids.push({ n: 'Button Icon', t: 'icon', hs: 'fix', vs: 'fix', icon: { name: icon, size: 16, color: 'color.text.primary' } });
  kids.push({ n: 'Button Label', t: 'text', hs: 'auto', vs: 'auto', text: { chars: label, size: label.length > 20 ? 12 : 14, weight: 400, lh: 18, color: 'color.text.primary' } });
  return {
    n: 'Button: ' + label, t: 'board', hs: 'auto', vs: 'auto', r: { tok: 'radius.sm', px: 4 }, fill: fillTok,
    flex: { dir: 'row', colGap: 8, align: 'center', justify: 'center', hs: 'auto', vs: 'auto', padH: padH, padV: padV }, colGapTok: 'space.sm',
    children: kids
  };
};

L.settingRow = function (r) {
  // module row: name + status (+ optional download button) over a progress bar
  if (r.progress !== undefined) {
    const status = [
      { n: 'Status Label', t: 'text', hs: 'auto', vs: 'auto', text: { chars: r.status, size: 12, weight: 400, lh: 16, color: r.status === 'Ready' ? 'color.text.primary' : 'color.text.secondary' } }
    ];
    if (r.action) status.push(L.smallButton(r.action, 'color.selected.bgStrong', 8, 2));
    if (r.cancel) status.push({ n: 'Cancel Link', t: 'text', hs: 'auto', vs: 'auto', text: { chars: 'Cancel', size: 12, weight: 400, lh: 16, color: 'color.text.secondary' } });
    return {
      n: 'Setting Row: ' + r.title, t: 'board', hs: 'fill', vs: 'auto',
      flex: { dir: 'column', gap: 8, align: 'stretch', hs: 'auto', vs: 'auto' }, gapTok: 'space.sm',
      children: [
        {
          n: 'Module Row', t: 'board', hs: 'fill', vs: 'auto',
          flex: { dir: 'row', colGap: 8, justify: 'space-between', align: 'center', hs: 'fill', vs: 'auto' }, colGapTok: 'space.sm',
          children: [
            { n: 'Module Name', t: 'text', hs: 'auto', vs: 'auto', text: { chars: r.title, size: 14, weight: 500, lh: 18, color: 'color.text.primary' } },
            { n: 'Module Status', t: 'board', hs: 'auto', vs: 'auto', flex: { dir: 'row', colGap: 8, align: 'center', hs: 'auto', vs: 'auto' }, colGapTok: 'space.sm', children: status }
          ]
        },
        {
          n: 'Progress Track', t: 'board', hs: 'fill', vs: 'fix', w: L.SWC, h: 4, r: { px: 2 }, fill: 'color.key.bg',
          flex: { dir: 'row', align: 'center', justify: 'start', hs: 'fill', vs: 'fix' },
          children: [{ n: 'Progress Fill', t: 'rect', hs: 'fix', vs: 'fix', w: Math.round(L.SWC * r.progress), h: 4, r: { px: 2 }, fill: 'color.selected.bgStrong' }]
        }
      ]
    };
  }
  const kids = [L.settingLabel(r.title, r.desc)];
  if (r.input !== undefined) {
    kids.push({
      n: 'Text Field', t: 'board', hs: 'fill', vs: 'fix', w: L.SWC, h: 48, r: { tok: 'radius.sm', px: 4 },
      fillNone: true, stroke: { tok: 'color.border', w: 1 },
      flex: { dir: 'row', align: 'center', hs: 'fill', vs: 'fix', padH: 12 },
      children: [{ n: 'Field Placeholder', t: 'text', hs: 'auto', vs: 'auto', text: { chars: r.input, size: 14, weight: 400, lh: 18, color: 'color.text.secondary' } }]
    });
    return {
      n: 'Setting Row: ' + r.title, t: 'board', hs: 'fill', vs: 'auto',
      flex: { dir: 'column', gap: 8, align: 'stretch', hs: 'auto', vs: 'auto' }, gapTok: 'space.sm',
      children: kids
    };
  }
  if (r.switch !== undefined) kids.push(L.settingSwitch(r.switch));
  if (r.status) kids.push({ n: 'Status Label', t: 'text', hs: 'auto', vs: 'auto', text: { chars: r.status, size: 12, weight: 500, lh: 16, color: 'color.key.text' } });
  if (r.action) kids.push(L.smallButton(r.action, 'color.selected.bgStrong', 8, 2));
  return {
    n: 'Setting Row: ' + r.title, t: 'board', hs: 'fill', vs: 'auto',
    flex: { dir: 'row', colGap: 12, justify: 'space-between', align: 'center', hs: 'fill', vs: 'auto', padV: 6 }, colGapTok: 'space.md',
    children: kids
  };
};

L.settingsHeader = function (d) {
  const tabs = d.tabs.map(function (t) {
    const kids = [{ n: 'Tab Label: ' + t, t: 'text', hs: 'fill', vs: 'auto', text: { chars: t, size: 14, weight: 500, lh: 18, color: t === d.tab ? 'color.text.primary' : 'color.text.secondary' } }];
    if (t === d.tab) kids.push({ n: 'Tab Indicator', t: 'rect', hs: 'fill', vs: 'fix', w: 60, h: 2, fill: 'color.selected.bgStrong' });
    return {
      n: 'Tab: ' + t + (t === d.tab ? ' (selected)' : ''), t: 'board', hs: 'auto', vs: 'auto',
      flex: { dir: 'column', gap: 4, align: 'stretch', hs: 'auto', vs: 'auto' }, gapTok: 'space.xs',
      children: kids
    };
  });
  return {
    n: 'Settings Header', t: 'board', hs: 'fill', vs: 'auto',
    flex: { dir: 'row', justify: 'space-between', align: 'center', hs: 'fill', vs: 'auto' },
    children: [
      { n: 'Tabs', t: 'board', hs: 'auto', vs: 'auto', flex: { dir: 'row', colGap: 16, align: 'center', hs: 'auto', vs: 'auto' }, colGapTok: 'space.lg', children: tabs },
      { n: 'Close Button', t: 'board', hs: 'fix', vs: 'fix', w: 40, h: 40, fillNone: true, flex: { dir: 'row', align: 'center', justify: 'center', hs: 'fix', vs: 'fix' }, children: [{ n: 'Close Icon', t: 'icon', hs: 'fix', vs: 'fix', icon: { name: 'close', size: 24, color: 'color.text.secondary' } }] }
    ]
  };
};

// The selected tab's indicator is `hs: fill`, but an auto-width column leaves it
// at whatever the parent happened to be when flex was applied (60 or 100). Pin
// it to the measured label width so the underline matches the tab.
L.fixTabs = function (id) {
  var b = penpotUtils.findShapeById(id);
  var tabs = penpotUtils.findShapes(function (s) { return s.type === 'board' && s.name === 'Tabs'; }, b)[0];
  if (!tabs) return 'no Tabs board';
  var fixed = 0;
  var kids = tabs.children || [];
  for (var k = 0; k < kids.length; k++) {
    var label = null, ind = null;
    (kids[k].children || []).forEach(function (s) {
      if (s.type === 'text') { label = s; } else if (s.name === 'Tab Indicator') { ind = s; }
    });
    if (!label || !ind) continue;
    var w = Math.round(label.width);
    if (Math.round(ind.width) !== w) { ind.resize(w, 2); fixed++; }
  }
  try { tabs.flex.horizontalSizing = 'auto'; } catch (e) {}
  return 'indicators fixed: ' + fixed;
};

// connector row: icon + name + "kind • n roots" over a key.bg pill, with the
// enable switch and the edit / delete buttons on the right
L.connectorRow = function (c) {
  return {
    n: 'Connector Row: ' + c.name, t: 'board', hs: 'fill', vs: 'auto', r: { tok: 'radius.sm', px: 4 }, fill: 'color.key.bg',
    flex: { dir: 'row', colGap: 12, justify: 'space-between', align: 'center', hs: 'fill', vs: 'auto', padH: 12, padV: 8 }, colGapTok: 'space.md',
    children: [
      {
        n: 'Connector Identity', t: 'board', hs: 'auto', vs: 'auto',
        flex: { dir: 'row', colGap: 8, align: 'center', hs: 'auto', vs: 'auto' }, colGapTok: 'space.sm',
        children: [
          { n: 'Connector Icon (' + c.icon + ')', t: 'icon', hs: 'fix', vs: 'fix', icon: { name: c.icon, size: 20, color: 'color.text.secondary' } },
          {
            n: 'Connector Text', t: 'board', hs: 'auto', vs: 'auto',
            flex: { dir: 'column', gap: 2, align: 'start', hs: 'auto', vs: 'auto' },
            children: [
              { n: 'Connector Name', t: 'text', hs: 'auto', vs: 'auto', text: { chars: c.name, size: 14, weight: 500, lh: 18, color: 'color.text.primary' } },
              { n: 'Connector Kind', t: 'text', hs: 'auto', vs: 'auto', text: { chars: c.kind + ' \u2022 ' + c.roots + ' roots', size: 12, weight: 400, lh: 16, color: 'color.text.secondary' } }
            ]
          }
        ]
      },
      {
        n: 'Connector Actions', t: 'board', hs: 'auto', vs: 'auto',
        flex: { dir: 'row', colGap: 8, align: 'center', hs: 'auto', vs: 'auto' }, colGapTok: 'space.sm',
        children: [
          L.settingSwitch(c.enabled),
          { n: 'Edit Button', t: 'board', hs: 'fix', vs: 'fix', w: 40, h: 40, fillNone: true, flex: { dir: 'row', align: 'center', justify: 'center', hs: 'fix', vs: 'fix' }, children: [{ n: 'Edit Icon', t: 'icon', hs: 'fix', vs: 'fix', icon: { name: 'edit', size: 20, color: 'color.text.secondary' } }] },
          { n: 'Delete Button', t: 'board', hs: 'fix', vs: 'fix', w: 40, h: 40, fillNone: true, flex: { dir: 'row', align: 'center', justify: 'center', hs: 'fix', vs: 'fix' }, children: [{ n: 'Delete Icon', t: 'icon', hs: 'fix', vs: 'fix', icon: { name: 'delete', size: 20, color: 'color.text.secondary' } }] }
        ]
      }
    ]
  };
};

L.settingsNode = function (d) {
  const rows = [];
  d.rows.forEach(function (r, i) {
    if (i) rows.push({ n: 'Divider (row ' + i + ')', t: 'rect', hs: 'fill', vs: 'fix', w: L.SWC, h: 1, fill: 'color.border' });
    rows.push(r.node ? r.node : L.settingRow(r));
  });
  if (d.footerButton) {
    rows.push({
      n: 'Footer Row', t: 'board', hs: 'fill', vs: 'auto',
      flex: { dir: 'row', justify: 'end', align: 'center', hs: 'fill', vs: 'auto' },
      children: [{
        n: 'Add Button: ' + d.footerButton, t: 'board', hs: 'auto', vs: 'auto', r: { tok: 'radius.sm', px: 4 }, fill: 'color.key.bg',
        flex: { dir: 'row', colGap: 8, align: 'center', justify: 'center', hs: 'auto', vs: 'auto', padH: 12, padV: 8 },
        children: [
          { n: 'Button Icon', t: 'icon', hs: 'fix', vs: 'fix', icon: { name: 'add', size: 16, color: 'color.text.primary' } },
          { n: 'Button Label', t: 'text', hs: 'auto', vs: 'auto', text: { chars: d.footerButton, size: 14, weight: 400, lh: 18, color: 'color.text.primary' } }
        ]
      }]
    });
  }
  return L.windowBoard({
    n: d.name, w: L.SW, h: L.SH,
    flex: { dir: 'column', gap: 12, align: 'stretch', hs: 'fix', vs: 'fix', pad: 16 }, padTok: 'space.lg', gapTok: 'space.md',
    children: [
      L.settingsHeader(d),
      { n: 'Settings Body: ' + d.tab, t: 'board', hs: 'fill', vs: 'fill', w: L.SWC, h: L.SH - 32 - 40 - 12, fillNone: true, flex: { dir: 'column', gap: 12, align: 'stretch', hs: 'auto', vs: 'fill' }, gapTok: 'space.md', children: rows }
    ]
  }, true);
};

// Only used when (re)creating the token sets; `storage.T` holds the light token
// objects that the token-bound build applies.
const TOKENS = {
  light: Object.assign({}, L.HEX.light, {
    'space.xs': '4px', 'space.sm': '8px', 'space.md': '12px', 'space.lg': '16px', 'space.xl': '24px', 'space.input': '10px',
    'radius.sm': '4px', 'radius.md': '8px', 'radius.lg': '12px', 'radius.window': '16px',
    'font.size.input': '18px', 'font.size.result.title': '14px', 'font.size.result.meta': '12px', 'font.size.footer': '12px', 'font.size.section.title': '18px',
    'size.window.width': '720', 'size.window.height': '480', 'size.result.row': '44', 'size.icon.file': '32',
    'size.icon.cloud': '16', 'size.icon.source': '12', 'size.source.filter': '20', 'size.input.height': '48'
  })
};

// ============================================================ CHUNK settings2
// Round 2 of the settings window. It is a standalone app window, so it grows to
// 960x720, gets the system title bar on top, and the tab strip moves to the
// centre with an icon above each label. The indexing status that left the
// spotlight lands here, and the ConnectorEditorDialog, the kind DropdownMenu
// and the AWT tray menu get their own boards.

L.SW = 960;
L.SH = 720;
L.SWC = L.SW - 32;
L.DW = 520;
L.DH = 720;
L.DH2 = 520;

// macOS window chrome, identical in both themes.
L.HEX.dark['color.traffic.red'] = '#FF5F57';
L.HEX.dark['color.traffic.yellow'] = '#FEBC2E';
L.HEX.dark['color.traffic.green'] = '#28C840';
L.HEX.light['color.traffic.red'] = '#FF5F57';
L.HEX.light['color.traffic.yellow'] = '#FEBC2E';
L.HEX.light['color.traffic.green'] = '#28C840';

L.ICONS.extension = 'M20.5 11H19V7c0-1.1-.9-2-2-2h-4V3.5C13 2.12 11.88 1 10.5 1S8 2.12 8 3.5V5H4c-1.1 0-1.99.9-1.99 2v3.8H3.5c1.49 0 2.7 1.21 2.7 2.7s-1.21 2.7-2.7 2.7H2V20c0 1.1.9 2 2 2h3.8v-1.5c0-1.49 1.21-2.7 2.7-2.7 1.49 0 2.7 1.21 2.7 2.7V22H17c1.1 0 2-.9 2-2v-4h1.5c1.38 0 2.5-1.12 2.5-2.5S21.88 11 20.5 11z';
L.ICONS.link = 'M3.9 12c0-1.71 1.39-3.1 3.1-3.1h4V7H7c-2.76 0-5 2.24-5 5s2.24 5 5 5h4v-1.9H7c-1.71 0-3.1-1.39-3.1-3.1zM8 11h8v2H8v-2zm9-4h-4v1.9h4c1.71 0 3.1 1.39 3.1 3.1s-1.39 3.1-3.1 3.1h-4V17h4c2.76 0 5-2.24 5-5s-2.24-5-5-5z';
L.ICONS.dns = 'M20 13H4c-.55 0-1 .45-1 1v6c0 .55.45 1 1 1h16c.55 0 1-.45 1-1v-6c0-.55-.45-1-1-1zM7 19c-1.1 0-2-.9-2-2s.9-2 2-2 2 .9 2 2-.9 2-2 2zm13-12h-4V4c0-.55-.45-1-1-1H4c-.55 0-1 .45-1 1v3H3c-.55 0-1 .45-1 1v6c0 .55.45 1 1 1h1c.55 0 1-.45 1-1V9h10v3c0 .55.45 1 1 1h1c.55 0 1-.45 1-1V6.96c.61-.16 1-.62 1-1.21V4c0-.55-.45-1-1-1zm-6 3H4V5h10v5z';
L.ICONS.check = 'M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z';
L.ICONS.eye = 'M12 4.5C7 4.5 2.73 7.61 1 12c1.73 4.39 6 7.5 11 7.5s9.27-3.11 11-7.5c-1.73-4.39-6-7.5-11-7.5zM12 17c-2.76 0-5-2.24-5-5s2.24-5 5-5 5 2.24 5 5-2.24 5-5 5zm0-8c-1.66 0-3 1.34-3 3s1.34 3 3 3 3-1.34 3-3-1.34-3-3-3z';
L.ICONS.power = 'M13 3h-2v10h2V3zm4.83 2.17l-1.42 1.42C17.99 7.86 19 9.81 19 12c0 3.87-3.13 7-7 7s-7-3.13-7-7c0-2.19 1.01-4.14 2.58-5.42L6.17 5.17C4.23 6.82 3 9.26 3 12c0 4.97 4.03 9 9 9s9-4.03 9-9c0-2.74-1.23-5.18-3.17-6.83z';
L.ICONS.lock = 'M18 8h-1V6c0-2.76-2.24-5-5-5S7 3.24 7 6v2H6c-1.1 0-2 .9-2 2v10c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V10c0-1.1-.9-2-2-2zM9 8V6c0-1.66 1.34-3 3-3s3 1.34 3 3v2H9zm9 12H6V10h12v10z';
L.ICONS.vpn_key = 'M12.65 10C11.83 7.67 9.61 6 7 6c-3.31 0-6 2.69-6 6s2.69 6 6 6c2.61 0 4.83-1.67 5.65-4H17v4h4v-4h2v-4H12.65zM7 14c-1.1 0-2-.9-2-2s.9-2 2-2 2 .9 2 2-.9 2-2 2z';
L.ICONS.sync = 'M12 4V1L8 5l4 4V6c3.31 0 6 2.69 6 6 0 1.01-.25 1.97-.7 2.8l1.46 1.46C19.54 15.03 20 13.57 20 12c0-4.42-3.58-8-8-8zM6 7.7l-1.46 1.46C2.46 9.97 2 11.43 2 13c0 4.42 3.58 8 8 8v3l4-4-4-4v3c-3.31 0-6-2.69-6-6 0-1.01.25-1.97.7-2.8z';

// Title bar: traffic lights on the left, the window title centred, close right.
L.titleBar = function (d) {
  const lights = [
    { n: 'Traffic Light (close)', t: 'ellipse', hs: 'fix', vs: 'fix', w: 12, h: 12, fill: 'color.traffic.red' },
    { n: 'Traffic Light (minimize)', t: 'ellipse', hs: 'fix', vs: 'fix', w: 12, h: 12, fill: 'color.traffic.yellow' },
    { n: 'Traffic Light (zoom)', t: 'ellipse', hs: 'fix', vs: 'fix', w: 12, h: 12, fill: 'color.traffic.green' }
  ];
  return {
    n: 'Title Bar', t: 'board', hs: 'fill', vs: 'fix', w: d.w || L.SW, h: d.h || 44, fillNone: true,
    flex: { dir: 'row', colGap: 12, align: 'center', justify: 'space-between', hs: 'fill', vs: 'fix', padH: 16 },
    children: [
      { n: 'Traffic Lights', t: 'board', hs: 'auto', vs: 'auto', flex: { dir: 'row', colGap: 8, align: 'center', hs: 'auto', vs: 'auto' }, colGapTok: 'space.sm', children: d.lights === false ? [] : lights },
      { n: 'Title Center', t: 'board', hs: 'fill', vs: 'auto', w: 200, flex: { dir: 'row', align: 'center', justify: 'center', hs: 'fill', vs: 'auto' }, children: [
        { n: 'Window Title', t: 'text', hs: 'auto', vs: 'auto', text: { chars: d.title, size: 12, weight: 500, lh: 16, color: 'color.text.secondary' } }
      ] },
      { n: 'Close Button', t: 'board', hs: 'fix', vs: 'fix', w: 28, h: 28, fillNone: true, flex: { dir: 'row', align: 'center', justify: 'center', hs: 'fix', vs: 'fix' }, children: [
        { n: 'Close Icon', t: 'icon', hs: 'fix', vs: 'fix', icon: { name: 'close', size: 16, color: 'color.text.secondary' } }
      ] }
    ]
  };
};

// The tab strip is centred and every tab is an icon over its label; the active
// one keeps the 2dp underline from SettingsHeader.
L.SETTINGS_TABS2 = [
  { key: 'General', label: 'General', icon: 'tune' },
  { key: 'Modules', label: 'Modules', icon: 'extension' },
  { key: 'Connectors', label: 'Connectors', icon: 'link' },
  { key: 'Servers', label: 'Servers', icon: 'dns' }
];

L.settingsTabs = function (d) {
  const tabs = (d.tabs || L.SETTINGS_TABS2).map(function (t) {
    const sel = t.key === d.tab;
    const kids = [
      { n: 'Tab Icon (' + t.icon + ')', t: 'icon', hs: 'fix', vs: 'fix', icon: { name: t.icon, size: 20, color: sel ? 'color.text.primary' : 'color.text.secondary' } },
      { n: 'Tab Label: ' + t.label, t: 'text', hs: 'auto', vs: 'auto', text: { chars: t.label, size: 14, weight: 500, lh: 18, color: sel ? 'color.text.primary' : 'color.text.secondary' } }
    ];
    if (sel) kids.push({ n: 'Tab Indicator', t: 'rect', hs: 'auto', vs: 'fix', w: 40, h: 2, fill: 'color.selected.bgStrong' });
    return {
      n: 'Tab: ' + t.label + (sel ? ' (selected)' : ''), t: 'board', hs: 'auto', vs: 'auto',
      flex: { dir: 'column', gap: 6, align: 'center', hs: 'auto', vs: 'auto' }, children: kids
    };
  });
  return {
    n: 'Settings Tabs', t: 'board', hs: 'fill', vs: 'auto',
    flex: { dir: 'row', colGap: 40, justify: 'center', align: 'center', hs: 'fill', vs: 'auto' }, children: tabs
  };
};

L.sectionTitle = function (label) {
  return { n: 'Section: ' + label, t: 'board', hs: 'fill', vs: 'auto', flex: { dir: 'row', align: 'center', hs: 'fill', vs: 'auto' }, children: [
    { n: 'Section Title', t: 'text', hs: 'auto', vs: 'auto', text: { chars: label, size: 12, weight: 500, lh: 16, color: 'color.text.secondary' } }
  ] };
};

// IndexingStatus (SearchScreen.kt): label + "Start indexing" chip, or the
// LinearProgressIndicator while a pass runs.
L.indexingSection = function (d) {
  const running = d.progress !== undefined;
  const right = running
    ? { n: 'Indexing Percent', t: 'text', hs: 'auto', vs: 'auto', text: { chars: Math.round(d.progress * 100) + '%', size: 12, weight: 500, lh: 16, color: 'color.text.primary' } }
    : { n: 'Button: Start indexing', t: 'board', hs: 'auto', vs: 'auto', r: { tok: 'radius.sm', px: 4 }, fill: 'color.key.bg', flex: { dir: 'row', align: 'center', justify: 'center', hs: 'auto', vs: 'auto', padH: 10, padV: 4 }, children: [
        { n: 'Button Label', t: 'text', hs: 'auto', vs: 'auto', text: { chars: 'Start indexing', size: 12, weight: 400, lh: 16, color: 'color.text.primary' } }
      ] };
  const kids = [{ n: 'Indexing Row', t: 'board', hs: 'fill', vs: 'auto', flex: { dir: 'row', colGap: 12, justify: 'space-between', align: 'center', hs: 'fill', vs: 'auto' }, colGapTok: 'space.md', children: [
    { n: 'Indexing Label', t: 'text', hs: 'auto', vs: 'auto', text: { chars: d.label, size: 12, weight: 400, lh: 16, color: 'color.text.secondary' } },
    right
  ] }];
  if (running) kids.push({ n: 'Indexing Track', t: 'board', hs: 'fill', vs: 'fix', w: L.SWC, h: 4, r: { px: 2 }, fill: 'color.key.bg', flex: { dir: 'row', align: 'center', justify: 'start', hs: 'fill', vs: 'fix' }, children: [
    { n: 'Indexing Fill', t: 'rect', hs: 'fix', vs: 'fix', w: Math.round(L.SWC * d.progress), h: 4, r: { px: 2 }, fill: 'color.selected.bgStrong' }
  ] });
  return { n: 'Indexing Status', t: 'board', hs: 'fill', vs: 'auto', flex: { dir: 'column', gap: 8, align: 'stretch', hs: 'fill', vs: 'auto' }, gapTok: 'space.sm', children: kids };
};

// OutlinedTextField stand-in: the label sits above a 1px bordered field box.
L.field = function (d) {
  const shown = d.value === undefined || d.value === '' ? d.placeholder : d.value;
  const empty = d.value === undefined || d.value === '';
  const inner = [
    { n: 'Field Value', t: 'text', hs: 'auto', vs: 'auto', text: { chars: shown, size: 14, weight: 400, lh: 18, color: empty || d.muted ? 'color.text.secondary' : 'color.text.primary' } }
  ];
  if (d.trailing) inner.push({ n: 'Field Trailing: ' + d.trailing, t: 'board', hs: 'auto', vs: 'auto', flex: { dir: 'row', align: 'center', justify: 'end', hs: 'auto', vs: 'auto' }, children: [
    { n: 'Trailing Label', t: 'text', hs: 'auto', vs: 'auto', text: { chars: d.trailing, size: 14, weight: 500, lh: 18, color: 'color.text.primary' } }
  ] });
  const ih = d.compact ? (d.lines === 2 ? 54 : 38) : (d.lines === 2 ? 68 : 44);
  return {
    n: 'Field: ' + d.label, t: 'board', hs: 'fill', vs: 'auto',
    flex: { dir: 'column', gap: 4, align: 'stretch', hs: 'fill', vs: 'auto' }, gapTok: 'space.xs',
    children: [
      { n: 'Field Label', t: 'text', hs: 'auto', vs: 'auto', text: { chars: d.label, size: 12, weight: 400, lh: 16, color: 'color.text.secondary' } },
      { n: 'Field Input', t: 'board', hs: 'fill', vs: 'fix', w: d.w || L.SWC, h: ih, r: { tok: 'radius.sm', px: 8 }, fillNone: true, stroke: { tok: 'color.input.border', w: 1 }, flex: { dir: 'row', colGap: 12, align: d.lines === 2 ? 'start' : 'center', justify: d.trailing ? 'space-between' : 'start', hs: 'fill', vs: 'fix', padH: 12, padV: d.lines === 2 ? 8 : 0 }, children: inner }
    ]
  };
};

// M3 DropdownMenu surface and the AWT PopupMenu: a raised key.bg sheet of
// 36dp rows, the active row carrying a check.
L.menu = function (d) {
  const items = [];
  d.items.forEach(function (it, i) {
    if (it.separator) { items.push({ n: 'Menu Separator', t: 'rect', hs: 'fill', vs: 'fix', w: d.w - 24, h: 1, fill: 'color.border' }); return; }
    const kids = [];
    if (it.icon) kids.push({ n: 'Menu Icon (' + it.icon + ')', t: 'icon', hs: 'fix', vs: 'fix', icon: { name: it.icon, size: 16, color: 'color.text.secondary' } });
    kids.push({ n: 'Menu Label', t: 'text', hs: 'auto', vs: 'auto', text: { chars: it.label, size: 14, weight: it.selected ? 500 : 400, lh: 18, color: 'color.text.primary' } });
    if (it.selected) kids.push({ n: 'Menu Check', t: 'icon', hs: 'fix', vs: 'fix', icon: { name: 'check', size: 16, color: 'color.text.primary' } });
    items.push({
      n: 'Menu Item: ' + it.label, t: 'board', hs: 'fill', vs: 'fix', w: d.w, h: 36, fill: it.selected ? 'color.selected.bg' : null,
      flex: { dir: 'row', colGap: 12, align: 'center', justify: it.selected ? 'space-between' : 'start', hs: 'fill', vs: 'fix', padH: 12 },
      children: kids
    });
  });
  return {
    n: d.name, t: 'board', hs: 'fix', vs: 'auto', w: d.w, r: { tok: 'radius.md', px: 8 }, fill: 'color.hover.bg',
    shadows: [ { ox: 0, oy: 4, blur: 12, spread: 0, opacity: 0.35 }, { ox: 0, oy: 1, blur: 3, spread: 0, opacity: 0.3 } ],
    flex: { dir: 'column', align: 'stretch', hs: 'fix', vs: 'auto', padV: 6 },
    children: items
  };
};

// Explanatory strip: the icon plus a wrapped sentence, used where the design
// has to answer "what leaves this device?" out loud.
L.noteBox = function (d) {
  return {
    n: 'Note: ' + d.title, t: 'board', hs: 'fill', vs: 'auto', w: d.w || L.SWC, r: { tok: 'radius.md', px: 8 }, fill: 'color.key.bg',
    flex: { dir: 'row', colGap: 10, align: 'start', hs: 'fill', vs: 'auto', padH: 12, padV: 10 },
    children: [
      { n: 'Note Icon', t: 'icon', hs: 'fix', vs: 'fix', icon: { name: d.icon || 'lock', size: 16, color: 'color.key.text' } },
      { n: 'Note Body', t: 'board', hs: 'fill', vs: 'auto', w: (d.w || L.SWC) - 60, flex: { dir: 'column', gap: 2, align: 'start', hs: 'fill', vs: 'auto' }, children: [
        { n: 'Note Title', t: 'text', hs: 'auto', vs: 'auto', text: { chars: d.title, size: 12, weight: 500, lh: 16, color: 'color.text.primary' } },
        { n: 'Note Text', t: 'text', hs: 'fill', vs: 'auto', w: (d.w || L.SWC) - 60, text: { chars: d.text, size: 12, weight: 400, lh: 16, color: 'color.text.secondary' } }
      ] }
    ]
  };
};

// A connected worker: address, the vault + read key used to pull the delta
// index, and the last sync.
L.workerRow = function (d) {
  return {
    n: 'Worker Row: ' + d.host, t: 'board', hs: 'fill', vs: 'auto', w: L.SWC, r: { tok: 'radius.sm', px: 8 }, fill: 'color.key.bg',
    flex: { dir: 'row', colGap: 12, align: 'center', justify: 'space-between', hs: 'fill', vs: 'auto', padH: 16, padV: 12 },
    children: [
      { n: 'Worker Identity', t: 'board', hs: 'auto', vs: 'auto', flex: { dir: 'row', colGap: 12, align: 'center', hs: 'auto', vs: 'auto' }, children: [
        { n: 'Worker Icon', t: 'icon', hs: 'fix', vs: 'fix', icon: { name: 'dns', size: 20, color: 'color.text.secondary' } },
        { n: 'Worker Label', t: 'board', hs: 'auto', vs: 'auto', flex: { dir: 'column', gap: 2, align: 'start', hs: 'auto', vs: 'auto' }, children: [
          { n: 'Worker Host', t: 'text', hs: 'auto', vs: 'auto', text: { chars: d.host + (d.port ? ':' + d.port : ''), size: 14, weight: 500, lh: 18, color: 'color.text.primary' } },
          { n: 'Worker Meta', t: 'text', hs: 'auto', vs: 'auto', text: { chars: d.meta, size: 12, weight: 400, lh: 16, color: 'color.text.secondary' } }
        ] }
      ] },
      { n: 'Worker Status', t: 'board', hs: 'auto', vs: 'auto', flex: { dir: 'column', gap: 2, align: 'end', hs: 'auto', vs: 'auto' }, children: [
        { n: 'Status Label', t: 'text', hs: 'auto', vs: 'auto', text: { chars: d.status, size: 12, weight: 500, lh: 16, color: d.status === 'Connected' ? 'color.text.primary' : 'color.text.secondary' } },
        { n: 'Status Detail', t: 'text', hs: 'auto', vs: 'auto', text: { chars: d.detail, size: 12, weight: 400, lh: 16, color: 'color.text.secondary' } }
      ] }
    ]
  };
};

// Window shell: title bar, centred tabs, divider, then the padded body.
L.settingsNode2 = function (d) {
  const rows = [];
  d.sections.forEach(function (sec, i) {
    if (i) {
      rows.push({ n: 'Divider (section ' + i + ')', t: 'rect', hs: 'fill', vs: 'fix', w: L.SWC, h: 1, fill: 'color.border' });
      // Sections marked anchor:'bottom' sit at the foot of the tall window.
      if (sec.anchor === 'bottom') {
        rows.push({ n: 'Spacer (fills the window)', t: 'rect', hs: 'fill', vs: 'fill', w: L.SWC, h: 24, fillNone: true });
      }
    }
    if (sec.title) rows.push(L.sectionTitle(sec.title));
    if (sec.desc) rows.push({ n: 'Section Description', t: 'text', hs: 'fill', vs: 'auto', w: L.SWC, text: { chars: sec.desc, size: 12, weight: 400, lh: 16, color: 'color.text.secondary' } });
    if (sec.indexing) rows.push(L.indexingSection(sec.indexing));
    if (sec.note) rows.push(L.noteBox(Object.assign({ w: L.SWC }, sec.note)));
    (sec.rowsAfter || []).forEach(function (r) { rows.push(r.node ? r.node : L.settingRow(r)); });
    (sec.rows || []).forEach(function (r) { rows.push(r.node ? r.node : L.settingRow(r)); });
  });
  if (d.footerButton) {
    if (d.footerAnchor) rows.push({ n: 'Spacer (fills the window)', t: 'rect', hs: 'fill', vs: 'fill', w: L.SWC, h: 24, fillNone: true });
    rows.push({
      n: 'Footer Row', t: 'board', hs: 'fill', vs: 'auto',
      flex: { dir: 'row', justify: 'end', align: 'center', hs: 'fill', vs: 'auto' },
      children: [{
        n: 'Add Button: ' + d.footerButton, t: 'board', hs: 'auto', vs: 'auto', r: { tok: 'radius.sm', px: 4 }, fill: 'color.key.bg',
        flex: { dir: 'row', colGap: 8, align: 'center', justify: 'center', hs: 'auto', vs: 'auto', padH: 12, padV: 8 },
        children: [
          { n: 'Button Icon', t: 'icon', hs: 'fix', vs: 'fix', icon: { name: 'add', size: 16, color: 'color.text.primary' } },
          { n: 'Button Label', t: 'text', hs: 'auto', vs: 'auto', text: { chars: d.footerButton, size: 14, weight: 400, lh: 18, color: 'color.text.primary' } }
        ]
      }]
    });
  }
  return L.windowBoard({
    n: d.name, w: L.SW, h: L.SH,
    flex: { dir: 'column', gap: 12, align: 'stretch', hs: 'fix', vs: 'fix', padH: 0, padB: 16 },
    children: [
      L.titleBar({ title: d.winTitle || 'Mirage Settings' }),
      L.settingsTabs(d),
      { n: 'Divider (tabs)', t: 'rect', hs: 'fill', vs: 'fix', w: L.SW, h: 1, fill: 'color.border' },
      { n: 'Settings Body: ' + d.tab, t: 'board', hs: 'fill', vs: 'fill', h: L.SH - 128, fillNone: true, flex: { dir: 'column', gap: 16, align: 'stretch', hs: 'auto', vs: 'fill', padH: 16 }, gapTok: 'space.lg', children: rows }
    ]
  }, true);
};

// ConnectorEditorDialog (SettingsWindow.kt): its own window; the Compose
// column is 520x640 but the 8 stacked OutlinedTextFields need ~700dp, so the
// board keeps the width and grows the height.
L.dialogNode = function (d) {
  const fw = L.DW - 40;
  const rows = [
    L.field({ label: 'Name', value: d.connName, w: fw, compact: true }),
    L.field({ label: 'Kind', value: d.kind, trailing: 'Change', w: fw, compact: true }),
    L.field({ label: 'Roots', value: d.roots, lines: 2, w: fw, compact: true }),
    { n: 'Enabled Row', t: 'board', hs: 'fill', vs: 'auto', flex: { dir: 'row', colGap: 8, align: 'center', hs: 'fill', vs: 'auto' }, children: [
      L.settingSwitch(d.enabled === undefined ? true : d.enabled),
      { n: 'Enabled Label', t: 'text', hs: 'auto', vs: 'auto', text: { chars: 'Enabled', size: 14, weight: 400, lh: 18, color: 'color.text.primary' } }
    ] },
    { n: 'Divider (credentials)', t: 'rect', hs: 'fill', vs: 'fix', w: fw, h: 1, fill: 'color.border' },
    { n: 'Credentials Title', t: 'text', hs: 'auto', vs: 'auto', text: { chars: 'Credentials', size: 14, weight: 500, lh: 18, color: 'color.text.primary' } }
  ];
  d.fields.forEach(function (f) { rows.push(L.field({ label: f[0], value: f[1], muted: f[2], w: fw, compact: true })); });
  rows.push({ n: 'Spacer (weight 1f)', t: 'rect', hs: 'fill', vs: 'fill', w: fw, h: 16, fillNone: true });
  rows.push({ n: 'Dialog Actions', t: 'board', hs: 'fill', vs: 'auto', flex: { dir: 'row', colGap: 12, align: 'center', justify: 'space-between', hs: 'fill', vs: 'auto' }, children: [
    { n: 'Cancel Button', t: 'board', hs: 'auto', vs: 'auto', flex: { dir: 'row', align: 'center', justify: 'center', hs: 'auto', vs: 'auto', padH: 8, padV: 6 }, children: [
      { n: 'Button Label', t: 'text', hs: 'auto', vs: 'auto', text: { chars: 'Cancel', size: 14, weight: 400, lh: 18, color: 'color.text.secondary' } }
    ] },
    { n: 'Save Button', t: 'board', hs: 'auto', vs: 'auto', r: { tok: 'radius.sm', px: 4 }, fill: 'color.selected.bgStrong', flex: { dir: 'row', align: 'center', justify: 'center', hs: 'auto', vs: 'auto', padH: 16, padV: 8 }, children: [
      { n: 'Button Label', t: 'text', hs: 'auto', vs: 'auto', text: { chars: 'Save', size: 14, weight: 500, lh: 18, color: 'color.text.primary' } }
    ] }
  ] });
  return L.windowBoard({
    n: d.name, w: L.DW, h: L.DH,
    flex: { dir: 'column', gap: 0, align: 'stretch', hs: 'fix', vs: 'fix', padH: 0, padB: 16 },
    children: [
      L.titleBar({ title: d.title, h: 40, w: L.DW }),
      { n: 'Dialog Body', t: 'board', hs: 'fill', vs: 'fill', h: L.DH - 56, fillNone: true, flex: { dir: 'column', gap: 8, align: 'stretch', hs: 'fill', vs: 'fill', padH: 20 }, gapTok: 'space.md', children: rows }
    ]
  }, true);
};

L.SETTINGS2 = [
  {
    name: 'Settings / General (Dark)', tab: 'General', winTitle: 'Mirage Settings',
    sections: [
      { title: 'Indexing', indexing: { label: 'Indexing...  12,480 of 20,000 files', progress: 0.62 } },
      { title: 'Application', rows: [
        { title: 'Start at login', desc: 'Launch Mirage automatically when you log in.', switch: false },
        { title: 'Clipboard indexing', desc: 'Keep a searchable history of copied text.', switch: true },
        { title: 'Excluded directories', desc: 'Comma-separated paths relative to the vault root.', input: 'e.g. node_modules, .git, build' }
      ] },
      { anchor: 'bottom', rows: [ { title: 'Quit Mirage', desc: 'Close the application.', action: 'Quit' } ] }
    ]
  },
  {
    name: 'Settings / Modules (Dark)', tab: 'Modules', winTitle: 'Mirage Settings',
    sections: [
      { title: 'On-device models', rows: [
        { title: 'OCR (Vision)', desc: '', status: 'Ready', progress: 1 },
        { title: 'Transcription (Whisper)', desc: '', status: 'Downloading...', progress: 0.42, cancel: true },
        { title: 'Summarization', desc: '', status: 'Not installed', progress: 0, action: 'Download' }
      ] },
      { indexing: { label: '12,480 indexed', progress: undefined } }
    ]
  },
  {
    name: 'Settings / Connectors (Dark)', tab: 'Connectors', winTitle: 'Mirage Settings', footerButton: 'Add connector',
    sections: [
      { title: 'Connected accounts', rows: [
        { node: L.connectorRow({ name: 'Company Dropbox', icon: 'cloud', kind: 'Dropbox', roots: 3, enabled: true }) },
        { node: L.connectorRow({ name: 'Backups bucket', icon: 'storage', kind: 'S3 / R2', roots: 1, enabled: true }) },
        { node: L.connectorRow({ name: 'NAS share', icon: 'folder', kind: 'SMB / NAS', roots: 2, enabled: false }) }
      ] }
    ]
  },
  {
    name: 'Settings / Servers (Dark)', tab: 'Servers', winTitle: 'Mirage Settings',
    footerButton: 'Add worker', footerAnchor: true,
    sections: [
      {
        title: 'Index workers',
        desc: 'A worker indexes large sources next to the data and sends back only the compressed delta index. Small and medium sources always stay on this device.',
        rows: [
          { node: L.workerRow({ host: 'index.internal.co', port: 443, meta: 'vault mirage-team • key sec_pk_9f8a…3d12', status: 'Connected', detail: 'delta 12 min ago • 1,204,880 vectors' }) },
          { node: L.workerRow({ host: '127.0.0.1', port: 8787, meta: 'vault mirage-local • key sec_pk_2b71…c40e', status: 'Connected', detail: 'delta just now • 48,210 vectors' }) }
        ]
      },
      {
        title: 'Offload',
        rows: [
          { title: 'Index large sources remotely', desc: 'Sources above 2 GB go to a worker instead of this machine.', switch: true }
        ],
        note: { icon: 'lock', title: 'Storage credentials stay on this device', text: 'Mirage shares bucket names, roots and file filters with the worker — never keys or tokens.\nThe worker signs into S3, Dropbox or the NAS with its own credentials, set in its admin console.' },
        rowsAfter: [
          { title: 'S3 / R2 • Backups bucket', desc: 'mirage/ • 1.4 TB • about 9 h if indexed here', action: 'Offload' },
          { title: 'SMB / NAS • NAS share', desc: 'docs/, archive/ • 8.2 TB • about 2 days if indexed here', action: 'Offload' }
        ]
      }
    ]
  }
];

L.DIALOG = {
  name: 'Dialog / Connector Editor (Dark)', title: 'Add connector',
  connName: 'Backups bucket',
  kind: 'S3 / R2', roots: 'mirage/, reports/',
  fields: [
    [ 'Bucket', 'mirage-vault' ],
    [ 'Endpoint (optional)', 'https://s3.wasabisys.com' ],
    [ 'Region', 'eu-central-1' ],
    [ 'Access key', 'AKIAIOSFODNN7EXAMPLE' ],
    [ 'Secret key', '••••••••••••••••' ]
  ]
};

// AddServerScreen.kt: address + server code (vaultId:passkey) or a full
// vault:// URI. The board adds the note that answers "what leaves this box?".
L.serverDialogNode = function (d) {
  const fw = L.DW - 40;
  const rows = [
    { n: 'Dialog Heading', t: 'text', hs: 'auto', vs: 'auto', text: { chars: d.title, size: 18, weight: 500, lh: 24, color: 'color.text.primary' } },
    { n: 'Dialog Subtitle', t: 'text', hs: 'fill', vs: 'auto', w: fw, text: { chars: d.subtitle, size: 12, weight: 400, lh: 16, color: 'color.text.secondary' } },
    L.field({ label: 'Server URL', placeholder: 'https://mirage.example.com', w: fw, compact: true }),
    L.field({ label: 'Server code', placeholder: 'my-vault:abc123', w: fw, compact: true }),
    { n: 'HTTPS Row', t: 'board', hs: 'fill', vs: 'auto', flex: { dir: 'row', colGap: 8, align: 'center', hs: 'fill', vs: 'auto' }, children: [
      L.settingSwitch(true),
      { n: 'HTTPS Label', t: 'text', hs: 'auto', vs: 'auto', text: { chars: 'Use HTTPS', size: 14, weight: 400, lh: 18, color: 'color.text.primary' } }
    ] },
    L.noteBox({ w: fw, icon: 'lock', title: 'Credentials never leave this device', text: 'The address and code only open the delta-sync API.\nThe worker reads storage with keys configured on itself.' }),
    { n: 'Offload Row', t: 'board', hs: 'fill', vs: 'auto', flex: { dir: 'row', colGap: 8, align: 'center', hs: 'fill', vs: 'auto' }, children: [
      L.settingSwitch(true),
      { n: 'Offload Label', t: 'text', hs: 'auto', vs: 'auto', text: { chars: 'Offload large sources to this worker', size: 14, weight: 400, lh: 18, color: 'color.text.primary' } }
    ] },
    { n: 'Spacer (weight 1f)', t: 'rect', hs: 'fill', vs: 'fill', w: fw, h: 12, fillNone: true },
    { n: 'Connect Progress', t: 'board', hs: 'fill', vs: 'fix', w: fw, h: 4, r: { px: 2 }, fill: 'color.key.bg', flex: { dir: 'row', align: 'center', justify: 'start', hs: 'fill', vs: 'fix' }, children: [
      { n: 'Progress Fill', t: 'rect', hs: 'fix', vs: 'fix', w: Math.round(fw * 0.65), h: 4, r: { px: 2 }, fill: 'color.selected.bgStrong' }
    ] },
    { n: 'Status Message', t: 'text', hs: 'auto', vs: 'auto', text: { chars: 'Connecting… syncing delta index', size: 12, weight: 400, lh: 16, color: 'color.key.text' } },
    { n: 'Server Actions', t: 'board', hs: 'fill', vs: 'auto', flex: { dir: 'row', colGap: 8, align: 'center', justify: 'space-between', hs: 'fill', vs: 'auto' }, children: [
      { n: 'Cancel Button', t: 'board', hs: 'auto', vs: 'auto', flex: { dir: 'row', align: 'center', justify: 'center', hs: 'auto', vs: 'auto', padH: 8, padV: 6 }, children: [
        { n: 'Button Label', t: 'text', hs: 'auto', vs: 'auto', text: { chars: 'Cancel', size: 14, weight: 400, lh: 18, color: 'color.text.secondary' } }
      ] },
      { n: 'Server Actions Right', t: 'board', hs: 'auto', vs: 'auto', flex: { dir: 'row', colGap: 8, align: 'center', hs: 'auto', vs: 'auto' }, children: [
        { n: 'Mode Button', t: 'board', hs: 'auto', vs: 'auto', flex: { dir: 'row', align: 'center', justify: 'center', hs: 'auto', vs: 'auto', padH: 8, padV: 6 }, children: [
          { n: 'Button Label', t: 'text', hs: 'auto', vs: 'auto', text: { chars: 'Paste full Vault URI', size: 14, weight: 400, lh: 18, color: 'color.text.secondary' } }
        ] },
        L.smallButton('Connect', 'color.selected.bgStrong', 16, 8)
      ] }
    ] }
  ];
  return L.windowBoard({
    n: d.name, w: L.DW, h: L.DH2,
    flex: { dir: 'column', gap: 0, align: 'stretch', hs: 'fix', vs: 'fix', padH: 0, padB: 16 },
    children: [
      L.titleBar({ title: d.winTitle, h: 40, w: L.DW }),
      { n: 'Dialog Body', t: 'board', hs: 'fill', vs: 'fill', h: L.DH2 - 56, fillNone: true, flex: { dir: 'column', gap: 10, align: 'stretch', hs: 'fill', vs: 'fill', padH: 20 }, children: rows }
    ]
  }, true);
};
L.SERVER_DIALOG = {
  name: 'Dialog / Add Server (Dark)', winTitle: 'Add Server', title: 'Add Server',
  subtitle: 'Connect to a Mirage worker that indexes your large sources for you.'
};

L.KIND_MENU = {
  name: 'Menu / Connector Kind (Dark)', w: 200,
  items: [
    { label: 'S3 / R2', selected: true },
    { label: 'Dropbox' },
    { label: 'Google Drive' },
    { label: 'SMB / NAS' }
  ]
};

L.TRAY_MENU = {
  name: 'Menu / System Tray (Dark)', w: 200,
  items: [
    { label: 'Show', icon: 'eye' },
    { label: 'Settings', icon: 'tune' },
    { separator: true },
    { label: 'Quit', icon: 'power' }
  ]
};

// Board slots: the four tabs on the top row, the dialog and the two menus below.
L.SET_SLOTS = { General: [0, 0], Modules: [1040, 0], Connectors: [2080, 0], Servers: [3120, 0], Dialog: [0, 800], KindMenu: [600, 800], TrayMenu: [900, 800], ServerDialog: [600, 1160] };

// Rebuild one of the four tab boards, dropping the previous version.
L.buildSettings2 = async function (i) {
  var d = Object.assign({}, L.SETTINGS2[i], { tabs: L.SETTINGS_TABS2 });
  var slot = L.SET_SLOTS[d.tab];
  var old = penpotUtils.findShapeById(storage['set' + d.tab]);
  if (old) old.remove();
  L.errors = []; L.created = [];
  var board = await L.build(L.settingsNode2(d), null, 'dark');
  board.x = slot[0]; board.y = slot[1];
  storage['set' + d.tab] = board.id;
  return { id: board.id, name: board.name, created: L.created.length, errors: L.errors.slice(0, 5) };
};

// The dialog and the two menus, each in its own slot.
L.buildExtra = async function (which) {
  var node = which === 'dialog' ? L.dialogNode(L.DIALOG)
    : which === 'server' ? L.serverDialogNode(L.SERVER_DIALOG)
    : which === 'kind' ? L.menu(L.KIND_MENU)
    : L.menu(L.TRAY_MENU);
  var key = which === 'dialog' ? 'dlgConnector' : which === 'server' ? 'dlgServer' : which === 'kind' ? 'menuKind' : 'menuTray';
  var old = penpotUtils.findShapeById(storage[key]);
  if (old) old.remove();
  L.errors = []; L.created = [];
  var board = await L.build(node, null, 'dark');
  var slot = L.SET_SLOTS[which === 'dialog' ? 'Dialog' : which === 'server' ? 'ServerDialog' : which === 'kind' ? 'KindMenu' : 'TrayMenu'];
  board.x = slot[0]; board.y = slot[1];
  storage[key] = board.id;
  return { id: board.id, name: board.name, created: L.created.length, errors: L.errors.slice(0, 5) };
};

// The MCP gateway aborts anything past a few seconds, so a board is finished in
// two short calls: build + flex + line-heights, then release and measure.
L.step1 = async function (kind, arg) {
  var r = kind === 'extra' ? await L.buildExtra(arg) : await L.buildSettings2(arg);
  var f = L.stepFlex(r.id);
  var h = L.fixLH(r.id);
  penpot.viewport.zoom = 0.75;
  return { r: r, f: f, h: h };
};

L.step2 = async function (id) {
  var rel = L.stepRelease(id);
  return { id: id, rel: rel };
};

L.step3 = function (id) {
  return L.stepMeasure(id);
};

// ================================================================= CHUNK data
// Sample content: query "annual report", the Dropbox PDF selected.
L.SPOT = {
  name: 'Spotlight', query: 'annual report', placeholder: 'Search files...', showDownload: true,
  results: [
    { icon: 'document', title: 'Annual Report 2025.pdf', path: '~/Dropbox/Finance/Annual Report 2025.pdf', cloud: true, selected: true },
    { icon: 'image', title: 'annual-report-cover.jpg', path: '~/Dropbox/Marketing/annual-report-cover.jpg', cloud: true },
    { icon: 'movie', title: 'Annual Report Recap.mp4', path: '~/Google Drive/Videos/Annual Report Recap.mp4', cloud: true },
    { icon: 'file', title: 'annual-report-draft.docx', path: '~/Projects/Mirage/docs/annual-report-draft.docx' },
    { icon: 'document', title: 'Q3-Board-Report.pdf', path: '~/S3 mirage-backups/Reports/Q3-Board-Report.pdf', cloud: true },
    { icon: 'image', title: 'report-chart-bar.png', path: '~/Downloads/report-chart-bar.png' }
  ],
  sources: [
    { kind: 'local', icon: 'folder', active: true },
    { kind: 'app', icon: 'document', active: true },
    { kind: 'dropbox', icon: 'cloud', active: true },
    { kind: 'gdrive', icon: 'cloud', active: true },
    { kind: 's3', icon: 'storage', active: true },
    { kind: 'smb', icon: 'folder', active: false }
  ]
};

L.CLIP = {
  name: 'Clipboard / History', query: '', placeholder: 'Search clipboard...',
  entries: [
    { icon: 'image', label: 'Image (2.4 MB)', time: '2026-08-30 11:42:07', selected: true },
    { icon: 'document', label: 'Annual Report 2025.pdf', time: '2026-08-30 11:39:52' },
    { icon: 'file', label: 'smb://nas/backups (18.2 GB)', time: '2026-08-30 10:15:31' },
    { icon: 'folder', label: '~/Projects/mirage/src', time: '2026-08-30 09:58:12' },
    { icon: 'movie', label: 'Annual Report Recap.mp4', time: '2026-08-29 18:04:45' }
  ],
  previewIcon: 'image', previewLabel: 'Image (2.4 MB)', previewType: 'Image', previewSize: '2.4 MB', copiedAt: '2026-08-30 11:42:07'
};

// Settings tabs (SettingsWindow.kt). Rows mirror the composables:
// SettingSwitchRow / SettingInputRow / SettingActionRow / ModuleDownloadRow /
// ConnectorRow / ServerRow.
L.SETTINGS_TABS = ['General', 'Modules', 'Connectors', 'Servers'];

L.SETTINGS = [
  {
    name: 'Settings / General (Dark)', tab: 'General',
    rows: [
      { title: 'Start at login', desc: 'Launch Mirage automatically when you log in.', switch: false },
      { title: 'Clipboard indexing', desc: 'Keep a searchable history of copied text.', switch: true },
      { title: 'Excluded directories', desc: 'Comma-separated paths relative to the vault root.', input: 'e.g. node_modules, .git, build' },
      { title: 'Quit Mirage', desc: 'Close the application.' }
    ]
  },
  {
    name: 'Settings / Modules (Dark)', tab: 'Modules',
    rows: [
      { title: 'OCR (Vision)', desc: '', status: 'Ready', progress: 1 },
      { title: 'Transcription (Whisper)', desc: '', status: 'Downloading...', progress: 0.42, cancel: true },
      { title: 'Summarization', desc: '', status: 'Not installed', progress: 0, action: 'Download' }
    ]
  },
  {
    name: 'Settings / Connectors (Dark)', tab: 'Connectors', footerButton: 'Add connector',
    rows: [
      { node: L.connectorRow({ name: 'Company Dropbox', icon: 'cloud', kind: 'Dropbox', roots: 3, enabled: true }) },
      { node: L.connectorRow({ name: 'Backups bucket', icon: 'storage', kind: 'S3 / R2', roots: 1, enabled: true }) },
      { node: L.connectorRow({ name: 'NAS share', icon: 'folder', kind: 'SMB / NAS', roots: 2, enabled: false }) }
    ]
  },
  {
    name: 'Settings / Servers (Dark)', tab: 'Servers', footerButton: 'Add server',
    rows: [
      { title: 'http://127.0.0.1:8787', desc: 'Vault: mirage-local' },
      { title: 'https://index.internal.co', desc: 'Vault: mirage-team' }
    ]
  }
];

// =============================================================== CHUNK layout
// Board slots on the page: results dark 0, empty dark 800, clipboard dark 1600,
// results light 2400.
L.SLOTS = { spotDark: [0, 0], emptyDark: [800, 0], clipDark: [1600, 0], spotLight: [2400, 0] };

/* Build commands, sent one at a time once the chunks above are in storage:

  await L.replaceBoard(storage.winEmptyDark,
    L.spotlightNode(Object.assign({}, L.SPOT, { name: 'Spotlight / Empty (Dark)', results: [], showDownload: false })),
    800, 0, 'dark')

  await L.replaceBoard(storage.winClipDark,
    L.clipboardNode(Object.assign({}, L.CLIP, { name: 'Clipboard / History (Dark)' })), 1600, 0, 'dark')

  await L.replaceBoard(storage.winSpotLight,
    L.spotlightNode(Object.assign({}, L.SPOT, { name: 'Spotlight / Results (Light)' })), 2400, 0)

  exportShape({ shapeId: storage.winClipDark, scale: 1, format: 'png' })

  // settings: its own page; 4 tabs on the top row, dialog + menus below
  var p = penpot.createPage('Mirage · Settings'); p.name = 'Mirage · Settings';
  await penpot.openPage(p.id);
  for (var i = 0; i < 4; i++) {
    await L.step1('settings2', i)  // build + flex + line-heights, returns { r: { id } }
    await L.step2(id)              // release the texts so the render loop measures
    L.step3(id)                    // pin frames; repeat until waiting === 0
  }
  await L.step1('extra', 'dialog')   // ConnectorEditorDialog
  await L.step1('extra', 'server')   // AddServerScreen
  await L.step1('extra', 'kind')     // ConnectorKind DropdownMenu
  await L.step1('extra', 'tray')     // AWT tray PopupMenu
  // L.menu boards need a manual resize(200, h) afterwards: L.build only
  // resizes when both w and h are given, and the menus are height-auto.

  // after a plugin reload: re-send the chunks, then re-point the ids
  storage.setGeneral = '<board id>'; // ... setModules/setConnectors/setServers
  storage.dlgConnector = '<board id>'; // ... dlgServer/menuKind/menuTray

  // ids as of 2026-08-30 (page "Mirage · Settings")
  storage.setGeneral    = '6d84823e-451f-80da-8008-9059020e7d1a';
  storage.setModules    = '6d84823e-451f-80da-8008-90594b5196b1';
  storage.setConnectors = '6d84823e-451f-80da-8008-90598d93e906';
  storage.setServers    = '6d84823e-451f-80da-8008-905c37c7f5b0';
  storage.dlgConnector  = '6d84823e-451f-80da-8008-905a220c9536';
  storage.dlgServer     = '6d84823e-451f-80da-8008-905ca0941f69';
  storage.menuKind      = '6d84823e-451f-80da-8008-9053f035be7d';
  storage.menuTray      = '6d84823e-451f-80da-8008-905401bfef8a';
*/

