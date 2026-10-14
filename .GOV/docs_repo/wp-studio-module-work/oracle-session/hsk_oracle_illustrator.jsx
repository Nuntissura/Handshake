// hsk_oracle_illustrator.jsx  -  WP-KERNEL-STUDIO D5 oracle session, Illustrator batch (ExtendScript, ES3).
// Plan: oracle-session/SESSION_PLAN.json (app "illustrator"). Operator steps: oracle-session/RUN.md.
// Writes ONLY under OUT_ROOT/illustrator. Global state touched: app.userInteractionLevel (restored in finally).
// API status: every class/property/enum used here was checked against the installed dictionary
// C:/Program Files/Common Files/Adobe/Scripting Dictionaries CC/Illustrator 2026/omv.xml (read-only).
// Not checked against a running app: exact pixel output (that is what this session measures).
#target illustrator
(function () {
  // Absolute path is unavoidable (Operator-specified output root). Change only this line to relocate.
  var OUT_ROOT = "D:/Projects/LLM projects/Handshake/Handshake Worktrees/Handshake_Artifacts/WP-KERNEL-STUDIO/oracle-corpus/adobe-session";
  var APP = "illustrator", OUT = OUT_ROOT + "/" + APP, CTRL = OUT_ROOT + "/control";
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
  function fileSafe(s) { return String(s).replace(/[^A-Za-z0-9_.+-]/g, "_"); }

  ensure(OUT); ensure(CTRL);
  var manifest = { schema: "hsk.oracle.session.manifest@1", app: APP, started_utc: nowIso(), script: "hsk_oracle_illustrator.jsx", app_version: String(app.version), locale: String(app.locale), os: String($.os), entries: [], errors: [], state: {} };
  function logErr(id, v, e) { var m = id + "/" + v + ": " + (e && e.message ? e.message : String(e)) + (e && e.line ? " (line " + e.line + ")" : ""); manifest.errors.push(m); atxt(OUT + "/errors.log", m + "\n"); }
  function flush() { manifest.finished_utc = nowIso(); wtxt(OUT + "/manifest.json", ser(manifest)); }
  var onlyTxt = rtxt(CTRL + "/ai_only.txt"), ONLY = null;
  if (onlyTxt) { ONLY = {}; var ls = onlyTxt.split(/\r?\n/); for (var q = 0; q < ls.length; q++) if (ls[q].replace(/\s/g, "")) ONLY[ls[q].replace(/\s/g, "")] = true; }
  function want(id) { return !ONLY || ONLY[id] === true; }

  var savedUIL = app.userInteractionLevel, savedCS = app.coordinateSystem;
  if (app.documents.length > 0) { logErr("PRE", "open-documents", "Close all Illustrator documents, then run again."); flush(); return; }

  // Page space used by this script: origin at the artboard bottom-left, y up, in points. Mapped to document space here.
  var OX = 0, OY = 0;
  function RC(coll, top, left, w, h) { return coll.rectangle(OY + top, OX + left, w, h); }
  function PT(a) { var o = []; for (var i = 0; i < a.length; i++) o.push([OX + a[i][0], OY + a[i][1]]); return o; }
  function PO(p) { return [OX + p[0], OY + p[1]]; }
  function rgb(r, g, b) { var c = new RGBColor(); c.red = r; c.green = g; c.blue = b; return c; }
  function newDoc(w, h) { var d = app.documents.add(DocumentColorSpace.RGB, w, h); d.rulerUnits = RulerUnits.Points; var ab = d.artboards[0].artboardRect; OX = ab[0]; OY = Math.min(ab[1], ab[3]); manifest.state.artboardRect_last = ab; return d; }
  // y-up page coordinates: pageRect(top, left, w, h) -> rectangle uses (top, left, width, height) with top measured from artboard bottom in y-up space
  function exportTiff(doc, caseId, variant, params, aa) {
    var fname = caseId + "__" + fileSafe(variant) + ".tif", f = new File(OUT + "/" + fname);
    var o = new ExportOptionsTIFF(); o.imageColorSpace = ImageColorSpace.RGB; o.resolution = 72; o.antiAliasing = aa === false ? AntiAliasingMethod.None : AntiAliasingMethod.ARTOPTIMIZED;
    o.lZWCompression = false; o.byteOrder = TIFFByteOrder.IBMPC; o.embedICCProfile = true; o.saveMultipleArtboards = false;
    doc.exportFile(f, ExportType.TIFF, o);
    manifest.entries.push({ case_id: caseId, variant: variant, file: fname, status: "ok", format: "TIFF 8-bit RGB, 72 dpi, no LZW, ICC embedded", antialias: aa === false ? "None" : "ARTOPTIMIZED", doc_profile: String(doc.colorProfileName), w_pt: doc.width, h_pt: doc.height, params: params || {} });
  }
  function variant(caseId, name, fn) {
    var doc = null;
    try { doc = fn(); } catch (e) { logErr(caseId, name, e); manifest.entries.push({ case_id: caseId, variant: name, status: "error" }); }
    try { if (app.documents.length > 0) app.activeDocument.close(SaveOptions.DONOTSAVECHANGES); } catch (e2) { }
  }
  var CASES = [];
  function C(id, fn) { CASES.push({ id: id, fn: fn }); }

  // AI-01  stroke cap x join x miter limit on an open zig-zag and a closed triangle (84 px wide stroke 14 pt)
  C("AI-01", function () {
    var caps = [["butt", StrokeCap.BUTTENDCAP], ["round", StrokeCap.ROUNDENDCAP], ["projecting", StrokeCap.PROJECTINGENDCAP]];
    var joins = [["miter", StrokeJoin.MITERENDJOIN], ["round", StrokeJoin.ROUNDENDJOIN], ["bevel", StrokeJoin.BEVELENDJOIN]];
    var miters = [1, 4, 10];
    for (var a = 0; a < 3; a++) for (var b = 0; b < 3; b++) for (var m = 0; m < 3; m++) (function (cp, jn, ml) {
      var nm = "cap_" + cp[0] + "_join_" + jn[0] + "_miter" + ml;
      variant("AI-01", nm, function () {
        var d = newDoc(256, 128); d.defaultFilled = false;
        var bg = RC(d.pathItems, 128, 0, 256, 128); bg.filled = true; bg.fillColor = rgb(255, 255, 255); bg.stroked = false;
        var zig = d.pathItems.add(); zig.setEntirePath(PT([[20, 30], [70, 100], [120, 30], [170, 100]])); zig.closed = false; zig.filled = false; zig.stroked = true; zig.strokeWidth = 14; zig.strokeColor = rgb(0, 0, 0);
        zig.strokeCap = cp[1]; zig.strokeJoin = jn[1]; zig.strokeMiterLimit = ml;
        var tri = d.pathItems.add(); tri.setEntirePath(PT([[190, 20], [240, 20], [215, 110]])); tri.closed = true; tri.filled = false; tri.stroked = true; tri.strokeWidth = 14; tri.strokeColor = rgb(0, 0, 0);
        tri.strokeCap = cp[1]; tri.strokeJoin = jn[1]; tri.strokeMiterLimit = ml;
        exportTiff(d, "AI-01", nm, { cap: cp[0], join: jn[0], miter_limit: ml, stroke_pt: 14 }, true);
        if (a === 0 && b === 0 && m === 0) exportTiff(d, "AI-01", nm + "_aa_none", { antialias: "none" }, false);
      });
    })(caps[a], joins[b], miters[m]);
  });

  // AI-02  dash arrays and offsets; even-odd vs non-zero on a pentagram
  C("AI-02", function () {
    var dashes = [["d12_6", [12, 6], 0], ["d12_6_off5", [12, 6], 5], ["d6_6_6_12", [6, 6, 6, 12], 0], ["d0_10_round", [0, 10], 0]];
    for (var i = 0; i < dashes.length; i++) (function (v) {
      variant("AI-02", "dash_" + v[0], function () {
        var d = newDoc(256, 64);
        var bg = RC(d.pathItems, 64, 0, 256, 64); bg.fillColor = rgb(255, 255, 255); bg.stroked = false;
        var ln = d.pathItems.add(); ln.setEntirePath(PT([[10, 32], [246, 32]])); ln.filled = false; ln.stroked = true; ln.strokeWidth = 8; ln.strokeColor = rgb(0, 0, 0);
        ln.strokeDashes = v[1]; ln.strokeDashOffset = v[2]; if (v[0].indexOf("round") >= 0) ln.strokeCap = StrokeCap.ROUNDENDCAP;
        exportTiff(d, "AI-02", "dash_" + v[0], { dashes: v[1], offset: v[2], stroke_pt: 8 }, true);
      });
    })(dashes[i]);
    var rules = [["evenodd", true], ["nonzero", false]];
    for (var j = 0; j < 2; j++) (function (r) {
      variant("AI-02", "fill_" + r[0], function () {
        var d = newDoc(128, 128);
        var bg = RC(d.pathItems, 128, 0, 128, 128); bg.fillColor = rgb(255, 255, 255); bg.stroked = false;
        var pts = [], k; for (k = 0; k < 5; k++) { var ang = -Math.PI / 2 + k * 4 * Math.PI / 5; pts.push([64 + 56 * Math.cos(ang), 64 - 56 * Math.sin(ang)]); }
        var star = d.pathItems.add(); star.setEntirePath(PT(pts)); star.closed = true; star.stroked = false; star.filled = true; star.fillColor = rgb(0, 0, 0); star.evenodd = r[1];
        exportTiff(d, "AI-02", "fill_" + r[0], { evenodd: r[1], shape: "pentagram, vertex k at angle -90deg + k*144deg, radius 56, centre 64,64" }, true);
      });
    })(rules[j]);
  });

  // AI-03  gradient midpoint 25/50/75, opacity stops, linear black -> white across 256 pt
  C("AI-03", function () {
    var mids = [25, 50, 75];
    for (var i = 0; i < 3; i++) (function (mp) {
      variant("AI-03", "mid" + mp, function () {
        var d = newDoc(256, 64); var g = d.gradients.add(); g.type = GradientType.LINEAR;
        g.gradientStops[0].rampPoint = 0; g.gradientStops[0].color = rgb(0, 0, 0); g.gradientStops[0].midPoint = mp;
        g.gradientStops[1].rampPoint = 100; g.gradientStops[1].color = rgb(255, 255, 255);
        var r = RC(d.pathItems, 64, 0, 256, 64); r.stroked = false; var gc = new GradientColor(); gc.gradient = g; gc.origin = PO([0, 32]); gc.angle = 0; gc.length = 256; r.fillColor = gc;
        exportTiff(d, "AI-03", "mid" + mp, { midpoint_percent: mp }, true);
      });
    })(mids[i]);
    variant("AI-03", "stop_opacity_50_over_red", function () {
      var d = newDoc(256, 64); var bg = RC(d.pathItems, 64, 0, 256, 64); bg.stroked = false; bg.fillColor = rgb(255, 0, 0);
      var g = d.gradients.add(); g.type = GradientType.LINEAR; g.gradientStops[0].rampPoint = 0; g.gradientStops[0].color = rgb(0, 0, 255); g.gradientStops[0].opacity = 100;
      g.gradientStops[1].rampPoint = 100; g.gradientStops[1].color = rgb(0, 0, 255); g.gradientStops[1].opacity = 0;
      var r = RC(d.pathItems, 64, 0, 256, 64); r.stroked = false; var gc = new GradientColor(); gc.gradient = g; gc.origin = PO([0, 32]); gc.angle = 0; gc.length = 256; r.fillColor = gc;
      exportTiff(d, "AI-03", "stop_opacity_50_over_red", { note: "blue stop opacity 100 -> 0 over solid red" }, true);
    });
    variant("AI-03", "radial_black_white", function () {
      var d = newDoc(128, 128); var g = d.gradients.add(); g.type = GradientType.RADIAL; g.gradientStops[0].rampPoint = 0; g.gradientStops[0].color = rgb(0, 0, 0); g.gradientStops[1].rampPoint = 100; g.gradientStops[1].color = rgb(255, 255, 255);
      var r = RC(d.pathItems, 128, 0, 128, 128); r.stroked = false; var gc = new GradientColor(); gc.gradient = g; gc.origin = PO([64, 64]); gc.length = 64; r.fillColor = gc;
      exportTiff(d, "AI-03", "radial_black_white", { radius_pt: 64 }, true);
    });
  });

  // AI-04  16 blend modes, backdrop = horizontal black->white gradient, source = vertical-ish solid bands, opacity 100 and 50
  C("AI-04", function () {
    var modes = ["NORMAL", "MULTIPLY", "SCREEN", "OVERLAY", "SOFTLIGHT", "HARDLIGHT", "COLORDODGE", "COLORBURN", "DARKEN", "LIGHTEN", "DIFFERENCE", "EXCLUSION", "HUE", "SATURATIONBLEND", "COLORBLEND", "LUMINOSITY"];
    var ops = [100, 50];
    for (var o = 0; o < 2; o++) for (var i = 0; i < modes.length; i++) (function (m, op) {
      var nm = m + "_o" + op;
      variant("AI-04", nm, function () {
        var d = newDoc(256, 64); var g = d.gradients.add(); g.type = GradientType.LINEAR; g.gradientStops[0].rampPoint = 0; g.gradientStops[0].color = rgb(0, 0, 0); g.gradientStops[1].rampPoint = 100; g.gradientStops[1].color = rgb(255, 255, 255);
        var bd = RC(d.pathItems, 64, 0, 256, 64); bd.stroked = false; var gc = new GradientColor(); gc.gradient = g; gc.origin = PO([0, 32]); gc.angle = 0; gc.length = 256; bd.fillColor = gc;
        var cols = [[200, 80, 40], [40, 160, 220], [128, 128, 128], [20, 200, 90]];
        for (var k = 0; k < 4; k++) { var s = RC(d.pathItems, 64 - 16 * k, 0, 256, 16); s.stroked = false; s.fillColor = rgb(cols[k][0], cols[k][1], cols[k][2]); s.blendingMode = BlendModes[m]; s.opacity = op; }
        exportTiff(d, "AI-04", nm, { mode: m, opacity: op, source_bands_rgb: cols }, true);
      });
    })(modes[i], ops[o]);
  });

  // AI-05  group isolate-blending and knockout: two overlapping 50 % Multiply squares inside a group
  C("AI-05", function () {
    var vs = [["iso_off_ko_disabled", false, KnockoutState.DISABLED], ["iso_on_ko_disabled", true, KnockoutState.DISABLED], ["iso_off_ko_enabled", false, KnockoutState.ENABLED], ["iso_on_ko_enabled", true, KnockoutState.ENABLED]];
    for (var i = 0; i < vs.length; i++) (function (v) {
      variant("AI-05", v[0], function () {
        var d = newDoc(128, 128); var bd = RC(d.pathItems, 128, 0, 128, 128); bd.stroked = false; bd.fillColor = rgb(230, 230, 60);
        var g = d.groupItems.add(); g.isIsolated = v[1]; g.artworkKnockout = v[2]; g.opacity = 100;
        var a = RC(g.pathItems, 100, 20, 60, 60); a.stroked = false; a.fillColor = rgb(220, 30, 30); a.opacity = 50; a.blendingMode = BlendModes.MULTIPLY;
        var b = RC(g.pathItems, 80, 50, 60, 60); b.stroked = false; b.fillColor = rgb(30, 30, 220); b.opacity = 50; b.blendingMode = BlendModes.MULTIPLY;
        exportTiff(d, "AI-05", v[0], { isolate_blending: v[1], knockout: String(v[2]) }, true);
      });
    })(vs[i]);
  });

  try {
    app.userInteractionLevel = UserInteractionLevel.DONTDISPLAYALERTS;
    app.coordinateSystem = CoordinateSystem.DOCUMENTCOORDINATESYSTEM;
    for (var ci = 0; ci < CASES.length; ci++) {
      if (!want(CASES[ci].id)) continue;
      try { manifest.state.current_case = CASES[ci].id; CASES[ci].fn(); } catch (e) { logErr(CASES[ci].id, "case", e); }
      flush();
    }
  } catch (eMain) { logErr("MAIN", "fatal", eMain); }
  finally {
    try { app.userInteractionLevel = savedUIL; } catch (r1) { }
    try { app.coordinateSystem = savedCS; } catch (r0) { }
    manifest.state.userInteractionLevel_restored = (String(app.userInteractionLevel) === String(savedUIL));
    flush();
  }
})();
