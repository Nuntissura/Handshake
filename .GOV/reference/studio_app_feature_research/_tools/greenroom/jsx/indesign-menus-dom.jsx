/*
  Handshake Studio green room: InDesign live capture.
  Executed through COM app.DoScript(source, idJavascript) from adobe-com-capture.py.
  Returns a JSON string: menu actions, script menu actions, menu tree, panels, tools,
  presets (document/PDF/print/preflight/flattener), styles, and enum/DOM surface.
*/
(function () {
  function safe(v) { try { return v === undefined || v === null ? null : String(v); } catch (e) { return "<unreadable>"; } }
  // ExtendScript is ES3: Date has no toISOString.
  function isoNow() {
    function p(n, w) { var s = String(n); while (s.length < (w || 2)) { s = "0" + s; } return s; }
    var d = new Date();
    return d.getUTCFullYear() + "-" + p(d.getUTCMonth() + 1) + "-" + p(d.getUTCDate()) + "T" + p(d.getUTCHours()) + ":" + p(d.getUTCMinutes()) + ":" + p(d.getUTCSeconds()) + "Z";
  }
  function collect(coll, fields, limit) {
    var out = [], n = 0;
    try { n = coll.length; } catch (e) { return [{ collection_error: safe(e) }]; }
    var max = limit ? Math.min(n, limit) : n;
    for (var i = 0; i < max; i++) {
      var row = {};
      var item;
      try { item = coll[i]; } catch (e) { out.push({ index: i, error: safe(e) }); continue; }
      for (var f = 0; f < fields.length; f++) {
        var key = fields[f];
        try { row[key] = safe(item[key]); } catch (e) { row[key] = null; }
      }
      out.push(row);
    }
    return out;
  }

  var r = {
    exporter_id: "handshake.adobe.indesign.live_capture.v1",
    exported_at: isoNow(),
    app: "indesign",
    app_name: safe(app.name),
    app_version: safe(app.version),
    full_version: safe(app.fullName),
    locale: safe(app.locale),
    errors: []
  };

  try { r.menu_actions = collect(app.menuActions, ["id", "name", "title", "label", "enabled", "checked", "area", "keyboardShortcut"]); }
  catch (e) { r.errors.push("menuActions: " + safe(e)); }

  try { r.script_menu_actions = collect(app.scriptMenuActions, ["id", "name", "title", "enabled", "checked"]); }
  catch (e) { r.errors.push("scriptMenuActions: " + safe(e)); }

  try {
    var menus = [];
    for (var m = 0; m < app.menus.length; m++) {
      var menu = app.menus[m];
      var entry = { name: safe(menu.name), title: safe(menu.title), items: [] };
      try {
        for (var mi = 0; mi < menu.menuElements.length; mi++) {
          var el = menu.menuElements[mi];
          var it = { name: safe(el.name), index: mi };
          try { it.constructorName = safe(el.constructor.name); } catch (e2) {}
          try { if (el.associatedMenuAction) { it.action_id = safe(el.associatedMenuAction.id); it.action_name = safe(el.associatedMenuAction.name); it.shortcut = safe(el.associatedMenuAction.keyboardShortcut); } } catch (e3) {}
          try {
            if (el.submenu) {
              it.submenu = [];
              for (var si = 0; si < el.submenu.menuElements.length; si++) {
                var sel = el.submenu.menuElements[si];
                var sit = { name: safe(sel.name) };
                try { if (sel.associatedMenuAction) { sit.action_id = safe(sel.associatedMenuAction.id); sit.shortcut = safe(sel.associatedMenuAction.keyboardShortcut); } } catch (e5) {}
                it.submenu.push(sit);
              }
            }
          } catch (e4) {}
          entry.items.push(it);
        }
      } catch (e6) { entry.items_error = safe(e6); }
      menus.push(entry);
    }
    r.menus = menus;
  } catch (e) { r.errors.push("menus: " + safe(e)); }

  try { r.panels = collect(app.panels, ["name", "id", "visible", "index", "associatedMenuAction"]); }
  catch (e) { r.errors.push("panels: " + safe(e)); }

  try { r.tools = collect(app.toolBoxTools ? [app.toolBoxTools] : [], ["currentTool"]); }
  catch (e) { r.errors.push("tools: " + safe(e)); }

  var presetSets = [
    ["document_presets", "documentPresets", ["name", "pageWidth", "pageHeight", "pagesPerDocument", "facingPages", "top", "bottom", "left", "right", "columnCount", "columnGutter", "slugTopOffset", "documentBleedTopOffset"]],
    ["pdf_export_presets", "pdfExportPresets", ["name", "standardsCompliance", "acrobatCompatibility", "colorBitmapCompression", "colorBitmapQuality", "colorBitmapSampling", "cropImagesToFrames", "includeBookmarks", "includeHyperlinks", "exportLayers", "pdfMarkType", "bleedTop"]],
    ["printer_presets", "printerPresets", ["name", "printer", "ppd", "paperSize", "colorOutput", "trapping"]],
    ["preflight_profiles", "preflightProfiles", ["name", "id", "description"]],
    ["flattener_presets", "flattenerPresets", ["name", "rasterVectorBalance", "lineArtAndTextResolution", "gradientAndMeshResolution", "convertAllStrokesToOutlines"]],
    ["transparency_presets", "transparencyPreferences", ["blendingSpace"]],
    ["mojikumi_tables", "mojikumiTables", ["name"]],
    ["kinsoku_tables", "kinsokuTables", ["name"]],
    ["composite_fonts", "compositeFonts", ["name"]],
    ["languages", "languagesWithVendors", ["name", "id", "singleWordSpelling", "hyphenationVendor", "spellingVendor"]],
    ["swatches_app", "swatches", ["name", "model", "space", "colorValue"]],
    ["paragraph_styles_app", "paragraphStyles", ["name", "appliedFont", "pointSize", "leading", "justification"]],
    ["character_styles_app", "characterStyles", ["name", "appliedFont", "pointSize"]],
    ["object_styles_app", "objectStyles", ["name", "appliedParagraphStyle", "enableFill", "enableStroke"]],
    ["table_styles_app", "tableStyles", ["name"]],
    ["cell_styles_app", "cellStyles", ["name"]],
    ["trap_presets", "trapPresets", ["name", "trapWidth", "blackWidth", "trapJoin"]],
    ["conditions", "conditions", ["name", "visible"]],
    ["xml_tags", "xmlTags", ["name"]],
    ["fonts", "fonts", ["name", "fontFamily", "fontStyleName", "postscriptName", "status", "fontType"]]
  ];
  r.presets = {};
  for (var p = 0; p < presetSets.length; p++) {
    var label = presetSets[p][0], prop = presetSets[p][1], fields = presetSets[p][2];
    try {
      var coll = app[prop];
      if (coll === undefined || coll === null) { r.presets[label] = { unavailable: true }; continue; }
      r.presets[label] = collect(coll, fields, 4000);
    } catch (e) { r.presets[label] = { error: safe(e) }; }
  }

  try {
    var prefs = {};
    var prefNames = ["generalPreferences", "textPreferences", "textEditingPreferences", "storyPreferences", "documentPreferences", "viewPreferences", "gridPreferences", "guidePreferences", "marginPreferences", "transparencyPreferences", "exportForWebPreferences", "linkingPreferences", "displayPerformancePreferences", "spellPreferences", "autoCorrectPreferences", "dictionaryPreferences", "footnoteOptions", "indexingSortOptions", "trackChangesPreferences", "epubExportPreferences", "colorSettings", "clipboardPreferences", "galleyPreferences", "smartGuidePreferences", "polygonPreferences", "printPreferences", "pdfPlacePreferences", "imageIOPreferences", "importedPageAttributes", "textDefaults", "pathFinderOptions", "contentPlacerPreferences"];
    for (var q = 0; q < prefNames.length; q++) {
      var pn = prefNames[q];
      try {
        var obj = app[pn];
        if (!obj) { continue; }
        var props = obj.properties;
        var flat = {};
        for (var k in props) {
          if (!props.hasOwnProperty(k)) continue;
          try { flat[k] = safe(props[k]); } catch (e7) { flat[k] = "<err>"; }
        }
        prefs[pn] = flat;
      } catch (e8) { prefs[pn] = { error: safe(e8) }; }
    }
    r.preferences = prefs;
  } catch (e) { r.errors.push("preferences: " + safe(e)); }

  return JSON.stringify(r);
}());
