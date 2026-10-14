// hsk_oracle_indesign.jsx  -  WP-KERNEL-STUDIO D5 oracle session, InDesign composer probes (ExtendScript, ES3).
// Plan: oracle-session/SESSION_PLAN.json (app "indesign"). Operator steps: oracle-session/RUN.md.
// Writes ONLY under OUT_ROOT/indesign. Global state touched: app.scriptPreferences.userInteractionLevel,
// app.scriptPreferences.measurementUnit (both restored in finally). No cloud, no dialogs.
// API status: names checked against the Adobe-dictionary-derived typings (pravdomil/types-for-adobe, InDesign 2023):
// scriptPreferences.userInteractionLevel, UserInteractionLevels.NEVER_INTERACT, Document.exportFile(format,to,showingOptions,using),
// ExportFormat.PDF_TYPE / INDESIGN_MARKUP (IDML), Paragraph.composer (string), hyphenation, min/desired/max word/letter/glyph spacing,
// Character.horizontalOffset, TextFrame.nextTextFrame, overflows. UNVERIFIED: the exact composer string values
// ("Adobe Paragraph Composer" / "Adobe Single-line Composer" are the UI names) and the PDF preset names; the script records the default string it reads.
#target indesign
(function () {
  // Absolute path is unavoidable (Operator-specified output root). Change only this line to relocate.
  var OUT_ROOT = "D:/Projects/LLM projects/Handshake/Handshake Worktrees/Handshake_Artifacts/WP-KERNEL-STUDIO/oracle-corpus/adobe-session";
  var APP = "indesign", OUT = OUT_ROOT + "/" + APP, CTRL = OUT_ROOT + "/control";
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
  var manifest = { schema: "hsk.oracle.session.manifest@1", app: APP, started_utc: nowIso(), script: "hsk_oracle_indesign.jsx", app_version: String(app.version), locale: String(app.locale), os: String($.os), entries: [], errors: [], state: {} };
  function logErr(id, v, e) { var m = id + "/" + v + ": " + (e && e.message ? e.message : String(e)) + (e && e.line ? " (line " + e.line + ")" : ""); manifest.errors.push(m); atxt(OUT + "/errors.log", m + "\n"); }
  function flush() { manifest.finished_utc = nowIso(); wtxt(OUT + "/manifest.json", ser(manifest)); }
  var onlyTxt = rtxt(CTRL + "/id_only.txt"), ONLY = null;
  if (onlyTxt) { ONLY = {}; var ls = onlyTxt.split(/\r?\n/); for (var q = 0; q < ls.length; q++) if (ls[q].replace(/\s/g, "")) ONLY[ls[q].replace(/\s/g, "")] = true; }
  function want(id) { return !ONLY || ONLY[id] === true; }

  var saved = { uil: app.scriptPreferences.userInteractionLevel, mu: app.scriptPreferences.measurementUnit };
  if (app.documents.length > 0) { logErr("PRE", "open-documents", "Close all InDesign documents, then run again."); flush(); return; }

  // Fixed probe text (author-written for this session; plain ASCII, single paragraph). Studio side uses the same string.
  var TEXT = "Typographic composition decides where every line of a paragraph ends, and a composer that looks at the whole paragraph can trade tighter word spacing against fewer hyphens. " +
    "Extraordinarily long words such as internationalization, counterrevolutionary and incomprehensibilities give the hyphenator real choices, while short words keep the lines honest. " +
    "Measure the result carefully, because small differences in spacing limits move entire line breaks.";
  var FONT_CANDIDATES = ["Arial\tRegular", "Minion Pro\tRegular"];

  function makeDoc(pageW, pageH) {
    var d = app.documents.add(false);   // no window
    d.documentPreferences.pageWidth = pageW; d.documentPreferences.pageHeight = pageH; d.documentPreferences.facingPages = false;
    d.marginPreferences.top = 0; d.marginPreferences.bottom = 0; d.marginPreferences.left = 0; d.marginPreferences.right = 0;
    return d;
  }
  function pickFont() {
    for (var i = 0; i < FONT_CANDIDATES.length; i++) { try { var f = app.fonts.itemByName(FONT_CANDIDATES[i]); var n = f.name; return { name: FONT_CANDIDATES[i], full: String(f.fullName), loc: String(f.location) }; } catch (e) { } }
    return null;
  }
  function frame(doc, w, h, text) {
    var tf = doc.pages[0].textFrames.add();
    tf.geometricBounds = [0, 0, h, w];   // [top, left, bottom, right] in points
    tf.textFramePreferences.insetSpacing = [0, 0, 0, 0]; tf.textFramePreferences.textColumnCount = 1;
    tf.contents = text; return tf;
  }
  function style(par, font, o) {
    par.appliedFont = font.name; par.pointSize = 12; par.leading = 14.4; par.tracking = 0; par.kerningMethod = "Metrics";
    par.justification = o.just; par.hyphenation = o.hyph; par.composer = o.composer;
    par.minimumWordSpacing = o.ws[0]; par.desiredWordSpacing = o.ws[1]; par.maximumWordSpacing = o.ws[2];
    par.minimumLetterSpacing = o.ls[0]; par.desiredLetterSpacing = o.ls[1]; par.maximumLetterSpacing = o.ls[2];
    par.minimumGlyphScaling = o.gs[0]; par.desiredGlyphScaling = o.gs[1]; par.maximumGlyphScaling = o.gs[2];
    par.hyphenateWordsLongerThan = 5; par.hyphenateAfterFirst = 2; par.hyphenateBeforeLast = 2; par.hyphenateLadderLimit = 3;
  }
  function measure(tf) {   // per line: start index (story-relative), text, first/last char x; per char x
    var lines = [], chars = [];
    for (var li = 0; li < tf.lines.length; li++) {
      var ln = tf.lines[li], n = ln.characters.length, first = ln.characters[0], last = ln.characters[n - 1];
      lines.push({ i: li, start: first.index, count: n, x0: Number(first.horizontalOffset), x1: Number(last.horizontalOffset), text: ln.contents });
      for (var ci = 0; ci < n; ci++) chars.push([ln.characters[ci].index, Number(ln.characters[ci].horizontalOffset)]);
    }
    return { lines: lines, chars: chars };
  }
  var FILES_TXT = [];
  function dump(caseId, name, params, tf, doc) {
    var m = measure(tf), fname = caseId + "__" + fileSafe(name) + ".json";
    wtxt(OUT + "/" + fname, ser({ case_id: caseId, variant: name, params: params, text_length: TEXT.length, overflows: tf.overflows, line_count: m.lines.length, lines: m.lines, char_x_pt: m.chars }));
    manifest.entries.push({ case_id: caseId, variant: name, file: fname, status: "ok", format: "JSON (points, story character index, x = Character.horizontalOffset)", params: params });
  }
  function variant(caseId, name, fn) { var doc = null; try { doc = fn(); } catch (e) { logErr(caseId, name, e); manifest.entries.push({ case_id: caseId, variant: name, status: "error" }); } closeAll(); }
  function closeAll() { for (var i = app.documents.length - 1; i >= 0; i--) { try { app.documents[i].close(SaveOptions.NO); } catch (e) { } } }
  var CASES = [];
  function C(id, fn) { CASES.push({ id: id, fn: fn }); }

  // ID-01: width sweep x composer x hyphenation x justification (80 variants)
  C("ID-01", function () {
    var font = pickFont(); if (!font) throw new Error("no usable font (Arial / Minion Pro)"); manifest.state.font = font;
    var widths = [150, 180, 210, 240, 270, 300, 330, 360, 390, 420];
    var composers = [["para", "Adobe Paragraph Composer"], ["single", "Adobe Single-line Composer"]], hy = [["hyph", true], ["nohyph", false]], js = [["leftjust", Justification.LEFT_JUSTIFIED], ["left", Justification.LEFT_ALIGN]];
    for (var a = 0; a < composers.length; a++) for (var b = 0; b < 2; b++) for (var c = 0; c < 2; c++) for (var w = 0; w < widths.length; w++) (function (cp, h, j, wd) {
      var nm = cp[0] + "_" + h[0] + "_" + j[0] + "_w" + wd;
      variant("ID-01", nm, function () {
        var d = makeDoc(500, 800); var tf = frame(d, wd, 700, TEXT); var par = tf.parentStory.paragraphs[0];
        if (a === 0 && b === 0 && c === 0 && w === 0) manifest.state.default_composer_string_before_set = String(par.composer);
        style(par, font, { just: j[1], hyph: h[1], composer: cp[1], ws: [80, 100, 133], ls: [0, 0, 0], gs: [100, 100, 100] });
        dump("ID-01", nm, { composer: cp[1], hyphenation: h[1], justification: j[0], frame_width_pt: wd, font: font.name, size_pt: 12, leading_pt: 14.4, word_spacing: [80, 100, 133], letter_spacing: [0, 0, 0], glyph_scaling: [100, 100, 100], hyphenate_words_longer_than: 5, after_first: 2, before_last: 2, ladder_limit: 3 }, tf, d);
        return d;
      });
    })(composers[a], hy[b], js[c], widths[w]);
  });

  // ID-02: spacing-limit sweep at width 240, both composers, hyphenation on, fully justified
  C("ID-02", function () {
    var font = pickFont(); if (!font) throw new Error("no usable font");
    var sets = [["ws100_100_100", [100, 100, 100], [0, 0, 0], [100, 100, 100]], ["ws50_100_200", [50, 100, 200], [0, 0, 0], [100, 100, 100]], ["ws80_100_133_ls-3_0_3", [80, 100, 133], [-3, 0, 3], [100, 100, 100]],
      ["ws80_100_133_gs95_100_105", [80, 100, 133], [0, 0, 0], [95, 100, 105]], ["ws0_100_300", [0, 100, 300], [0, 0, 0], [100, 100, 100]], ["ws100_100_100_ls-5_0_5_gs90_100_110", [100, 100, 100], [-5, 0, 5], [90, 100, 110]]];
    var comps = [["para", "Adobe Paragraph Composer"], ["single", "Adobe Single-line Composer"]];
    for (var i = 0; i < sets.length; i++) for (var k = 0; k < 2; k++) (function (s, cp) {
      var nm = cp[0] + "_" + s[0];
      variant("ID-02", nm, function () {
        var d = makeDoc(500, 800); var tf = frame(d, 240, 700, TEXT); var par = tf.parentStory.paragraphs[0];
        style(par, font, { just: Justification.FULLY_JUSTIFIED, hyph: true, composer: cp[1], ws: s[1], ls: s[2], gs: s[3] });
        dump("ID-02", nm, { composer: cp[1], word_spacing: s[1], letter_spacing: s[2], glyph_scaling: s[3], frame_width_pt: 240, justification: "FULLY_JUSTIFIED" }, tf, d);
        return d;
      });
    })(sets[i], comps[k]);
  });

  // ID-03: PDF and IDML exports for three configurations (parsed on the Studio side); PDF preset names UNVERIFIED
  C("ID-03", function () {
    var font = pickFont(); if (!font) throw new Error("no usable font");
    var cfgs = [["para_hyph_w240", "Adobe Paragraph Composer", true], ["single_hyph_w240", "Adobe Single-line Composer", true], ["para_nohyph_w240", "Adobe Paragraph Composer", false]];
    for (var i = 0; i < cfgs.length; i++) (function (c) {
      variant("ID-03", c[0], function () {
        var d = makeDoc(500, 800); var tf = frame(d, 240, 700, TEXT); var par = tf.parentStory.paragraphs[0];
        style(par, font, { just: Justification.FULLY_JUSTIFIED, hyph: c[2], composer: c[1], ws: [80, 100, 133], ls: [0, 0, 0], gs: [100, 100, 100] });
        dump("ID-03", c[0], { composer: c[1], hyphenation: c[2], frame_width_pt: 240 }, tf, d);
        var idml = new File(OUT + "/ID-03__" + c[0] + ".idml"); d.exportFile(ExportFormat.INDESIGN_MARKUP, idml, false);
        manifest.entries.push({ case_id: "ID-03", variant: c[0], file: "ID-03__" + c[0] + ".idml", status: "ok", format: "IDML" });
        var preset = null; try { preset = app.pdfExportPresets.itemByName("[High Quality Print]"); preset.name; } catch (e1) { try { preset = app.pdfExportPresets[0]; } catch (e2) { preset = null; } }
        var pdf = new File(OUT + "/ID-03__" + c[0] + ".pdf");
        if (preset) { d.exportFile(ExportFormat.PDF_TYPE, pdf, false, preset); manifest.entries.push({ case_id: "ID-03", variant: c[0], file: "ID-03__" + c[0] + ".pdf", status: "ok", format: "PDF (preset " + String(preset.name) + ")" }); }
        else logErr("ID-03", c[0], "no PDF export preset available");
        return d;
      });
    })(cfgs[i]);
  });

  // ID-04: threaded frames and overset detection (frame 1 holds part of the text, frame 2 the rest, frame 3 absent -> overset)
  C("ID-04", function () {
    var font = pickFont(); if (!font) throw new Error("no usable font");
    variant("ID-04", "two_linked_small_frames", function () {
      var d = makeDoc(500, 800); var f1 = frame(d, 200, 120, TEXT); var par = f1.parentStory.paragraphs[0];
      style(par, font, { just: Justification.LEFT_JUSTIFIED, hyph: true, composer: "Adobe Paragraph Composer", ws: [80, 100, 133], ls: [0, 0, 0], gs: [100, 100, 100] });
      var f2 = d.pages[0].textFrames.add(); f2.geometricBounds = [150, 0, 270, 200]; f2.textFramePreferences.insetSpacing = [0, 0, 0, 0]; f1.nextTextFrame = f2;
      var m1 = measure(f1), m2 = measure(f2);
      wtxt(OUT + "/ID-04__two_linked_small_frames.json", ser({ frame1: { overflows: f1.overflows, lines: m1.lines }, frame2: { overflows: f2.overflows, lines: m2.lines }, story_length: f1.parentStory.characters.length, text_length: TEXT.length }));
      manifest.entries.push({ case_id: "ID-04", variant: "two_linked_small_frames", file: "ID-04__two_linked_small_frames.json", status: "ok", params: { frame1_pt: [200, 120], frame2_pt: [200, 120] } });
      return d;
    });
  });

  try {
    app.scriptPreferences.userInteractionLevel = UserInteractionLevels.NEVER_INTERACT;
    app.scriptPreferences.measurementUnit = MeasurementUnits.POINTS;
    for (var ci = 0; ci < CASES.length; ci++) {
      if (!want(CASES[ci].id)) continue;
      try { manifest.state.current_case = CASES[ci].id; CASES[ci].fn(); } catch (e) { logErr(CASES[ci].id, "case", e); }
      flush();
    }
  } catch (eMain) { logErr("MAIN", "fatal", eMain); }
  finally {
    closeAll();
    try { app.scriptPreferences.measurementUnit = saved.mu; } catch (r1) { }
    try { app.scriptPreferences.userInteractionLevel = saved.uil; } catch (r2) { }
    manifest.state.restored = (String(app.scriptPreferences.userInteractionLevel) === String(saved.uil));
    flush();
  }
})();
