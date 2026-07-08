/*
  Handshake Studio Adobe inventory exporter for Illustrator.
  Run inside Illustrator via File > Scripts > Other Script.
  Output: JSON file on Desktop named handshake-illustrator-installed-inventory.json.
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

  function staticKnownMenuCommands() {
    return [
      "Adobe Flash Builder", "Adobe Bridge Browse", "Adobe Bridge", "Adobe Photoshop", "Adobe Device Central",
      "new", "open", "close", "save", "saveas", "saveacopy", "revert", "print", "exit",
      "undo", "redo", "cut", "copy", "paste", "pasteInFront", "pasteInBack", "clear",
      "selectall", "deselectall", "Find Fill Color menu item", "Find Stroke Color menu item",
      "group", "ungroup", "lock", "unlockAll", "hide", "showAll", "makeMask", "releaseMask",
      "makeCompoundPath", "releaseCompoundPath", "Live Pathfinder Add", "Live Pathfinder Subtract",
      "Live Pathfinder Intersect", "Live Pathfinder Exclude", "expandStyle", "expand3", "OffsetPath v22",
      "outline", "Rasterize 8 menu item", "Live Trace Make", "Live Trace Expand", "Make Planet X",
      "makeguide", "releaseguide", "clearguide", "average", "join", "cleanup menu item"
    ];
  }

  var inventory = {
    exporter_id: "handshake.adobe.illustrator.installed_inventory.v0",
    exported_at: new Date().toISOString(),
    app: "illustrator",
    app_name: safe(app.name),
    app_version: safe(app.version),
    locale: safe(app.locale),
    platform: safe($.os),
    rows: []
  };

  inventory.rows = inventory.rows.concat(reflectObject(app, "app"));
  if (app.documents.length > 0) {
    inventory.rows = inventory.rows.concat(reflectObject(app.activeDocument, "activeDocument"));
  }

  var commands = staticKnownMenuCommands();
  for (var i = 0; i < commands.length; i++) {
    inventory.rows.push({
      source_surface: "known_execute_menu_command_seed",
      name: commands[i],
      note: "Seed list only; complete menu/tool/panel count still requires installed shortcut/menu/toolbar exports."
    });
  }

  var out = File(Folder.desktop + "/handshake-illustrator-installed-inventory.json");
  out.encoding = "UTF-8";
  out.open("w");
  out.write(JSON.stringify(inventory, null, 2));
  out.close();
  alert("Handshake Illustrator inventory exported: " + out.fsName);
}());
