// hsk_oracle_aftereffects.jsx  -  WP-KERNEL-STUDIO D5 oracle session, After Effects valueAtTime oracle (ExtendScript, ES3).
// Plan: oracle-session/SESSION_PLAN.json (app "aftereffects"). Operator steps: oracle-session/RUN.md.
// No render. Writes ONLY under OUT_ROOT/aftereffects. Global state touched: none beyond a scratch project (closed without saving).
// Needs AE preference "Allow Scripts to Write Files and Access Network" (RUN.md step). Without it the script reports once via alert() and stops.
// API status: Property.valueAtTime(time, preExpression), setValueAtTime, setInterpolationTypeAtKey, setTemporalEaseAtKey, KeyframeEase(speed, influence),
// setSpatialTangentsAtKey, setRovingAtKey, keyInTemporalEase / keyOutTemporalEase, removeKey, app.beginSuppressDialogs / endSuppressDialogs(alert)
// checked against ae-scripting.docsforadobe.dev (fetched) and the Adobe-derived typings (pravdomil/types-for-adobe AfterEffects/26.0).
// UNVERIFIED: speed units for a spatial property; behaviour of exact match names ("ADBE Transform Group", "ADBE Rotate Z", "ADBE Position").
#target aftereffects
(function () {
  // Absolute path is unavoidable (Operator-specified output root). Change only this line to relocate.
  var OUT_ROOT = "D:/Projects/LLM projects/Handshake/Handshake Worktrees/Handshake_Artifacts/WP-KERNEL-STUDIO/oracle-corpus/adobe-session";
  var APP = "aftereffects", OUT = OUT_ROOT + "/" + APP, CTRL = OUT_ROOT + "/control";
  function ensure(p) { var f = new Folder(p); if (!f.exists) { ensure(f.parent.fsName.replace(/\\/g, "/")); f.create(); } }
  function wtxt(p, t) { var f = new File(p); f.encoding = "UTF-8"; if (!f.open("w")) throw new Error("cannot write " + p); f.write(t); f.close(); }
  function atxt(p, t) { var f = new File(p); f.encoding = "UTF-8"; if (!f.open("a")) throw new Error("cannot append " + p); f.write(t); f.close(); }
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

  try { ensure(OUT); ensure(CTRL); wtxt(OUT + "/write_probe.txt", "ok"); }
  catch (eW) { alert("HSK oracle: After Effects may not write files. Turn on Edit > Preferences > Scripting & Expressions > Allow Scripts to Write Files and Access Network, then run again.\n" + eW.message); return; }
  var manifest = { schema: "hsk.oracle.session.manifest@1", app: APP, started_utc: nowIso(), script: "hsk_oracle_aftereffects.jsx", app_version: String(app.version), os: String($.os), entries: [], errors: [], state: {} };
  function logErr(id, v, e) { var m = id + "/" + v + ": " + (e && e.message ? e.message : String(e)) + (e && e.line ? " (line " + e.line + ")" : ""); manifest.errors.push(m); try { atxt(OUT + "/errors.log", m + "\n"); } catch (x) { } }
  function flush() { manifest.finished_utc = nowIso(); wtxt(OUT + "/manifest.json", ser(manifest)); }
  var onlyTxt = rtxt(CTRL + "/ae_only.txt"), ONLY = null;
  if (onlyTxt) { ONLY = {}; var ls = onlyTxt.split(/\r?\n/); for (var q = 0; q < ls.length; q++) if (ls[q].replace(/\s/g, "")) ONLY[ls[q].replace(/\s/g, "")] = true; }
  function want(id) { return !ONLY || ONLY[id] === true; }

  if (app.project && (app.project.numItems > 0 || app.project.file)) { logErr("PRE", "project-open", "A project is already open. Start After Effects with an empty project, then run again."); flush(); return; }

  function round(v) { return Math.round(v * 1e9) / 1e9; }
  function clearKeys(p) { while (p.numKeys > 0) p.removeKey(1); }
  var KT = { LINEAR: KeyframeInterpolationType.LINEAR, BEZIER: KeyframeInterpolationType.BEZIER, HOLD: KeyframeInterpolationType.HOLD };
  var N = 100;   // samples per segment: t0 + dt*k/N, k = 0..N

  function newComp(duration) { return app.project.items.addComp("hsk_oracle", 320, 240, 1, duration, 24); }
  function sample1D(p, t0, dt) { var out = []; for (var k = 0; k <= N; k++) out.push(round(p.valueAtTime(t0 + dt * k / N, true))); return out; }

  // AE-01: dense temporal grid on a 1-D property (Null layer, Rotation, degrees). Two keys, both BEZIER, KeyframeEase(speed, influence).
  function ae01() {
    var comp = newComp(10), nl = comp.layers.addNull(10), rot = nl.property("ADBE Transform Group").property("ADBE Rotate Z");
    var speeds = [0, 100, -100], infl = [16.666666667, 33.333333333, 75], dts = [1, 3], dvs = [90, -90], seg = 0, header = { case_id: "AE-01", n_samples_per_segment: N + 1, sample_times: "t0 + dt*k/" + N, value_unit: "degrees", property: "Null layer Rotation (ADBE Rotate Z)", keys: "k1 at t=0 value 0 (out ease s0,i0); k2 at t=dt value dv (in ease s1,i1)", preExpression: true };
    var f = OUT + "/AE-01__bezier_grid.jsonl"; wtxt(f, ser(header) + "\n");
    for (var a = 0; a < 3; a++) for (var b = 0; b < 3; b++) for (var c = 0; c < 3; c++) for (var d = 0; d < 3; d++) for (var e = 0; e < 2; e++) for (var g = 0; g < 2; g++) {
      var s0 = speeds[a], i0 = infl[b], s1 = speeds[c], i1 = infl[d], dt = dts[e], dv = dvs[g];
      try {
        clearKeys(rot); rot.setValueAtTime(0, 0); rot.setValueAtTime(dt, dv);
        rot.setInterpolationTypeAtKey(1, KT.BEZIER, KT.BEZIER); rot.setInterpolationTypeAtKey(2, KT.BEZIER, KT.BEZIER);
        rot.setTemporalEaseAtKey(1, [new KeyframeEase(s0, i0)], [new KeyframeEase(s0, i0)]);
        rot.setTemporalEaseAtKey(2, [new KeyframeEase(s1, i1)], [new KeyframeEase(s1, i1)]);
        var readOut = rot.keyOutTemporalEase(1)[0], readIn = rot.keyInTemporalEase(2)[0];
        atxt(f, ser({ seg: seg, s0: s0, i0: i0, s1: s1, i1: i1, dt: dt, dv: dv, read_out0: [round(readOut.speed), round(readOut.influence)], read_in1: [round(readIn.speed), round(readIn.influence)], v: sample1D(rot, 0, dt) }) + "\n");
      } catch (ex) { logErr("AE-01", "seg" + seg, ex); }
      seg++;
    }
    manifest.entries.push({ case_id: "AE-01", variant: "bezier_grid", file: "AE-01__bezier_grid.jsonl", status: "ok", segments: seg, format: "JSON lines (header + one object per segment)" });
  }

  // AE-01b: mixed interpolation types (LINEAR / BEZIER / HOLD), temporal continuity and auto-bezier flags, 3-key chains
  function ae01b() {
    var comp = newComp(10), nl = comp.layers.addNull(10), rot = nl.property("ADBE Transform Group").property("ADBE Rotate Z"), out = [];
    var pairs = [["LINEAR", "LINEAR", null, null], ["LINEAR", "BEZIER", null, [100, 33.333333333]], ["BEZIER", "LINEAR", [100, 33.333333333], null], ["HOLD", "LINEAR", null, null], ["HOLD", "BEZIER", null, [50, 50]], ["BEZIER", "BEZIER", [0, 16.666666667], [0, 16.666666667]]];
    for (var i = 0; i < pairs.length; i++) {
      var p = pairs[i];
      try {
        clearKeys(rot); rot.setValueAtTime(0, 0); rot.setValueAtTime(2, 90);
        rot.setInterpolationTypeAtKey(1, KT.LINEAR, KT[p[0]]); rot.setInterpolationTypeAtKey(2, KT[p[1]], KT.LINEAR);
        if (p[2]) rot.setTemporalEaseAtKey(1, [new KeyframeEase(p[2][0], p[2][1])], [new KeyframeEase(p[2][0], p[2][1])]);
        if (p[3]) rot.setTemporalEaseAtKey(2, [new KeyframeEase(p[3][0], p[3][1])], [new KeyframeEase(p[3][0], p[3][1])]);
        out.push({ out_type_k1: p[0], in_type_k2: p[1], ease_k1: p[2], ease_k2: p[3], v: sample1D(rot, 0, 2) });
      } catch (ex) { logErr("AE-01b", p[0] + "_" + p[1], ex); }
    }
    // 3-key chain, middle key temporal-continuous + auto-bezier variants
    var flags = [["plain", false, false], ["auto_bezier", true, false], ["continuous", false, true]];
    for (var j = 0; j < flags.length; j++) {
      try {
        clearKeys(rot); rot.setValueAtTime(0, 0); rot.setValueAtTime(2, 90); rot.setValueAtTime(4, 30);
        for (var k = 1; k <= 3; k++) rot.setInterpolationTypeAtKey(k, KT.BEZIER, KT.BEZIER);
        rot.setTemporalEaseAtKey(2, [new KeyframeEase(20, 40)], [new KeyframeEase(60, 25)]);
        if (flags[j][1]) rot.setTemporalAutoBezierAtKey(2, true);
        if (flags[j][2]) rot.setTemporalContinuousAtKey(2, true);
        out.push({ chain3: flags[j][0], ease_k2_read: [round(rot.keyInTemporalEase(2)[0].speed), round(rot.keyInTemporalEase(2)[0].influence), round(rot.keyOutTemporalEase(2)[0].speed), round(rot.keyOutTemporalEase(2)[0].influence)], v: sample1D(rot, 0, 4) });
      } catch (ex2) { logErr("AE-01b", "chain3_" + flags[j][0], ex2); }
    }
    wtxt(OUT + "/AE-01b__mixed_types.json", ser({ case_id: "AE-01b", n_samples_per_segment: N + 1, rows: out }));
    manifest.entries.push({ case_id: "AE-01b", variant: "mixed_types", file: "AE-01b__mixed_types.json", status: "ok" });
  }

  // AE-02: 2-D spatial property (Position): ease array length, default spatial tangents, spatial tangents set, roving middle key
  function ae02() {
    var comp = newComp(10), nl = comp.layers.addNull(10), pos = nl.property("ADBE Transform Group").property("ADBE Position"), rows = [];
    function samp(t0, dt) { var o = []; for (var k = 0; k <= N; k++) { var v = pos.valueAtTime(t0 + dt * k / N, true); o.push([round(v[0]), round(v[1])]); } return o; }
    try {
      clearKeys(pos); pos.setValueAtTime(0, [40, 200]); pos.setValueAtTime(2, [280, 40]);
      for (var k = 1; k <= 2; k++) pos.setInterpolationTypeAtKey(k, KT.BEZIER, KT.BEZIER);
      pos.setTemporalEaseAtKey(1, [new KeyframeEase(0, 16.666666667)], [new KeyframeEase(0, 16.666666667)]);
      pos.setTemporalEaseAtKey(2, [new KeyframeEase(0, 16.666666667)], [new KeyframeEase(0, 16.666666667)]);
      rows.push({ variant: "two_keys_default_spatial", ease_array_length_out_k1: pos.keyOutTemporalEase(1).length, ease_k1: [round(pos.keyOutTemporalEase(1)[0].speed), round(pos.keyOutTemporalEase(1)[0].influence)], out_spatial_tangent_k1: pos.keyOutSpatialTangent(1), in_spatial_tangent_k2: pos.keyInSpatialTangent(2), v: samp(0, 2) });
    } catch (e1) { logErr("AE-02", "default_spatial", e1); }
    // Position is 3-D valued in AE 26.5 even on a 2-D layer: tangents need one entry per value component
    // (2026-10-16 run: "Value array does not have 3 elements"). tan() pads with zeros to the value length.
    function tan(x, y) { var n = pos.value.length, t = [x, y]; while (t.length < n) t.push(0); return t; }
    try {
      pos.setSpatialTangentsAtKey(1, tan(0, 0), tan(120, 120)); pos.setSpatialTangentsAtKey(2, tan(-120, 60), tan(0, 0));
      rows.push({ variant: "two_keys_spatial_tangents_set", out_spatial_tangent_k1: pos.keyOutSpatialTangent(1), in_spatial_tangent_k2: pos.keyInSpatialTangent(2), v: samp(0, 2) });
    } catch (e2) { logErr("AE-02", "spatial_tangents", e2); }
    try {
      clearKeys(pos); pos.setValueAtTime(0, [40, 200]); pos.setValueAtTime(1, [100, 60]); pos.setValueAtTime(3, [280, 40]);
      for (var m = 1; m <= 3; m++) pos.setInterpolationTypeAtKey(m, KT.LINEAR, KT.LINEAR);
      pos.setSpatialAutoBezierAtKey(2, true); pos.setSpatialContinuousAtKey(2, true);
      pos.setRovingAtKey(2, true);
      rows.push({ variant: "three_keys_roving_middle", roving_k2: pos.keyRoving(2), time_k2_after_roving: pos.keyTime(2), v: samp(0, 3) });
    } catch (e3) { logErr("AE-02", "roving", e3); }
    wtxt(OUT + "/AE-02__spatial.json", ser({ case_id: "AE-02", n_samples_per_segment: N + 1, property: "Null layer Position (ADBE Position)", rows: rows }));
    manifest.entries.push({ case_id: "AE-02", variant: "spatial", file: "AE-02__spatial.json", status: "ok" });
  }

  // AE-03: expression on/off: valueAtTime(t, preExpression true vs false)
  function ae03() {
    var comp = newComp(10), nl = comp.layers.addNull(10), rot = nl.property("ADBE Transform Group").property("ADBE Rotate Z"), rows = [];
    clearKeys(rot); rot.setValueAtTime(0, 0); rot.setValueAtTime(2, 90); rot.setInterpolationTypeAtKey(1, KT.LINEAR, KT.LINEAR); rot.setInterpolationTypeAtKey(2, KT.LINEAR, KT.LINEAR);
    var exprs = ["value*2", "value + time*10", "linear(time, 0, 2, 0, 180)"];
    for (var i = 0; i < exprs.length; i++) {
      try {
        rot.expression = exprs[i]; rot.expressionEnabled = true;
        var pre = [], post = [];
        for (var k = 0; k <= N; k++) { var t = 2 * k / N; pre.push(round(rot.valueAtTime(t, true))); post.push(round(rot.valueAtTime(t, false))); }
        rows.push({ expression: exprs[i], expression_error: String(rot.expressionError), pre_expression_true: pre, pre_expression_false: post });
      } catch (e) { logErr("AE-03", exprs[i], e); }
    }
    wtxt(OUT + "/AE-03__expressions.json", ser({ case_id: "AE-03", n_samples: N + 1, sample_times: "2*k/" + N, rows: rows }));
    manifest.entries.push({ case_id: "AE-03", variant: "expressions", file: "AE-03__expressions.json", status: "ok" });
  }

  var CASES = [["AE-01", ae01], ["AE-01b", ae01b], ["AE-02", ae02], ["AE-03", ae03]];
  try {
    app.beginSuppressDialogs();
    for (var ci = 0; ci < CASES.length; ci++) {
      if (!want(CASES[ci][0])) continue;
      try { manifest.state.current_case = CASES[ci][0]; CASES[ci][1](); } catch (e) { logErr(CASES[ci][0], "case", e); }
      flush();
    }
  } catch (eMain) { logErr("MAIN", "fatal", eMain); }
  finally {
    try { app.endSuppressDialogs(false); } catch (r1) { }
    try { app.project.close(CloseOptions.DO_NOT_SAVE_CHANGES); } catch (r2) { }
    flush();
  }
})();
