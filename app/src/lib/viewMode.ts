export type ViewMode = "NSFW" | "SFW";

export const DEFAULT_VIEW_MODE: ViewMode = "NSFW";

export const VIEW_MODE_STORAGE_KEY = "handshake.view_mode";

export function isViewMode(value: unknown): value is ViewMode {
  return value === "NSFW" || value === "SFW";
}

export function loadViewModeFromStorage(): ViewMode {
  try {
    const stored = localStorage.getItem(VIEW_MODE_STORAGE_KEY);
    if (!stored) return DEFAULT_VIEW_MODE;
    const normalized = stored.trim();
    return isViewMode(normalized) ? normalized : DEFAULT_VIEW_MODE;
  } catch {
    return DEFAULT_VIEW_MODE;
  }
}

export function saveViewModeToStorage(mode: ViewMode): void {
  try {
    localStorage.setItem(VIEW_MODE_STORAGE_KEY, mode);
  } catch {
    // Best-effort persistence only.
  }
}

