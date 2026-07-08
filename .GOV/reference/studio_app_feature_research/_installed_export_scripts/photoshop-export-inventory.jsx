/*
  Handshake Studio Adobe inventory exporter for Photoshop.
  Run inside Photoshop via File > Scripts > Browse.
  Output: JSON file on Desktop named handshake-photoshop-installed-inventory.json.
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

  function descriptorCommandSeed() {
    return [
      "new", "open", "close", "save", "saveAs", "export", "print", "undo", "redo", "cut", "copy", "paste",
      "selectAll", "deselect", "inverse", "feather", "modify", "colorRange", "contentAwareFill",
      "freeTransform", "transform", "warp", "crop", "trim", "imageSize", "canvasSize", "mode",
      "levels", "curves", "brightnessEvent", "hueSaturation", "colorBalance", "blackAndWhite",
      "cameraRawFilter", "gaussianBlur", "unsharpMask", "smartSharpen", "liquify", "lensCorrection",
      "make", "delete", "duplicate", "mergeLayers", "flattenImage", "newPlacedLayer", "convertToSmartObject",
      "rasterizeLayer", "makeClippingMask", "releaseClippingMask", "addLayerMask", "applyLayerMask"
    ];
  }

  var inventory = {
    exporter_id: "handshake.adobe.photoshop.installed_inventory.v0",
    exported_at: new Date().toISOString(),
    app: "photoshop",
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

  var commands = descriptorCommandSeed();
  for (var i = 0; i < commands.length; i++) {
    inventory.rows.push({
      source_surface: "known_action_descriptor_seed",
      name: commands[i],
      note: "Seed list only; complete menu/tool/panel count requires installed shortcut summary, menu summary, toolbar export, and UXP/batchPlay catalog."
    });
  }

  var out = File(Folder.desktop + "/handshake-photoshop-installed-inventory.json");
  out.encoding = "UTF-8";
  out.open("w");
  out.write(JSON.stringify(inventory, null, 2));
  out.close();
  alert("Handshake Photoshop inventory exported: " + out.fsName);
}());
