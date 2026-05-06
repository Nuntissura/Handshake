import { registerFailCaptureHook, failWithMemory } from "../scripts/lib/fail-capture-lib.mjs";
import { validateActiveWpDossiers } from "../scripts/audit/wp-dossier-runtime.mjs";

registerFailCaptureHook("wp-dossier-runtime-check.mjs", { role: "SHARED" });

const result = validateActiveWpDossiers();

if (result.errors.length > 0) {
  failWithMemory("wp-dossier-runtime-check.mjs", "WP dossier runtime contract violations found", {
    role: "SHARED",
    details: [
      ...result.errors,
      "Run: node .GOV/roles_shared/scripts/audit/wp-dossier-runtime.mjs --sync",
    ],
  });
}

console.log(`wp-dossier-runtime-check ok (${result.activeWpIds.length} active WP dossier(s))`);
