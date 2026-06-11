// WP-KERNEL-009 / MT-031 — DependencyFailureMessaging banner.
//
// Real UI surface for the typed bundled-dependency failure registry
// (src/lib/dependency_policy/dependency_failure.ts). When a bundled editor
// dependency fails to initialize at runtime — Monaco worker construction,
// Tiptap extension init, editor mount — the user must see an ACTIONABLE
// message instead of a blank editor or a silent console error.
//
// Mounted wherever editors mount (DocumentView next to TiptapEditor, the
// dependency-policy harness next to Monaco+Tiptap). Renders nothing while no
// failure is recorded; data-testid="dependency-failure-banner" is the stable
// hook for tests and visual debugging.
//
// All editor assets ship inside Handshake (MT-017/MT-018 bundling policy), so
// the guidance deliberately says NO download/network step can fix a failure:
// the recovery path is restart, then reinstall if the installation is
// corrupted.

import { useEffect, useState } from "react";
import {
  dependencyFailures,
  type DependencyFailure,
} from "../lib/dependency_policy/dependency_failure";

export interface DependencyFailureBannerProps {
  /** Registry override for tests; defaults to the application-wide registry. */
  registry?: typeof dependencyFailures;
}

export function DependencyFailureBanner({
  registry = dependencyFailures,
}: DependencyFailureBannerProps) {
  const [failures, setFailures] = useState<readonly DependencyFailure[]>(() => registry.list());
  const [dismissed, setDismissed] = useState(false);

  useEffect(() => {
    // Catch up on failures reported between render and subscription, then
    // follow the registry live. New failures re-surface a dismissed banner.
    setFailures([...registry.list()]);
    const unsubscribe = registry.subscribe(() => {
      setFailures([...registry.list()]);
      setDismissed(false);
    });
    return unsubscribe;
  }, [registry]);

  if (dismissed || failures.length === 0) return null;

  return (
    <div
      data-testid="dependency-failure-banner"
      role="alert"
      className="dependency-failure-banner"
      style={{
        border: "1px solid #b3261e",
        background: "#fdecea",
        color: "#5f1410",
        padding: "8px 12px",
        marginBottom: 8,
        borderRadius: 4,
        fontSize: 13,
      }}
    >
      <strong>Bundled editor component failed to load.</strong>
      <ul style={{ margin: "6px 0", paddingLeft: 18 }}>
        {failures.map((failure, index) => (
          <li key={`${failure.dependency}-${failure.component}-${index}`}>
            {failure.message}
            {failure.cause ? <span className="muted"> (cause: {failure.cause})</span> : null}
          </li>
        ))}
      </ul>
      <p style={{ margin: "4px 0" }}>
        What you can do: the affected editor keeps running in degraded mode where possible.
        Restart Handshake to reinitialize the bundled components; if the failure persists,
        the installation is likely corrupted — reinstall Handshake. No internet connection
        is required and no download will be attempted: every editor asset ships inside the
        application.
      </p>
      <button
        type="button"
        data-testid="dependency-failure-banner-dismiss"
        onClick={() => setDismissed(true)}
      >
        Dismiss
      </button>
    </div>
  );
}
