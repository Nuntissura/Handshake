// hsk_oracle_photoshop.jsx  -  WP-KERNEL-STUDIO D5 oracle session, Photoshop batch (ExtendScript, ES3).
// Plan: oracle-session/SESSION_PLAN.json (app "photoshop"). Operator steps: oracle-session/RUN.md.
// Writes ONLY under OUT_ROOT/photoshop. Never saves over user files. Opens no dialogs (DialogModes.NO).
// Global state touched: app.displayDialogs, app.preferences.rulerUnits, app.colorSettings (gamma pass only).
// All three are read first and restored in the finally block.
// API status: DOM calls were checked against the Adobe-dictionary-derived typings (pravdomil/types-for-adobe,
// Photoshop 2015.5) and the Photoshop scripting reference mirror. Every Action Manager (executeAction)
// key below is UNVERIFIED (ScriptingListener-style IDs written from memory; no Adobe document lists them).
// Each variant runs in its own try/catch; failures go to errors.log and manifest.json (status "error").
#target photoshop
(function () {
  // Absolute path is unavoidable (Operator-specified output root). Change only this line to relocate.
  var OUT_ROOT = "D:/Projects/LLM projects/Handshake/Handshake Worktrees/Handshake_Artifacts/WP-KERNEL-STUDIO/oracle-corpus/adobe-session";
  var APP = "photoshop";
  var GAMMA_PRESET = "HSK-gamma-1.0";   // Color Settings preset saved by the Operator beforehand (RUN.md step 2)
  var OUT = OUT_ROOT + "/" + APP, CTRL = OUT_ROOT + "/control";

  // ---------- generic helpers ----------
  function ensure(p) { var f = new Folder(p); if (!f.exists) { ensure(f.parent.fsName.replace(/\\/g, "/")); f.create(); } }
  function wtxt(p, t) { var f = new File(p); f.encoding = "UTF-8"; f.open("w"); f.write(t); f.close(); }
  function atxt(p, t) { var f = new File(p); f.encoding = "UTF-8"; f.open("a"); f.write(t); f.close(); }
  function rtxt(p) { var f = new File(p); if (!f.exists) return null; f.encoding = "UTF-8"; f.open("r"); var t = f.read(); f.close(); return t; }
  function ser(v) {
    if (v === null || v === undefined) return "null";
    var t = typeof v;
    if (t === "number") return isFinite(v) ? String(v) : "null";
    if (t === "boolean") return v ? "true" : "false";
    if (t === "string") return '"' + v.replace(/\\/g, "\\\\").replace(/"/g, '\\"').replace(/\r/g, "\\r").replace(/\n/g, "\\n").replace(/\t/g, "\\t") + '"';
    var i, a = [];
    if (v instanceof Array) { for (i = 0; i < v.length; i++) a.push(ser(v[i])); return "[" + a.join(",") + "]"; }
    for (var k in v) if (v.hasOwnProperty(k)) a.push(ser(k) + ":" + ser(v[k]));
    return "{" + a.join(",") + "}";
  }
  function nowIso() { var d = new Date(); return d.getUTCFullYear() + "-" + (d.getUTCMonth() + 1) + "-" + d.getUTCDate() + "T" + d.getUTCHours() + ":" + d.getUTCMinutes() + ":" + d.getUTCSeconds() + "Z"; }

  // ---------- Action Manager helpers (UNVERIFIED keys) ----------
  // Keys/IDs starting with "#" are 4-char charIDs, others are stringIDs.
  function tid(s) { return s.charAt(0) === "#" ? charIDToTypeID(s.substr(1)) : stringIDToTypeID(s); }
  function I(v) { return { t: "i", v: v }; }
  function U(u, v) { return { t: "u", u: u, v: v }; }
  function E(e, v) { return { t: "e", e: e, v: v }; }
  function O(c, o) { return { t: "o", c: c, o: o }; }
  function L(a) { return { t: "l", a: a }; }
  function RF(r) { return { t: "r", ref: r }; }
  function CL(c) { return { t: "c", v: c }; }
  function mkL(a) { var l = new ActionList(); for (var i = 0; i < a.length; i++) lput(l, a[i]); return l; }
  function lput(l, v) {
    if (typeof v === "number") l.putDouble(v);
    else if (typeof v === "string") l.putString(v);
    else if (typeof v === "boolean") l.putBoolean(v);
    else switch (v.t) {
      case "i": l.putInteger(v.v); break;
      case "u": l.putUnitDouble(tid(v.u), v.v); break;
      case "e": l.putEnumerated(tid(v.e), tid(v.v)); break;
      case "o": l.putObject(tid(v.c), mk(v.o)); break;
      case "r": l.putReference(v.ref); break;
      case "l": l.putList(mkL(v.a)); break;
      case "c": l.putClass(tid(v.v)); break;
    }
  }
  function put(d, k, v) {
    var key = tid(k);
    if (typeof v === "number") d.putDouble(key, v);
    else if (typeof v === "string") d.putString(key, v);
    else if (typeof v === "boolean") d.putBoolean(key, v);
    else switch (v.t) {
      case "i": d.putInteger(key, v.v); break;
      case "u": d.putUnitDouble(key, tid(v.u), v.v); break;
      case "e": d.putEnumerated(key, tid(v.e), tid(v.v)); break;
      case "o": d.putObject(key, tid(v.c), mk(v.o)); break;
      case "r": d.putReference(key, v.ref); break;
      case "l": d.putList(key, mkL(v.a)); break;
      case "c": d.putClass(key, tid(v.v)); break;
    }
  }
  function mk(o) { var d = new ActionDescriptor(); for (var k in o) if (o.hasOwnProperty(k)) put(d, k, o[k]); return d; }
  function ex(ev, o) { return executeAction(tid(ev), mk(o), DialogModes.NO); }
  function refLayer() { var r = new ActionReference(); r.putEnumerated(tid("#Lyr "), tid("#Ordn"), tid("#Trgt")); return r; }
  function refChan(code) { var r = new ActionReference(); r.putEnumerated(tid("#Chnl"), tid("#Chnl"), tid(code)); return r; }
  function refClass(c) { var r = new ActionReference(); r.putClass(tid(c)); return r; }
  function setLayer(o) { ex("#setd", { "#null": RF(refLayer()), "#T   ": O("#Lyr ", o) }); }
  function addAdj(typeClass, typeObj, name) {
    ex("#Mk  ", { "#null": RF(refClass("#AdjL")), "#Usng": O("#AdjL", { "#Type": O(typeClass, typeObj) }) });
    if (name) app.activeDocument.activeLayer.name = name;
    return app.activeDocument.activeLayer;
  }
  var PRESET = { presetKind: E("presetKindType", "presetKindCustom") };
  function adjLevels(lo, hi, gam, olo, ohi, chan) {
    var a = O("#LvlA", { "#Chnl": RF(refChan(chan || "#Cmps")), "#Inpt": L([I(lo), I(hi)]), "#Gmm ": gam, "#Otpt": L([I(olo), I(ohi)]) });
    return a;
  }
  function adjCurvePts(pts, chan) {
    var p = [];
    for (var i = 0; i < pts.length; i++) p.push(O("#Pnt ", { "#Hrzn": pts[i][0], "#Vrtc": pts[i][1] }));
    return O("#CrvA", { "#Chnl": RF(refChan(chan || "#Cmps")), "#Crv ": L(p) });
  }
  function rgbc(r, g, b) { return O("#RGBC", { "#Rd  ": r, "#Grn ": g, "#Bl  ": b }); }
  function bmEnum(code) { return E("#BlnM", code); }   // "#Nrml","#Mltp","#Scrn"
  // unit charIDs are "#Prc", "#Pxl", "#Ang" themselves, so they need the "#" marker in front (fixed 2026-10-16:
  // the single-# form passed 3-char codes to charIDToTypeID -> "Illegal Argument" in every unit-bearing descriptor)
  var PCT = "##Prc", PX = "##Pxl", ANG = "##Ang";
  function fx(kind, o) { var e = { "#enab": true, "#Md  ": bmEnum("#Nrml"), "#Clr ": rgbc(0, 0, 0), "#Opct": U(PCT, 100) }; for (var k in o) if (o.hasOwnProperty(k)) e[k] = o[k]; var r = {}; r[kind] = O(kind, e); return r; }
  function setEffects(list) {   // list: array of objects from fx(); merged into one Lefx
    var all = { "#Scl ": U(PCT, 100) };
    for (var i = 0; i < list.length; i++) for (var k in list[i]) if (list[i].hasOwnProperty(k)) all[k] = list[i][k];
    setLayer({ "#Lefx": O("#Lefx", all) });
  }
  function shadowOpts(size, spread, angle, dist, extra) {
    var o = { "#uglg": false, "#lagl": U(ANG, angle), "#Dstn": U(PX, dist), "#Ckmt": U(PX, spread), "blur": U(PX, size), "#Nose": U(PCT, 0), "#AntA": false, "layerConceals": true };
    for (var k in extra || {}) o[k] = extra[k];
    return o;
  }

  // ---------- state, manifest, logging ----------
  ensure(OUT); ensure(OUT + "/inputs"); ensure(CTRL);
  var manifest = { schema: "hsk.oracle.session.manifest@1", app: APP, started_utc: nowIso(), script: "hsk_oracle_photoshop.jsx", app_version: String(app.version), app_build: String(app.build), locale: String(app.locale), os: String($.os), entries: [], errors: [], state: {} };
  function logErr(id, variant, e) { var m = id + "/" + variant + ": " + (e && e.message ? e.message : String(e)) + (e && e.line ? " (line " + e.line + ")" : ""); manifest.errors.push(m); atxt(OUT + "/errors.log", m + "\n"); }
  function flushManifest() { manifest.finished_utc = nowIso(); wtxt(OUT + "/manifest.json", ser(manifest)); }
  var onlyTxt = rtxt(CTRL + "/ps_only.txt"), ONLY = null;
  if (onlyTxt) { ONLY = {}; var ls = onlyTxt.split(/\r?\n/); for (var q = 0; q < ls.length; q++) if (ls[q].replace(/\s/g, "")) ONLY[ls[q].replace(/\s/g, "")] = true; }
  function want(id) { return !ONLY || ONLY[id] === true; }

  var saved = { dialogs: app.displayDialogs, ruler: app.preferences.rulerUnits, colorSettings: null };
  try { saved.colorSettings = String(app.colorSettings); } catch (e0) { saved.colorSettings = null; }
  manifest.state.colorSettings_before = saved.colorSettings;
  manifest.state.fixture_provenance = rtxt(OUT + "/inputs/FIXTURE_PROVENANCE.json");
  manifest.state.blend_gamma_readable ="UNVERIFIED: no DOM property for 'Blend RGB Colors Using Gamma'; only the preset NAME is read/restored";

  if (app.documents.length > 0) {   // never touch the Operator's open documents
    logErr("PRE", "open-documents", "Close all Photoshop documents, then run again.");
    flushManifest();
    return;
  }

  // ---------- documents and fixtures ----------
  var W = 256, H = 64, SRGB = "sRGB IEC61966-2.1";
  function newDoc(name, w, h, bits, mode) {
    var m = mode || NewDocumentMode.RGB, b = bits || BitsPerChannelType.EIGHT;
    if (m === NewDocumentMode.LAB) return app.documents.add(w, h, 72, name, m, DocumentFill.TRANSPARENT, 1, b);
    return app.documents.add(w, h, 72, name, m, DocumentFill.TRANSPARENT, 1, b, SRGB);
  }
  function col(r, g, b) { var c = new SolidColor(); c.rgb.red = r; c.rgb.green = g; c.rgb.blue = b; return c; }
  function fillRect(doc, layer, x, y, w, h, c, feather) {
    doc.activeLayer = layer;
    doc.selection.select([[x, y], [x + w, y], [x + w, y + h], [x, y + h]], SelectionType.REPLACE, feather || 0, true);
    doc.selection.fill(c);
    doc.selection.deselect();
  }
  function fillCircle(doc, layer, cx, cy, r, c) {
    var pts = [], n = 96;
    for (var i = 0; i < n; i++) { var a = 2 * Math.PI * i / n; pts.push([cx + r * Math.cos(a), cy + r * Math.sin(a)]); }
    doc.activeLayer = layer; doc.selection.select(pts, SelectionType.REPLACE, 0, true); doc.selection.fill(c); doc.selection.deselect();
  }
  var fixtures = {};   // name -> Document (kept open; closed at the end)
  function saveInput(doc, name) {
    var o = new TiffSaveOptions(); o.layers = false; o.transparency = true; o.embedColorProfile = true; o.imageCompression = TIFFEncoding.NONE; o.byteOrder = ByteOrder.IBM;
    doc.saveAs(new File(OUT + "/inputs/" + name + ".tif"), o, true);
  }
  // Fast path: when inputs/<name>.tif already exists (an earlier run, or oracle-session/gen_ps_fixtures.py,
  // which writes the same pixel definitions bit-exactly), open it instead of drawing it with one-pixel fills
  // (7473 fills for all six fixtures, about 0.5 s each). The source used is recorded in manifest.state.
  function mkFixture(name, w, h, drawFn, mode, bits) {
    var src = new File(OUT + "/inputs/" + name + ".tif"), d;
    if (!manifest.state.fixture_source) manifest.state.fixture_source = {};
    if (src.exists) {
      d = app.open(src);
      if (d.width.as("px") !== w || d.height.as("px") !== h) throw new Error("existing input " + name + ".tif has wrong size");
      manifest.state.fixture_source[name] = "opened existing inputs/" + name + ".tif (see inputs/FIXTURE_PROVENANCE.json)";
    } else {
      d = newDoc("fx_" + name, w, h, bits, mode); drawFn(d, d.artLayers[0]); saveInput(d, name);
      manifest.state.fixture_source[name] = "drawn in script and saved to inputs/" + name + ".tif";
    }
    fixtures[name] = d; return d;
  }
  // F_GRAY_RAMP_256x64: pixel (x,y) = (x,x,x), x=0..255
  function drawRamp(d, l) { for (var x = 0; x < 256; x++) fillRect(d, l, x, 0, 1, H, col(x, x, x)); }
  // F_GRAY_RAMP_REV_256x64: pixel (x,y) = (255-x) gray
  function drawRampRev(d, l) { for (var x = 0; x < 256; x++) fillRect(d, l, x, 0, 1, H, col(255 - x, 255 - x, 255 - x)); }
  // F_RGBCMY_BANDS_256x48: band k (rows 8k..8k+7) = ramps of R,G,B,C,M,Y: channel(s)=x, others 0
  function drawBands(d, l) {
    var bands = [[1, 0, 0], [0, 1, 0], [0, 0, 1], [0, 1, 1], [1, 0, 1], [1, 1, 0]];
    for (var k = 0; k < 6; k++) for (var x = 0; x < 256; x++) fillRect(d, l, x, 8 * k, 1, 8, col(bands[k][0] * x, bands[k][1] * x, bands[k][2] * x));
  }
  // F_RGB_LATTICE17_289x17: pixel (x = ri*17+gi, y = bi) = round(k*255/16) for k=ri,gi,bi in 0..16
  function drawLattice(d, l) {
    function v(k) { return Math.round(k * 255 / 16); }
    for (var ri = 0; ri < 17; ri++) for (var gi = 0; gi < 17; gi++) for (var bi = 0; bi < 17; bi++) fillRect(d, l, ri * 17 + gi, bi, 1, 1, col(v(ri), v(gi), v(bi)));
  }
  // F_GRID_HRAMP_256 (x gray) and F_GRID_VRAMP_256 (y gray), 256x256
  function drawH256(d, l) { for (var x = 0; x < 256; x++) fillRect(d, l, x, 0, 1, 256, col(x, x, x)); }
  function drawV256(d, l) { for (var y = 0; y < 256; y++) fillRect(d, l, 0, y, 256, 1, col(y, y, y)); }
  var white = col(255, 255, 255), black = col(0, 0, 0);
  function getFix(name) {
    if (fixtures[name]) return fixtures[name];
    switch (name) {
      case "F_GRAY_RAMP_256x64": return mkFixture(name, W, H, drawRamp);
      case "F_GRAY_RAMP_REV_256x64": return mkFixture(name, W, H, drawRampRev);
      case "F_RGBCMY_BANDS_256x48": return mkFixture(name, 256, 48, drawBands);
      case "F_RGB_LATTICE17_289x17": return mkFixture(name, 289, 17, drawLattice);
      case "F_GRID_HRAMP_256": return mkFixture(name, 256, 256, drawH256);
      case "F_GRID_VRAMP_256": return mkFixture(name, 256, 256, drawV256);
    }
    throw new Error("unknown fixture " + name);
  }
  // paste a fixture into dst above `above` (a layer in dst); returns the new layer
  function pasteFix(fixName, dst, above, name) {
    var f = getFix(fixName);
    app.activeDocument = f; f.activeLayer = f.artLayers[0]; f.selection.selectAll(); f.selection.copy(); f.selection.deselect();
    app.activeDocument = dst; if (above) dst.activeLayer = above;
    var l = dst.paste(); if (name) l.name = name; return l;
  }
  function solid(doc, container, r, g, b, x, y, w, h, name) {
    var l = (container || doc).artLayers.add(); if (name) l.name = name; fillRect(doc, l, x === undefined ? 0 : x, y === undefined ? 0 : y, w || doc.width.as("px"), h || doc.height.as("px"), col(r, g, b)); return l;
  }
  function group(doc, name, mode) { var g = doc.layerSets.add(); g.name = name; g.blendMode = mode || BlendMode.PASSTHROUGH; return g; }
  // PS 27.11 pastes INTO an empty active pixel layer (paste() returns the placeholder itself, same id), so the
  // placeholder is removed only when paste created a separate layer (observed 2026-10-16: "Object is invalid").
  function pasteInGroup(fixName, doc, g, name) { var ph = g.artLayers.add(); doc.activeLayer = ph; var phId = ph.id; var l = pasteFix(fixName, doc, ph, name); if (l.id !== phId) ph.remove(); return l; }
  function shapeLayer(doc, kind, container) {   // 128x128 canvas: square 32..96, circle r=32, soft = square with 4 px feather
    var l = (container || doc).artLayers.add(); l.name = "shape_" + kind;
    if (kind === "square") fillRect(doc, l, 32, 32, 64, 64, white);
    else if (kind === "circle") fillCircle(doc, l, 64, 64, 32, white);
    else if (kind === "soft") fillRect(doc, l, 32, 32, 64, 64, white, 4);
    return l;
  }

  // ---------- export ----------
  function fileSafe(s) { return String(s).replace(/[^A-Za-z0-9_.+-]/g, "_"); }
  function exportDoc(doc, caseId, variant, params, alpha) {
    app.activeDocument = doc;
    var fname = caseId + "__" + fileSafe(variant) + ".tif", f = new File(OUT + "/" + fname);
    var o = new TiffSaveOptions(); o.layers = false; o.alphaChannels = false; o.transparency = alpha === true; o.embedColorProfile = true; o.imageCompression = TIFFEncoding.NONE; o.byteOrder = ByteOrder.IBM; o.interleaveChannels = true;
    doc.saveAs(f, o, true);
    var prof = ""; try { prof = String(doc.colorProfileName); } catch (e1) { }
    manifest.entries.push({ case_id: caseId, variant: variant, file: fname, status: "ok", bits: String(doc.bitsPerChannel), mode: String(doc.mode), profile: prof, w: doc.width.as("px"), h: doc.height.as("px"), alpha: alpha === true, params: params || {} });
  }
  function variant(caseId, name, fn) {   // run one variant, isolate failures
    var before = null;
    try { fn(); } catch (e) { logErr(caseId, name, e); manifest.entries.push({ case_id: caseId, variant: name, status: "error" }); }
    closeWork();
  }
  var keepOpen = null;
  function closeWork() {   // close every document except fixtures
    var guard = 0;
    for (var i = app.documents.length - 1; i >= 0 && guard < 500; i--, guard++) {
      var d = app.documents[i], isFix = false;
      for (var k in fixtures) if (fixtures[k] === d) isFix = true;
      if (!isFix) { try { d.close(SaveOptions.DONOTSAVECHANGES); } catch (e2) { } }
    }
  }

  // ---------- cases ----------
  var CASES = [];
  function C(id, group, fn) { CASES.push({ id: id, group: group || "main", fn: fn }); }

  // PS-C01 (#1): adjustment layer over a semi-transparent backdrop inside a Normal group; alpha preserved?
  C("PS-C01", "main", function () {
    var vs = [["base_no_adj", null], ["invert_normal", BlendMode.NORMAL], ["invert_multiply", BlendMode.MULTIPLY]];
    for (var i = 0; i < vs.length; i++) (function (v) {
      variant("PS-C01", v[0], function () {
        var d = newDoc("c01", W, H); var g = group(d, "G", BlendMode.NORMAL);
        var r = pasteInGroup("F_GRAY_RAMP_256x64", d, g, "ramp"); r.opacity = 50;
        if (v[1] !== null) { d.activeLayer = r; var a = addAdj("#Invr", {}, "adj"); a.blendMode = v[1]; }
        exportDoc(d, "PS-C01", v[0], { backdrop: "ramp at layer opacity 50 in Normal group on transparent", adj_mode: String(v[1]) }, true);
      });
    })(vs[i]);
  });

  // PS-C02 (#2): Levels, composite vs red record order; gamma ramp at 8/16 bit; DOM adjustLevels reference
  C("PS-C02", "main", function () {
    var bitsList = [["8", BitsPerChannelType.EIGHT], ["16", BitsPerChannelType.SIXTEEN]];
    for (var bi = 0; bi < bitsList.length; bi++) (function (bt) {
      var specs = [["comp_g0.5", [adjLevels(0, 255, 0.5, 0, 255)]], ["comp_g1", [adjLevels(0, 255, 1, 0, 255)]], ["comp_g2", [adjLevels(0, 255, 2, 0, 255)]]];
      if (bt[0] === "8") {
        specs.push(["comp_in20_230_g1.5_out10_240", [adjLevels(20, 230, 1.5, 10, 240)]]);
        specs.push(["red_in10_200_g0.8", [adjLevels(10, 200, 0.8, 0, 255, "#Rd  ")]]);
        specs.push(["comp_and_red", [adjLevels(20, 230, 1.5, 10, 240), adjLevels(10, 200, 0.8, 0, 255, "#Rd  ")]]);
      } else specs.push(["comp_and_red", [adjLevels(20, 230, 1.5, 10, 240), adjLevels(10, 200, 0.8, 0, 255, "#Rd  ")]]);
      for (var i = 0; i < specs.length; i++) (function (s) {
        variant("PS-C02", bt[0] + "bit_" + s[0], function () {
          var d = newDoc("c02", W, H, bt[1]); var r = pasteFix("F_GRAY_RAMP_256x64", d, d.artLayers[0], "ramp");
          addAdj("#Lvls", { "#Adjs": L(s[1]), presetKind: PRESET.presetKind }, "levels");
          exportDoc(d, "PS-C02", bt[0] + "bit_" + s[0], { depth: bt[0], levels: s[0] }, false);
        });
      })(specs[i]);
    })(bitsList[bi]);
    variant("PS-C02", "dom_adjustLevels_20_230_g1.5_10_240", function () {
      var d = newDoc("c02d", W, H); var r = pasteFix("F_GRAY_RAMP_256x64", d, d.artLayers[0], "ramp"); d.activeLayer = r;
      r.adjustLevels(20, 230, 1.5, 10, 240);
      exportDoc(d, "PS-C02", "dom_adjustLevels_20_230_g1.5_10_240", { api: "ArtLayer.adjustLevels (pixel layer, verified DOM)" }, false);
    });
  });

  // PS-C03 (#3): Curves. Strong Contrast (RGB) read from the installed ACV (read only; nothing copied to assets), 3-point overshoot, Negative, identity
  function readAcvMaster(path) {
    var f = new File(path); if (!f.exists) throw new Error("ACV not found: " + path);
    f.encoding = "BINARY"; f.open("r"); var s = f.read(); f.close();
    function u16(i) { return (s.charCodeAt(i) << 8) | s.charCodeAt(i + 1); }
    var ver = u16(0), cnt = u16(2), n = u16(4), pts = [];
    for (var i = 0; i < n; i++) { var out = u16(6 + 4 * i), inp = u16(8 + 4 * i); pts.push([inp, out]); }
    return { version: ver, curves: cnt, master: pts };
  }
  C("PS-C03", "main", function () {
    var strong = null, neg = null;
    try { strong = readAcvMaster(Folder.appPackage.fsName.replace(/\\/g, "/") + "/Presets/Curves/Strong Contrast (RGB).acv"); } catch (e) { logErr("PS-C03", "acv-strong", e); }
    try { neg = readAcvMaster(Folder.appPackage.fsName.replace(/\\/g, "/") + "/Presets/Curves/Negative (RGB).acv"); } catch (e2) { logErr("PS-C03", "acv-negative", e2); }
    var vs = [["identity", [[0, 0], [255, 255]]], ["overshoot3", [[0, 0], [128, 230], [255, 255]]]];
    if (strong) vs.push(["strong_contrast_master", strong.master]);
    if (neg) vs.push(["negative_master", neg.master]);
    manifest.state.acv_points = { strong: strong ? strong.master : null, negative: neg ? neg.master : null };
    for (var i = 0; i < vs.length; i++) (function (v) {
      variant("PS-C03", v[0], function () {
        var d = newDoc("c03", W, H); pasteFix("F_GRAY_RAMP_256x64", d, d.artLayers[0], "ramp");
        addAdj("#Crvs", { "#Adjs": L([adjCurvePts(v[1])]), presetKind: PRESET.presetKind }, "curves");
        exportDoc(d, "PS-C03", v[0], { points_input_output: v[1] }, false);
      });
    })(vs[i]);
  });

  // PS-C04 (#4): Hue/Saturation master + colorize on the 17^3 lattice
  C("PS-C04", "main", function () {
    var vs = [["identity", 0, 0, 0, false], ["hue+60", 60, 0, 0, false], ["sat+50", 0, 50, 0, false], ["sat-50", 0, -50, 0, false], ["light+50", 0, 0, 50, false], ["light-50", 0, 0, -50, false], ["colorize_h30_s50_l0", 30, 50, 0, true]];
    for (var i = 0; i < vs.length; i++) (function (v) {
      variant("PS-C04", v[0], function () {
        var d = newDoc("c04", 289, 17); pasteFix("F_RGB_LATTICE17_289x17", d, d.artLayers[0], "lattice");
        addAdj("#HStr", { "#Clrz": v[4], "#Adjs": L([O("#Hst2", { "#H   ": I(v[1]), "#Strt": I(v[2]), "#Lght": I(v[3]) })]), presetKind: PRESET.presetKind }, "huesat");
        exportDoc(d, "PS-C04", v[0], { hue: v[1], saturation: v[2], lightness: v[3], colorize: v[4] }, false);
      });
    })(vs[i]);
  });

  // PS-C05 (#5): Brightness/Contrast modern and legacy on the gray ramp
  C("PS-C05", "main", function () {
    var pairs = [[50, 0], [-50, 0], [0, 50], [0, -50], [0, 100], [0, -100], [100, -100]];
    for (var m = 0; m < 2; m++) for (var i = 0; i < pairs.length; i++) (function (legacy, p) {
      if (!legacy && i > 3) return;   // modern: four rows; legacy: all seven (W3C-relayed corner cases)
      var nm = (legacy ? "legacy" : "modern") + "_b" + p[0] + "_c" + p[1];
      variant("PS-C05", nm, function () {
        var d = newDoc("c05", W, H); pasteFix("F_GRAY_RAMP_256x64", d, d.artLayers[0], "ramp");
        addAdj("#BrgC", { "#Brgh": I(p[0]), "#Cntr": I(p[1]), useLegacy: legacy }, "bc");
        exportDoc(d, "PS-C05", nm, { brightness: p[0], contrast: p[1], useLegacy: legacy }, false);
      });
    })(m === 1, pairs[i]);
  });

  // PS-C06 (#6): Exposure at 8-bit and 32-bit float
  C("PS-C06", "main", function () {
    var depths = [["8", BitsPerChannelType.EIGHT], ["32", BitsPerChannelType.THIRTYTWO]];
    var vs = [["identity", 0, 0, 1], ["ev+1_off0.1_g1.5", 1, 0.1, 1.5], ["ev+1_only", 1, 0, 1], ["off0.1_only", 0, 0.1, 1], ["g1.5_only", 0, 0, 1.5]];
    for (var di = 0; di < 2; di++) for (var i = 0; i < vs.length; i++) (function (dp, v) {
      variant("PS-C06", dp[0] + "bit_" + v[0], function () {
        var d = newDoc("c06", W, H, dp[1]); pasteFix("F_GRAY_RAMP_256x64", d, d.artLayers[0], "ramp");
        addAdj("#Exps", { "#Exps": v[1], "#Ofst": v[2], gammaCorrection: v[3], presetKind: PRESET.presetKind }, "exposure");
        exportDoc(d, "PS-C06", dp[0] + "bit_" + v[0], { depth: dp[0], exposure_ev: v[1], offset: v[2], gamma: v[3], note: "32-bit docs are linear-light: pasted sRGB ramp values are converted by Photoshop" }, false);
      });
    })(depths[di], vs[i]);
  });

  // PS-C07 (#7): Threshold 128 on lattice; boundary on the ramp (127,128,129); DOM threshold reference
  C("PS-C07", "main", function () {
    variant("PS-C07", "lattice_t128", function () {
      var d = newDoc("c07", 289, 17); pasteFix("F_RGB_LATTICE17_289x17", d, d.artLayers[0], "lattice");
      addAdj("#Thrs", { "#Lvl ": I(128) }, "threshold");
      exportDoc(d, "PS-C07", "lattice_t128", { level: 128 }, false);
    });
    var ts = [127, 128, 129];
    for (var i = 0; i < 3; i++) (function (t) {
      variant("PS-C07", "ramp_t" + t, function () {
        var d = newDoc("c07r", W, H); pasteFix("F_GRAY_RAMP_256x64", d, d.artLayers[0], "ramp");
        addAdj("#Thrs", { "#Lvl ": I(t) }, "threshold");
        exportDoc(d, "PS-C07", "ramp_t" + t, { level: t }, false);
      });
    })(ts[i]);
    variant("PS-C07", "dom_lattice_t128", function () {
      var d = newDoc("c07d", 289, 17); var l = pasteFix("F_RGB_LATTICE17_289x17", d, d.artLayers[0], "lattice"); d.activeLayer = l; l.threshold(128);
      exportDoc(d, "PS-C07", "dom_lattice_t128", { api: "ArtLayer.threshold (verified DOM)", level: 128 }, false);
    });
  });

  // PS-C08 (#8): Posterize 2, 4, 255 on the ramp; DOM posterize reference
  C("PS-C08", "main", function () {
    var ls = [2, 4, 255];
    for (var i = 0; i < 3; i++) (function (n) {
      variant("PS-C08", "adj_levels" + n, function () {
        var d = newDoc("c08", W, H); pasteFix("F_GRAY_RAMP_256x64", d, d.artLayers[0], "ramp");
        addAdj("#Pstr", { "#Lvls": I(n) }, "posterize");
        exportDoc(d, "PS-C08", "adj_levels" + n, { levels: n }, false);
      });
      variant("PS-C08", "dom_levels" + n, function () {
        var d = newDoc("c08d", W, H); var l = pasteFix("F_GRAY_RAMP_256x64", d, d.artLayers[0], "ramp"); d.activeLayer = l; l.posterize(n);
        exportDoc(d, "PS-C08", "dom_levels" + n, { api: "ArtLayer.posterize (verified DOM)", levels: n }, false);
      });
    })(ls[i]);
  });

  // layer Blend-If (UNVERIFIED keys). side: "this" (Src*) or "under" (Dst*); chan: "#Gry ","#Rd  "; split sliders black(lo,hi) white(lo,hi)
  function blendIf(side, chan, bk, wh) {
    // Keys measured on PS 27.11 (2026-10-16): SrcB black-low, Srcl black-high, Srcm white-low, SrcW white-high;
    // the accepted Dst set is DstB, Dstl, Dstt, DstW (Dstt = white-low by analogy, not measured). The earlier
    // Srcb/Srcw/Dstb/Dstw keys failed with "Could not complete the command because of a program error".
    var p = side === "this" ? "Src" : "Dst", o = { "#Chnl": RF(refChan(chan)) };
    function wlo(s) { return s === "Src" ? "m" : "t"; }
    o["#" + p + "B"] = I(bk[0]); o["#" + p + "l"] = I(bk[1]); o["#" + p + wlo(p)] = I(wh[0]); o["#" + p + "W"] = I(wh[1]);
    // the opposite side stays at identity
    var q = side === "this" ? "Dst" : "Src";
    o["#" + q + "B"] = I(0); o["#" + q + "l"] = I(0); o["#" + q + wlo(q)] = I(255); o["#" + q + "W"] = I(255);
    return O("#Blnd", o);
  }
  function setBlendIf(list) { setLayer({ "#Blnd": L(list) }); }

  // PS-C09 (#9): Blend-If split sliders on RGBCMY bands over a gray ramp backdrop
  C("PS-C09", "main", function () {
    var vs = [["none", []], ["this_gray_50_100__200_240", [blendIf("this", "#Gry ", [50, 100], [200, 240])]], ["under_gray_50_100__200_240", [blendIf("under", "#Gry ", [50, 100], [200, 240])]],
      ["this_red_50_100__200_240", [blendIf("this", "#Rd  ", [50, 100], [200, 240])]], ["under_red_50_100__200_240", [blendIf("under", "#Rd  ", [50, 100], [200, 240])]],
      ["this_gray_hard_100_100__200_200", [blendIf("this", "#Gry ", [100, 100], [200, 200])]]];
    for (var i = 0; i < vs.length; i++) (function (v) {
      variant("PS-C09", v[0], function () {
        var d = newDoc("c09", 256, 48); var bk = pasteFix("F_GRAY_RAMP_256x64", d, d.artLayers[0], "backdrop");
        var top = pasteFix("F_RGBCMY_BANDS_256x48", d, bk, "bands"); d.activeLayer = top;
        if (v[1].length) setBlendIf(v[1]);
        exportDoc(d, "PS-C09", v[0], { blend_if: v[0] }, false);
      });
    })(vs[i]);
  });

  // PS-C10 (#10): mask density and feather on a hard mask (left half revealed): white layer over black backdrop
  C("PS-C10", "main", function () {
    var vs = [["d100_f0", 100, 0], ["d50_f0", 50, 0], ["d100_f2", 100, 2], ["d100_f10", 100, 10], ["d100_f50", 100, 50], ["d50_f10", 50, 10]];
    for (var i = 0; i < vs.length; i++) (function (v) {
      variant("PS-C10", v[0], function () {
        var d = newDoc("c10", W, H); var bg = solid(d, null, 0, 0, 0, 0, 0, W, H, "black"); var l = solid(d, null, 255, 255, 255, 0, 0, W, H, "white"); d.activeLayer = l;
        ex("#Mk  ", { "#Nw  ": CL("#Chnl"), "#At  ": RF(refChan("#Msk ")), "#Usng": E("#UsrM", "#RvlA") });
        d.selection.select([[128, 0], [256, 0], [256, 64], [128, 64]]); d.selection.fill(black); d.selection.deselect();   // active channel = the mask
        d.activeLayer = l;
        setLayer({ userMaskDensity: U(PCT, v[1]), userMaskFeather: U(PX, v[2]) });
        exportDoc(d, "PS-C10", v[0], { density_percent: v[1], feather_px: v[2] }, false);
      });
    })(vs[i]);
  });

  // PS-C11 (#11): knockout none/shallow/deep in nested groups
  C("PS-C11", "main", function () {
    var ks = ["none", "shallow", "deep"];
    for (var i = 0; i < 3; i++) (function (k) {
      variant("PS-C11", k, function () {
        var d = newDoc("c11", W, H); var base = pasteFix("F_GRAY_RAMP_256x64", d, d.artLayers[0], "base");
        var go = group(d, "outer", BlendMode.NORMAL); var a = solid(d, go, 255, 0, 0, 0, 0, 160, H, "red_left");
        var gi = group(d, "inner", BlendMode.NORMAL);   // inner group is a child of the active container
        var yl = solid(d, gi, 255, 255, 0, 64, 0, 128, H, "yellow_knock"); d.activeLayer = yl; yl.fillOpacity = 50;
        setLayer({ knockout: E("knockout", k) });
        exportDoc(d, "PS-C11", k, { knockout: k, structure: "ramp / outer(red left) / inner(yellow, fill 50)" }, false);
      });
    })(ks[i]);
  });

  // ----- effect cases (AM layer styles: UNVERIFIED) -----
  function effDoc(shape, backdrop) {
    var d = newDoc("eff", 128, 128);
    if (backdrop) solid(d, null, backdrop[0], backdrop[1], backdrop[2], 0, 0, 128, 128, "backdrop");
    var l = shapeLayer(d, shape); d.activeLayer = l; return { d: d, l: l };
  }
  var SHAPES = ["square", "circle"];
  // PS-C12 (#12): drop shadow
  C("PS-C12", "effects", function () {
    var sizes = [2, 10, 40], spreads = [0, 50, 100], angles = [120, -60, 0, 90, 180];
    for (var s = 0; s < 2; s++) for (var a = 0; a < 3; a++) for (var b = 0; b < 3; b++) for (var c = 0; c < 5; c++) (function (sh, sz, sp, an) {
      var nm = sh + "_s" + sz + "_sp" + sp + "_a" + an + "_d5";
      variant("PS-C12", nm, function () { var e = effDoc(sh); setEffects([fx("#DrSh", shadowOpts(sz, sp, an, 5))]); exportDoc(e.d, "PS-C12", nm, { shape: sh, size: sz, spread: sp, angle: an, distance: 5, colour: "000000", mode: "normal", opacity: 100, global_light: false }, true); });
    })(SHAPES[s], sizes[a], spreads[b], angles[c]);
    for (var s2 = 0; s2 < 2; s2++) (function (sh) {
      variant("PS-C12", sh + "_fractional_d2.5_a30_s10", function () { var e = effDoc(sh); setEffects([fx("#DrSh", shadowOpts(10, 0, 30, 2.5))]); exportDoc(e.d, "PS-C12", sh + "_fractional_d2.5_a30_s10", { distance: 2.5, angle: 30, size: 10, spread: 0 }, true); });
    })(SHAPES[s2]);
    var ko = [["fill100_knocks", 100, true], ["fill100_noknock", 100, false], ["fill50_knocks", 50, true], ["fill50_noknock", 50, false], ["fill0_knocks", 0, true], ["fill0_noknock", 0, false]];
    for (var k = 0; k < ko.length; k++) (function (v) {
      variant("PS-C12", "soft_" + v[0], function () { var e = effDoc("soft"); e.l.fillOpacity = v[1]; setEffects([fx("#DrSh", shadowOpts(10, 0, 120, 8, { layerConceals: v[2] }))]); exportDoc(e.d, "PS-C12", "soft_" + v[0], { fill: v[1], layer_knocks_out_shadow: v[2] }, true); });
    })(ko[k]);
  });

  // PS-C13 (#13): solid stroke
  C("PS-C13", "effects", function () {
    var shapes = ["square", "circle", "soft"], pos = [["outside", "#OutF"], ["inside", "#InsF"], ["center", "#CtrF"]], sizes = [1, 2, 5, 50];
    for (var a = 0; a < 3; a++) for (var b = 0; b < 3; b++) for (var c = 0; c < 4; c++) (function (sh, p, sz) {
      var nm = sh + "_" + p[0] + "_s" + sz;
      variant("PS-C13", nm, function () {
        var e = effDoc(sh); setEffects([fx("#FrFX", { "#Styl": E("#FStl", p[1]), "#PntT": E("#FrFl", "#SClr"), "#Sz  ": U(PX, sz), "#Clr ": rgbc(255, 0, 0) })]);
        exportDoc(e.d, "PS-C13", nm, { shape: sh, position: p[0], size: sz, colour: "FF0000", opacity: 100 }, true);
      });
    })(shapes[a], pos[b], sizes[c]);
    variant("PS-C13", "square_overlay_blue_stroke_red_o5_opacity50", function () {
      var e = effDoc("square");
      setEffects([fx("#SoFi", { "#Clr ": rgbc(0, 0, 255) }), fx("#FrFX", { "#Styl": E("#FStl", "#OutF"), "#PntT": E("#FrFl", "#SClr"), "#Sz  ": U(PX, 5), "#Clr ": rgbc(255, 0, 0), "#Opct": U(PCT, 50) })]);
      exportDoc(e.d, "PS-C13", "square_overlay_blue_stroke_red_o5_opacity50", { note: "stroke above/below colour overlay; stroke opacity over content" }, true);
    });
  });

  // PS-C14 (#14): effect stack interplay
  C("PS-C14", "effects", function () {
    function stack() { return [fx("#DrSh", shadowOpts(6, 0, 120, 8)), fx("#SoFi", { "#Clr ": rgbc(255, 0, 0) }), fx("#FrFX", { "#Styl": E("#FStl", "#OutF"), "#PntT": E("#FrFl", "#SClr"), "#Sz  ": U(PX, 3), "#Clr ": rgbc(0, 0, 255) })]; }
    var vs = [["fill100", function (e) { }], ["fill50", function (e) { e.l.fillOpacity = 50; }], ["fill0", function (e) { e.l.fillOpacity = 0; }], ["opacity50", function (e) { e.l.opacity = 50; }],
      ["blendInterior_on", function (e) { setLayer({ blendInterior: true }); e.l.fillOpacity = 50; }], ["blendInterior_off_fill50", function (e) { setLayer({ blendInterior: false }); e.l.fillOpacity = 50; }]];
    for (var i = 0; i < vs.length; i++) (function (v) {
      variant("PS-C14", v[0], function () { var e = effDoc("square", [200, 200, 200]); setEffects(stack()); v[1](e); exportDoc(e.d, "PS-C14", v[0], {}, false); });
    })(vs[i]);
    var mk = [["mask_hides_effects_on", true], ["mask_hides_effects_off", false]];
    for (var j = 0; j < 2; j++) (function (m) {
      variant("PS-C14", m[0], function () {
        var e = effDoc("square", [200, 200, 200]); setEffects(stack());
        ex("#Mk  ", { "#Nw  ": CL("#Chnl"), "#At  ": RF(refChan("#Msk ")), "#Usng": E("#UsrM", "#RvlA") });
        e.d.selection.select([[64, 0], [128, 0], [128, 128], [64, 128]]); e.d.selection.fill(black); e.d.selection.deselect(); e.d.activeLayer = e.l;
        setLayer({ layerMaskAsGlobalMask: m[1] });
        exportDoc(e.d, "PS-C14", m[0], { note: "mask hides right half of the layer" }, false);
      });
    })(mk[j]);
    variant("PS-C14", "multiply_shadow_over_gradient", function () {
      var e = effDoc("square"); e.d.activeLayer = e.l;
      var bg = pasteFix("F_GRAY_RAMP_256x64", e.d, e.l, "ramp"); bg.move(e.l, ElementPlacement.PLACEAFTER); e.d.activeLayer = e.l;
      setEffects([fx("#DrSh", shadowOpts(6, 0, 120, 8, { "#Md  ": bmEnum("#Mltp") }))]);
      exportDoc(e.d, "PS-C14", "multiply_shadow_over_gradient", { shadow_mode: "multiply" }, false);
    });
    variant("PS-C14", "two_strokes_instance_order", function () {
      var e = effDoc("square");
      function strk(clr, sz) { return O("#FrFX", { "#enab": true, "#Styl": E("#FStl", "#OutF"), "#PntT": E("#FrFl", "#SClr"), "#Md  ": bmEnum("#Nrml"), "#Opct": U(PCT, 100), "#Sz  ": U(PX, sz), "#Clr ": rgbc(clr[0], clr[1], clr[2]) }); }
      setLayer({ "#Lefx": O("#Lefx", { "#Scl ": U(PCT, 100), frameFXMulti: L([strk([255, 0, 0], 8), strk([0, 0, 255], 4)]) }) });   // UNVERIFIED key frameFXMulti
      exportDoc(e.d, "PS-C14", "two_strokes_instance_order", { stroke1: "red 8 px", stroke2: "blue 4 px" }, true);
    });
  });

  // PS-C15 (#15): inner shadow
  C("PS-C15", "effects", function () {
    var sizes = [2, 10, 40], chokes = [0, 50, 100], angles = [120, -60, 0, 90, 180];
    for (var s = 0; s < 2; s++) for (var a = 0; a < 3; a++) for (var b = 0; b < 3; b++) for (var c = 0; c < 5; c++) (function (sh, sz, ck, an) {
      var nm = sh + "_s" + sz + "_ck" + ck + "_a" + an + "_d5";
      variant("PS-C15", nm, function () { var e = effDoc(sh); setEffects([fx("#IrSh", shadowOpts(sz, ck, an, 5))]); exportDoc(e.d, "PS-C15", nm, { shape: sh, size: sz, choke: ck, angle: an, distance: 5 }, true); });
    })(SHAPES[s], sizes[a], chokes[b], angles[c]);
    for (var s2 = 0; s2 < 2; s2++) for (var a2 = 0; a2 < 3; a2++) for (var b2 = 0; b2 < 3; b2++) (function (sh, sz, ck) {
      var nm = sh + "_s" + sz + "_ck" + ck + "_d0";
      variant("PS-C15", nm, function () { var e = effDoc(sh); setEffects([fx("#IrSh", shadowOpts(sz, ck, 120, 0))]); exportDoc(e.d, "PS-C15", nm, { shape: sh, size: sz, choke: ck, distance: 0 }, true); });
    })(SHAPES[s2], sizes[a2], chokes[b2]);
    variant("PS-C15", "soft_s10_ck0_a120_d5", function () { var e = effDoc("soft"); setEffects([fx("#IrSh", shadowOpts(10, 0, 120, 5))]); exportDoc(e.d, "PS-C15", "soft_s10_ck0_a120_d5", {}, true); });
    variant("PS-C15", "square_s10_ck0_a120_d100_distance_larger_than_shape", function () { var e = effDoc("square"); setEffects([fx("#IrSh", shadowOpts(10, 0, 120, 100))]); exportDoc(e.d, "PS-C15", "square_s10_ck0_a120_d100_distance_larger_than_shape", {}, true); });
    variant("PS-C15", "square_s10_ck0_a120_d5_fill0", function () { var e = effDoc("square"); e.l.fillOpacity = 0; setEffects([fx("#IrSh", shadowOpts(10, 0, 120, 5))]); exportDoc(e.d, "PS-C15", "square_s10_ck0_a120_d5_fill0", {}, true); });
  });

  function glowOpts(size, spread, tech, extra) {
    var o = { "#GlwT": E("#BETE", tech === "precise" ? "#PrBL" : "#SfBL"), "#Ckmt": U(PX, spread), "blur": U(PX, size), "#Nose": U(PCT, 0), "#ShdN": U(PCT, 0), "#AntA": false, "#Inpr": U(PCT, 100), "#Md  ": bmEnum("#Scrn"), "#Clr ": rgbc(255, 255, 190) };
    for (var k in extra || {}) o[k] = extra[k];
    return o;
  }
  // PS-C16 (#16): outer glow
  C("PS-C16", "effects", function () {
    var sizes = [2, 10, 40], spreads = [0, 50, 100], techs = ["softer", "precise"];
    for (var s = 0; s < 2; s++) for (var a = 0; a < 3; a++) for (var b = 0; b < 3; b++) for (var c = 0; c < 2; c++) (function (sh, sz, sp, t) {
      var nm = sh + "_s" + sz + "_sp" + sp + "_" + t;
      variant("PS-C16", nm, function () { var e = effDoc(sh); setEffects([fx("#OrGl", glowOpts(sz, sp, t))]); exportDoc(e.d, "PS-C16", nm, { shape: sh, size: sz, spread: sp, technique: t, mode: "screen" }, true); });
    })(SHAPES[s], sizes[a], spreads[b], techs[c]);
    var rng = [100, 50, 25], jit = [0, 50];
    for (var r = 0; r < 3; r++) (function (rg) { variant("PS-C16", "square_s10_sp0_softer_range" + rg, function () { var e = effDoc("square"); setEffects([fx("#OrGl", glowOpts(10, 0, "softer", { "#Inpr": U(PCT, rg) }))]); exportDoc(e.d, "PS-C16", "square_s10_sp0_softer_range" + rg, { range: rg }, true); }); })(rng[r]);
    for (var j = 0; j < 2; j++) (function (jt) { variant("PS-C16", "square_s10_sp0_softer_jitter" + jt, function () { var e = effDoc("square"); setEffects([fx("#OrGl", glowOpts(10, 0, "softer", { "#ShdN": U(PCT, jt) }))]); exportDoc(e.d, "PS-C16", "square_s10_sp0_softer_jitter" + jt, { jitter: jt }, true); }); })(jit[j]);
    variant("PS-C16", "soft_fill0_s10_sp0_softer", function () { var e = effDoc("soft"); e.l.fillOpacity = 0; setEffects([fx("#OrGl", glowOpts(10, 0, "softer"))]); exportDoc(e.d, "PS-C16", "soft_fill0_s10_sp0_softer", {}, true); });
  });

  // PS-C17 (#17): inner glow, source edge/center
  C("PS-C17", "effects", function () {
    var srcs = [["edge", "#SrcE"], ["center", "#SrcC"]], sizes = [2, 10, 40], chokes = [0, 50, 100], techs = ["softer", "precise"];
    for (var s = 0; s < 2; s++) for (var a = 0; a < 3; a++) for (var b = 0; b < 3; b++) for (var c = 0; c < 2; c++) for (var sh = 0; sh < 2; sh++) (function (src, sz, ck, t, shp) {
      var nm = shp + "_" + src[0] + "_s" + sz + "_ck" + ck + "_" + t;
      variant("PS-C17", nm, function () { var e = effDoc(shp); setEffects([fx("#IrGl", glowOpts(sz, ck, t, { "#glwS": E("#IGSr", src[1]) }))]); exportDoc(e.d, "PS-C17", nm, { shape: shp, source: src[0], size: sz, choke: ck, technique: t }, true); });
    })(srcs[s], sizes[a], chokes[b], techs[c], SHAPES[sh]);
    variant("PS-C17", "thin_shape_edge_s40", function () { var d = newDoc("c17t", 128, 128); var l = d.artLayers.add(); fillRect(d, l, 54, 32, 20, 64, white); d.activeLayer = l; setEffects([fx("#IrGl", glowOpts(40, 0, "softer", { "#glwS": E("#IGSr", "#SrcE") }))]); exportDoc(d, "PS-C17", "thin_shape_edge_s40", { note: "shape thinner than 2x size" }, true); });
    variant("PS-C17", "soft_edge_s10_ck0", function () { var e = effDoc("soft"); setEffects([fx("#IrGl", glowOpts(10, 0, "softer", { "#glwS": E("#IGSr", "#SrcE") }))]); exportDoc(e.d, "PS-C17", "soft_edge_s10_ck0", {}, true); });
  });

  // ----- clipping + Blend-If (PS-C18..C23) -----
  function clipRig(opts) {   // backdrop ramp, [bright layer below base], base layer, clipped layer(s)
    var d = newDoc("clip", W, H); var bd = pasteFix("F_GRAY_RAMP_256x64", d, d.artLayers[0], "backdrop"); var below = null;
    if (opts.brightBelow) { below = pasteFix("F_GRAY_RAMP_REV_256x64", d, bd, "bright_below"); }
    var base = pasteFix("F_GRAY_RAMP_256x64", d, below || bd, "base");
    if (opts.baseShape === "circle") { base.remove(); base = d.artLayers.add(); base.name = "base"; fillCircle(d, base, 128, 32, 30, white); }
    return { d: d, base: base };
  }
  function clipped(d, above, kind) {
    var l; d.activeLayer = above;
    if (kind === "multiply") { l = solid(d, null, 200, 160, 120, 0, 0, W, H, "clip_multiply"); l.blendMode = BlendMode.MULTIPLY; }
    else if (kind === "ramp") { l = pasteFix("F_GRAY_RAMP_256x64", d, above, "clip_ramp"); }
    l.move(above, ElementPlacement.PLACEBEFORE); l.grouped = true; return l;
  }
  C("PS-C18", "main", function () {
    var vs = [["noblendif_clbl_on", false, true], ["blendif_this_clbl_on", true, true], ["blendif_this_clbl_off", true, false]];
    for (var i = 0; i < vs.length; i++) (function (v) {
      variant("PS-C18", v[0], function () {
        var r = clipRig({}); clipped(r.d, r.base, "multiply"); r.d.activeLayer = r.base;
        if (v[1]) setBlendIf([blendIf("this", "#Gry ", [50, 100], [200, 240])]);
        setLayer({ blendClipped: v[2] });
        exportDoc(r.d, "PS-C18", v[0], { blend_if_on_base: v[1], blend_clipped_layers_as_group: v[2] }, false);
      });
    })(vs[i]);
  });
  C("PS-C19", "main", function () {
    var vs = [["blendif_under_clbl_on", true], ["blendif_under_clbl_off", false]];
    for (var i = 0; i < vs.length; i++) (function (v) {
      variant("PS-C19", v[0], function () {
        var r = clipRig({ brightBelow: true }); clipped(r.d, r.base, "multiply"); r.d.activeLayer = r.base;
        setBlendIf([blendIf("under", "#Gry ", [50, 100], [200, 240])]); setLayer({ blendClipped: v[1] });
        exportDoc(r.d, "PS-C19", v[0], { blend_clipped_layers_as_group: v[1] }, false);
      });
    })(vs[i]);
  });
  C("PS-C20", "main", function () {
    var vs = [["clipped_this_gray", true], ["clipped_no_blendif", false]];
    for (var i = 0; i < vs.length; i++) (function (v) {
      variant("PS-C20", v[0], function () {
        var r = clipRig({ baseShape: "circle" }); var c = clipped(r.d, r.base, "ramp"); r.d.activeLayer = c;
        if (v[1]) setBlendIf([blendIf("this", "#Gry ", [50, 100], [200, 240])]);
        exportDoc(r.d, "PS-C20", v[0], { base: "circle (alpha shape)" }, true);
      });
    })(vs[i]);
  });
  C("PS-C21", "main", function () {
    var vs = [["under_gray_bright_below", true], ["under_gray_no_bright_below", false]];
    for (var i = 0; i < vs.length; i++) (function (v) {
      variant("PS-C21", v[0], function () {
        var r = clipRig({ brightBelow: v[1] }); var c = clipped(r.d, r.base, "ramp"); r.d.activeLayer = c;
        setBlendIf([blendIf("under", "#Gry ", [50, 100], [200, 240])]);
        exportDoc(r.d, "PS-C21", v[0], { bright_layer_below_base: v[1] }, false);
      });
    })(vs[i]);
  });
  C("PS-C22", "main", function () {
    var vs = [["clipped_invert_this_gray", true], ["clipped_invert_no_blendif", false]];
    for (var i = 0; i < vs.length; i++) (function (v) {
      variant("PS-C22", v[0], function () {
        var r = clipRig({}); r.d.activeLayer = r.base; var a = addAdj("#Invr", {}, "inv_clipped"); a.grouped = true;
        if (v[1]) setBlendIf([blendIf("this", "#Gry ", [50, 100], [200, 240])]);
        exportDoc(r.d, "PS-C22", v[0], {}, false);
      });
    })(vs[i]);
  });
  // PS-C23: reported bug repro: Underlying ranges on a layer stop working when a layer is clipped to the group
  C("PS-C23", "main", function () {
    var vs = [["no_clip", false], ["clipped_layer_to_group", true]];
    for (var i = 0; i < vs.length; i++) (function (v) {
      variant("PS-C23", v[0], function () {
        var d = newDoc("c23", W, H); var bd = pasteFix("F_GRAY_RAMP_256x64", d, d.artLayers[0], "backdrop");
        var g = group(d, "G", BlendMode.NORMAL); var gl = pasteInGroup("F_GRAY_RAMP_REV_256x64", d, g, "in_group"); d.activeLayer = gl;
        setBlendIf([blendIf("under", "#Gry ", [50, 100], [200, 240])]);
        if (v[1]) { d.activeLayer = g; var c = solid(d, null, 200, 160, 120, 0, 0, W, H, "clip_to_group"); c.move(g, ElementPlacement.PLACEBEFORE); c.grouped = true; }
        exportDoc(d, "PS-C23", v[0], { note: "compare with the validation Photoshop build on the Operator's side" }, false);
      });
    })(vs[i]);
  });

  // ----- effects + Blend-If (PS-C24..C27) -----
  function fxBi(caseId, nm, shadowKind, side, fill0, tsOff, interiorOff) {
    variant(caseId, nm, function () {
      var d = newDoc("fxbi", 128, 128); var bd = d.artLayers.add(); bd.name = "backdrop"; for (var x = 0; x < 128; x++) fillRect(d, bd, x, 0, 1, 128, col(x * 2, x * 2, x * 2));
      var l = shapeLayer(d, "square"); d.activeLayer = l; l.fillOpacity = fill0 ? 0 : 100;
      if (shadowKind === "dropshadow") setEffects([fx("#DrSh", shadowOpts(6, 0, 120, 8))]);
      else setEffects([fx("#IrSh", shadowOpts(6, 0, 120, 4)), fx("#SoFi", { "#Clr ": rgbc(255, 0, 0), "#Opct": U(PCT, 50) })]);
      if (side) setBlendIf([blendIf(side, "#Gry ", [50, 100], [200, 240])]);
      if (tsOff) setLayer({ transparencyShape: false });
      if (interiorOff !== undefined) setLayer({ blendInterior: interiorOff });
      exportDoc(d, caseId, nm, { effect: shadowKind, blend_if: side || "none", fill0: fill0, transparency_shapes_layer_off: !!tsOff, blend_interior: interiorOff }, true);
    });
  }
  C("PS-C24", "main", function () {
    fxBi("PS-C24", "ds_none", "dropshadow", null, false, false); fxBi("PS-C24", "ds_this_gray", "dropshadow", "this", false, false);
    fxBi("PS-C24", "ds_this_gray_fill0", "dropshadow", "this", true, false); fxBi("PS-C24", "ds_this_gray_tsl_off", "dropshadow", "this", false, true);
  });
  C("PS-C25", "main", function () { fxBi("PS-C25", "ds_under_gray", "dropshadow", "under", false, false); fxBi("PS-C25", "ds_under_gray_fill0", "dropshadow", "under", true, false); });
  C("PS-C26", "main", function () {
    fxBi("PS-C26", "interior_this_gray_blendinterior_on", "interior", "this", false, false, true); fxBi("PS-C26", "interior_this_gray_blendinterior_off", "interior", "this", false, false, false);
    fxBi("PS-C26", "interior_none_blendinterior_off", "interior", null, false, false, false);
  });
  C("PS-C27", "main", function () {
    var vs = [["child_only", false, true], ["group_only", true, false], ["both", true, true]];
    for (var i = 0; i < vs.length; i++) (function (v) {
      variant("PS-C27", v[0], function () {
        var d = newDoc("c27", 128, 128); var g = group(d, "G", BlendMode.NORMAL); var c = shapeLayer(d, "square", g);
        if (v[2]) { d.activeLayer = c; setEffects([fx("#DrSh", shadowOpts(4, 0, 120, 6, {}))]); }
        if (v[1]) { d.activeLayer = g; setEffects([fx("#DrSh", shadowOpts(10, 0, 120, 14, { "#Clr ": rgbc(255, 0, 0) }))]); }
        exportDoc(d, "PS-C27", v[0], { group_shadow: v[1], child_shadow: v[2] }, true);
      });
    })(vs[i]);
  });

  // ----- Pass Through group + mask (PS-C28, PS-C29) -----
  function ptRig(gMask, gOpacity, maskInvert, density, gFill) {
    var d = newDoc("pt", W, H); var bd = pasteFix("F_GRAY_RAMP_256x64", d, d.artLayers[0], "backdrop");
    var g = group(d, "PT", BlendMode.PASSTHROUGH);
    var px = solid(d, g, 200, 120, 40, 0, 0, W, H, "pixel_multiply"); px.blendMode = BlendMode.MULTIPLY;
    d.activeLayer = px; addAdj("#Lvls", { "#Adjs": L([adjLevels(30, 220, 1.4, 0, 255)]), presetKind: PRESET.presetKind }, "levels_in_group");
    d.activeLayer = g;
    if (gMask) {
      ex("#Mk  ", { "#Nw  ": CL("#Chnl"), "#At  ": RF(refChan("#Msk ")), "#Usng": E("#UsrM", "#RvlA") });
      // 50 % mask: left half black, right half white
      d.selection.select([[0, 0], [128, 0], [128, 64], [0, 64]]); d.selection.fill(black); d.selection.deselect();
      d.activeLayer = g;
      if (density !== 100) setLayer({ userMaskDensity: U(PCT, density) });
    }
    if (gOpacity !== 100) g.opacity = gOpacity;
    if (gFill !== 100) setLayer({ fillOpacity: U(PCT, gFill) });
    return d;
  }
  C("PS-C28", "main", function () {
    var vs = [["nomask", false, 100, false, 100], ["mask", true, 100, false, 100], ["mask_opacity50", true, 50, false, 100], ["mask_density50", true, 100, false, 50], ["opacity50_nomask", false, 50, false, 100]];
    for (var i = 0; i < vs.length; i++) (function (v) {
      variant("PS-C28", v[0], function () { var d = ptRig(v[1], v[2], v[3], v[4], 100); exportDoc(d, "PS-C28", v[0], { mask: v[1], group_opacity: v[2], mask_density: v[4] }, false); });
    })(vs[i]);
    variant("PS-C28", "mask_inverted_note", function () {
      // mask inversion: Photoshop has no DOM/AM-verified flag here; recorded as NOT_SCRIPTED. Use Properties > Invert by hand if needed.
      manifest.entries.push({ case_id: "PS-C28", variant: "mask_inverted", status: "not_scripted", reason: "no verified invert-mask scripting path" });
    });
  });
  C("PS-C29", "main", function () {
    var vs = [["fill100", 100], ["fill50", 50]];
    for (var i = 0; i < vs.length; i++) (function (v) {
      variant("PS-C29", v[0], function () { var d = ptRig(false, 100, false, 100, v[1]); exportDoc(d, "PS-C29", v[0], { group_fill: v[1] }, false); });
    })(vs[i]);
  });

  // ----- PS-B01: blend-mode grids (backdrop = x gray, source = y gray), 256x256 -----
  var MODES = ["NORMAL", "DISSOLVE", "DARKEN", "MULTIPLY", "COLORBURN", "LINEARBURN", "DARKERCOLOR", "LIGHTEN", "SCREEN", "COLORDODGE", "LINEARDODGE", "LIGHTERCOLOR", "OVERLAY", "SOFTLIGHT", "HARDLIGHT", "VIVIDLIGHT", "LINEARLIGHT", "PINLIGHT", "HARDMIX", "DIFFERENCE", "EXCLUSION", "SUBTRACT", "DIVIDE", "HUE", "SATURATION", "COLORBLEND", "LUMINOSITY"];
  function blendGrid(caseId, tag, bits, opacity, fill) {
    for (var i = 0; i < MODES.length; i++) (function (m) {
      variant(caseId, tag + "_" + m, function () {
        var d = newDoc("bg", 256, 256, bits); var bd = pasteFix("F_GRID_HRAMP_256", d, d.artLayers[0], "backdrop");
        var s = pasteFix("F_GRID_VRAMP_256", d, bd, "source"); s.blendMode = BlendMode[m]; s.opacity = opacity; s.fillOpacity = fill;
        exportDoc(d, caseId, tag + "_" + m, { mode: m, opacity: opacity, fill: fill, depth: String(bits) }, false);
      });
    })(MODES[i]);
  }
  C("PS-B01", "blend", function () {
    blendGrid("PS-B01", "d8_o100_f100", BitsPerChannelType.EIGHT, 100, 100); blendGrid("PS-B01", "d16_o100_f100", BitsPerChannelType.SIXTEEN, 100, 100);
    blendGrid("PS-B01", "d8_o50_f100", BitsPerChannelType.EIGHT, 50, 100); blendGrid("PS-B01", "d8_o100_f50", BitsPerChannelType.EIGHT, 100, 50);
  });

  // ----- PS-L01 (prism CLUT provisional): Lab 16-bit patches off-node -> CMYK with a Lab-indexed output CLUT -----
  // Patch k: L = (l+0.5)*100/32 for l in {3,7,11,15,19,23,27,31}; a = -128+(i+0.5)*255/32 for i in {3,...,31 step 4}; b likewise; 8x8x8 = 512 patches, 1 px each, 512 x 1 canvas.
  function labColor(L_, a_, b_) { var c = new SolidColor(); c.lab.l = L_; c.lab.a = a_; c.lab.b = b_; return c; }
  C("PS-L01", "color", function () {
    var steps = [3, 7, 11, 15, 19, 23, 27, 31], profile = "U.S. Web Coated (SWOP) v2", ints = [["relcol", Intent.RELATIVECOLORIMETRIC], ["perceptual", Intent.PERCEPTUAL]];
    for (var ii = 0; ii < 2; ii++) (function (it) {
      variant("PS-L01", "lab16_to_swop_" + it[0], function () {
        var d = newDoc("l01", 512, 1, BitsPerChannelType.EIGHT, NewDocumentMode.LAB); var l = d.artLayers[0], n = 0;
        for (var a = 0; a < 8; a++) for (var b = 0; b < 8; b++) for (var c = 0; c < 8; c++) { fillRect(d, l, n, 0, 1, 1, labColor((steps[a] + 0.5) * 100 / 32, -128 + (steps[b] + 0.5) * 255 / 32, -128 + (steps[c] + 0.5) * 255 / 32)); n++; }
        d.flatten(); d.bitsPerChannel = BitsPerChannelType.SIXTEEN;
        saveInput(d, "F_LAB_OFFNODE_512x1_8bit_src");
        d.convertProfile(profile, it[1], true, false);   // profile NAME UNVERIFIED for this build; doc.colorProfileName is recorded
        d.changeMode(ChangeMode.CMYK);
        exportDoc(d, "PS-L01", "lab16_to_swop_" + it[0], { patch_order: "L-major (a,b,c loops), x = a*64+b*8+c", L: "(s+0.5)*100/32", a_b: "-128+(s+0.5)*255/32", s_set: steps, intent: it[0], note: "8-bit Lab patches promoted to 16-bit before conversion" }, false);
      });
    })(ints[ii]);
  });
  // PS-L02 (PLR-016): Photoshop-authored 16-bit Lab neutral encoding
  C("PS-L02", "color", function () {
    variant("PS-L02", "lab16_neutral_ramp", function () {
      var d = newDoc("l02", 11, 1, BitsPerChannelType.EIGHT, NewDocumentMode.LAB); var l = d.artLayers[0];
      for (var i = 0; i < 11; i++) fillRect(d, l, i, 0, 1, 1, labColor(i * 10, 0, 0));
      d.flatten(); d.bitsPerChannel = BitsPerChannelType.SIXTEEN;
      var po = new PhotoshopSaveOptions(); d.saveAs(new File(OUT + "/inputs/PS-L02__lab16_neutral_ramp.psd"), po, true);
      exportDoc(d, "PS-L02", "lab16_neutral_ramp", { patches: "L=0,10..100, a=0, b=0", also: "inputs/PS-L02__lab16_neutral_ramp.psd (Photoshop-authored 16-bit Lab PSD)" }, false);
    });
  });

  // ----- main loop -----
  var gammaCases = ["PS-B01", "PS-C28"];
  function runGroup(filterFn) {
    for (var i = 0; i < CASES.length; i++) {
      var c = CASES[i]; if (!filterFn(c) || !want(c.id)) continue;
      try { manifest.state.current_case = c.id; c.fn(); } catch (e) { logErr(c.id, "case", e); }
      closeWork(); flushManifest();
    }
  }
  try {
    app.displayDialogs = DialogModes.NO;
    app.preferences.rulerUnits = Units.PIXELS;
    runGroup(function (c) { return true; });
    // gamma pass: same blend grid (8-bit, 100/100) and the Pass-Through mask case under the Operator-saved gamma 1.00 preset
    if (want("PS-G01") && saved.colorSettings && saved.colorSettings.toLowerCase() !== "custom") {
      var okSet = false;
      try { app.colorSettings = GAMMA_PRESET; okSet = (String(app.colorSettings) === GAMMA_PRESET); } catch (eg) { logErr("PS-G01", "set-colorSettings", eg); }
      manifest.state.gamma_preset_active = okSet;
      if (okSet) {
        blendGridG();
        variant("PS-G01", "ptmask_mask", function () { var d = ptRig(true, 100, false, 100, 100); exportDoc(d, "PS-G01", "ptmask_mask", { gamma_preset: GAMMA_PRESET }, false); });
        variant("PS-G01", "ptmask_nomask", function () { var d = ptRig(false, 100, false, 100, 100); exportDoc(d, "PS-G01", "ptmask_nomask", { gamma_preset: GAMMA_PRESET }, false); });
      }
    } else if (want("PS-G01")) {
      logErr("PS-G01", "skipped", "colorSettings is '" + saved.colorSettings + "' (Custom or unreadable): save the current Color Settings as a named preset first (RUN.md)");
    }
  } catch (eMain) {
    logErr("MAIN", "fatal", eMain);
  } finally {
    // restore every global setting we touched
    try { if (saved.colorSettings && String(app.colorSettings) !== saved.colorSettings) app.colorSettings = saved.colorSettings; } catch (er1) { logErr("RESTORE", "colorSettings", er1); }
    try { manifest.state.colorSettings_after = String(app.colorSettings); } catch (er2) { }
    try { app.preferences.rulerUnits = saved.ruler; } catch (er3) { }
    try { closeWork(); for (var fk in fixtures) { try { fixtures[fk].close(SaveOptions.DONOTSAVECHANGES); } catch (er4) { } } } catch (er5) { }
    try { app.displayDialogs = saved.dialogs; } catch (er6) { }
    manifest.state.restore_checked = (manifest.state.colorSettings_after === saved.colorSettings);
    flushManifest();
  }
  function blendGridG() { blendGridGImpl(); }
  function blendGridGImpl() { for (var i = 0; i < MODES.length; i++) (function (m) { variant("PS-G01", "gamma1_d8_o100_f100_" + m, function () {
      var d = newDoc("bgG", 256, 256, BitsPerChannelType.EIGHT); var bd = pasteFix("F_GRID_HRAMP_256", d, d.artLayers[0], "backdrop");
      var s = pasteFix("F_GRID_VRAMP_256", d, bd, "source"); s.blendMode = BlendMode[m];
      exportDoc(d, "PS-G01", "gamma1_d8_o100_f100_" + m, { mode: m, gamma_preset: GAMMA_PRESET }, false); }); })(MODES[i]); }
})();
