---
file_id: motion-preference-readme
file_kind: product-module-readme
updated_at: 2026-10-28
---

<topic id="motion-preference">

# Motion preference

This Native workspace package owns per-context motion timing and the complete theme-toggle widget. It uses egui; Native owns theme meaning, palette installation, workspace settings, authenticated saving and account lifecycle.

Call `apply_motion_preference` with the actual context and resolved workspace preference. Each context retains its own Light and Dark normal durations. Repeated equal preferences do nothing. Restoring motion reinstates those separate durations; restore it before discarding the context's temporary data. Native selects the root, detached and capture contexts and preserves its reset ordering.

Call `theme_toggle` with the existing stable widget ID and the target theme label. It returns the live response for Native's author-ID attachment and click dispatch. The widget preserves its interaction rectangle and immediate text, focus, pressed and disabled feedback; decorative hover fill uses egui's existing interpolation.

The package is a member of Native's existing workspace through its local path dependency. It shares the workspace lock, profiles and configured build output. Its three context tests live here; workspace settings and their two tests remain in Native.

</topic>
