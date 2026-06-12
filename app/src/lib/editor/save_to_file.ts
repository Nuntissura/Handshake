// WP-KERNEL-009 / MT-244 — quiet file download for save-to-format exports.
//
// Saves an export projection as a file through the standard browser download
// path (Blob → object URL → synthetic <a download> click). Deliberately QUIET:
// no foreground window, no focus steal, no dialog — the file lands in the
// browser/webview download flow (HBR-QUIET / GLOBAL-BUILD-QUIET). The anchor
// is detached and the object URL revoked after the click so repeated exports
// do not leak blobs.
//
// Injectable document/URL for jsdom tests (jsdom has no real createObjectURL).

export interface SaveToFileDeps {
  documentRef?: Document;
  createObjectURL?: (blob: Blob) => string;
  revokeObjectURL?: (url: string) => void;
}

/** Triggers a download of `content` as `filename`. Returns the byte length. */
export function saveTextToFile(
  filename: string,
  content: string,
  mimeType: string,
  deps: SaveToFileDeps = {},
): number {
  const doc = deps.documentRef ?? document;
  const createUrl = deps.createObjectURL ?? ((blob: Blob) => URL.createObjectURL(blob));
  const revokeUrl = deps.revokeObjectURL ?? ((url: string) => URL.revokeObjectURL(url));

  const blob = new Blob([content], { type: mimeType });
  const url = createUrl(blob);
  const anchor = doc.createElement("a");
  anchor.href = url;
  anchor.download = filename;
  anchor.rel = "noopener";
  anchor.style.display = "none";
  doc.body.appendChild(anchor);
  try {
    anchor.click();
  } finally {
    anchor.remove();
    // Revoke on a microtask so the click's navigation has consumed the URL.
    queueMicrotask(() => revokeUrl(url));
  }
  return blob.size;
}
