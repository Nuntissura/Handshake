/*
  Handshake Studio green room: Photoshop installed capture.
  Executed through COM app.DoJavaScript from adobe-com-capture.py.
  Returns a JSON string: app info, full menuBarInfo tree (command IDs, shortcuts,
  enabled state), preset manager inventories, installed fonts.
*/
(function () {
  var s2t = stringIDToTypeID, t2s = typeIDToStringID;

  function safe(v) { try { return v === undefined || v === null ? null : String(v); } catch (e) { return "<unreadable>"; } }

  function descToJson(desc, depth) {
    if (depth > 12) return "<depth>";
    var out = {};
    for (var i = 0; i < desc.count; i++) {
      var key = desc.getKey(i);
      var name = t2s(key) || String(key);
      var type = desc.getType(key);
      try {
        switch (type) {
          case DescValueType.BOOLEANTYPE: out[name] = desc.getBoolean(key); break;
          case DescValueType.STRINGTYPE: out[name] = desc.getString(key); break;
          case DescValueType.INTEGERTYPE: out[name] = desc.getInteger(key); break;
          case DescValueType.LARGEINTEGERTYPE: out[name] = desc.getLargeInteger(key); break;
          case DescValueType.DOUBLETYPE: out[name] = desc.getDouble(key); break;
          case DescValueType.UNITDOUBLE: out[name] = { unit: t2s(desc.getUnitDoubleType(key)), value: desc.getUnitDoubleValue(key) }; break;
          case DescValueType.ENUMERATEDTYPE: out[name] = { enumType: t2s(desc.getEnumerationType(key)), value: t2s(desc.getEnumerationValue(key)) }; break;
          case DescValueType.OBJECTTYPE: out[name] = descToJson(desc.getObjectValue(key), depth + 1); out[name]._class = t2s(desc.getObjectType(key)); break;
          case DescValueType.LISTTYPE: out[name] = listToJson(desc.getList(key), depth + 1); break;
          case DescValueType.REFERENCETYPE: out[name] = "<reference>"; break;
          case DescValueType.CLASSTYPE: out[name] = { classType: t2s(desc.getClass(key)) }; break;
          case DescValueType.ALIASTYPE: out[name] = safe(desc.getPath(key)); break;
          case DescValueType.RAWTYPE: out[name] = "<raw>"; break;
          default: out[name] = "<unknown:" + type + ">";
        }
      } catch (e) { out[name] = "<error:" + safe(e) + ">"; }
    }
    return out;
  }

  function listToJson(list, depth) {
    var arr = [];
    for (var i = 0; i < list.count; i++) {
      var type = list.getType(i);
      try {
        switch (type) {
          case DescValueType.OBJECTTYPE: var o = descToJson(list.getObjectValue(i), depth + 1); o._class = t2s(list.getObjectType(i)); arr.push(o); break;
          case DescValueType.STRINGTYPE: arr.push(list.getString(i)); break;
          case DescValueType.INTEGERTYPE: arr.push(list.getInteger(i)); break;
          case DescValueType.DOUBLETYPE: arr.push(list.getDouble(i)); break;
          case DescValueType.BOOLEANTYPE: arr.push(list.getBoolean(i)); break;
          case DescValueType.ENUMERATEDTYPE: arr.push({ enumType: t2s(list.getEnumerationType(i)), value: t2s(list.getEnumerationValue(i)) }); break;
          case DescValueType.LISTTYPE: arr.push(listToJson(list.getList(i), depth + 1)); break;
          default: arr.push("<type:" + type + ">");
        }
      } catch (e) { arr.push("<error:" + safe(e) + ">"); }
    }
    return arr;
  }

  function appProperty(prop) {
    var ref = new ActionReference();
    ref.putProperty(s2t("property"), s2t(prop));
    ref.putEnumerated(s2t("application"), s2t("ordinal"), s2t("targetEnum"));
    return executeActionGet(ref);
  }

  var result = {
    exporter_id: "handshake.adobe.photoshop.installed_capture.v1",
    exported_at: new Date().toISOString(),
    app: "photoshop",
    app_name: safe(app.name),
    app_version: safe(app.version),
    build: safe(app.build),
    locale: safe(app.locale),
    platform: safe($.os),
    errors: []
  };

  try { result.menu_bar_info = descToJson(appProperty("menuBarInfo"), 0); } catch (e) { result.errors.push("menuBarInfo: " + safe(e)); }

  try {
    var pm = descToJson(appProperty("presetManager"), 0);
    result.preset_manager = pm;
  } catch (e) { result.errors.push("presetManager: " + safe(e)); }

  try {
    var props = ["toolsPreferences", "tool", "interfacePrefs", "generalPreferences", "unitsPrefs", "transparencyPrefs", "guidesPrefs", "pluginPrefs", "typePrefs", "colorSettings", "workspaceList", "currentToolOptions"];
    result.app_properties = {};
    for (var p = 0; p < props.length; p++) {
      try { result.app_properties[props[p]] = descToJson(appProperty(props[p]), 0); }
      catch (e) { result.app_properties[props[p]] = "<error:" + safe(e) + ">"; }
    }
  } catch (e) { result.errors.push("app_properties: " + safe(e)); }

  try {
    var fonts = [];
    for (var f = 0; f < app.fonts.length; f++) {
      var fo = app.fonts[f];
      fonts.push({ name: safe(fo.name), postScriptName: safe(fo.postScriptName), family: safe(fo.family), style: safe(fo.style) });
    }
    result.fonts = fonts;
  } catch (e) { result.errors.push("fonts: " + safe(e)); }

  return JSON.stringify(result);
}());
