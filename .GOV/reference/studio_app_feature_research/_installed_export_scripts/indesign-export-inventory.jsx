/*
  Handshake Studio Adobe inventory exporter for InDesign.
  Run inside InDesign via Scripts panel or UXP/ExtendScript-compatible runner.
  Output: JSON file on Desktop named handshake-indesign-installed-inventory.json.
*/
(function () {
  function safe(value) {
    try {
      if (value === undefined) return null;
      if (value === null) return null;
      return String(value);
    } catch (e) {
      return "<unreadable>";
    }
  }

  function collectionToRows(collection, sourceSurface) {
    var rows = [];
    try {
      var items = collection.everyItem().getElements();
      for (var i = 0; i < items.length; i++) {
        var item = items[i];
        var row = {
          source_surface: sourceSurface,
          name: safe(item.name),
          id: safe(item.id),
          index: i,
          constructor_name: safe(item.constructor && item.constructor.name),
          enabled: safe(item.enabled),
          visible: safe(item.visible),
          parent_name: safe(item.parent && item.parent.name)
        };
        rows.push(row);
      }
    } catch (e) {
      rows.push({ source_surface: sourceSurface, export_error: safe(e) });
    }
    return rows;
  }

  function reflectObject(obj, label) {
    var rows = [];
    try {
      var props = obj.reflect && obj.reflect.properties ? obj.reflect.properties : [];
      for (var p = 0; p < props.length; p++) {
        rows.push({ source_surface: "scripting_reflect_property", object: label, name: safe(props[p].name), type: safe(props[p].type) });
      }
      var methods = obj.reflect && obj.reflect.methods ? obj.reflect.methods : [];
      for (var m = 0; m < methods.length; m++) {
        rows.push({ source_surface: "scripting_reflect_method", object: label, name: safe(methods[m].name), type: safe(methods[m].type) });
      }
    } catch (e) {
      rows.push({ source_surface: "scripting_reflect", object: label, export_error: safe(e) });
    }
    return rows;
  }

  var inventory = {
    exporter_id: "handshake.adobe.indesign.installed_inventory.v0",
    exported_at: new Date().toISOString(),
    app: "indesign",
    app_name: safe(app.name),
    app_version: safe(app.version),
    locale: safe(app.locale),
    platform: safe($.os),
    rows: []
  };

  inventory.rows = inventory.rows.concat(collectionToRows(app.menuActions, "menu_action"));
  inventory.rows = inventory.rows.concat(collectionToRows(app.scriptMenuActions, "script_menu_action"));
  inventory.rows = inventory.rows.concat(collectionToRows(app.menus, "menu"));
  inventory.rows = inventory.rows.concat(collectionToRows(app.panels, "panel"));
  inventory.rows = inventory.rows.concat(collectionToRows(app.toolBoxTools, "toolbox_tool"));
  inventory.rows = inventory.rows.concat(reflectObject(app, "app"));

  var out = File(Folder.desktop + "/handshake-indesign-installed-inventory.json");
  out.encoding = "UTF-8";
  out.open("w");
  out.write(JSON.stringify(inventory, null, 2));
  out.close();
  alert("Handshake InDesign inventory exported: " + out.fsName);
}());
