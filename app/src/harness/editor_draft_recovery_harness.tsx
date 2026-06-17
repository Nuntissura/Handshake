// WP-KERNEL-009 / MT-255 — RichDocument draft recovery proof harness.
//
// Mounts the real RichDocumentView against loopback API endpoints that the
// Playwright spec fulfills. No localStorage draft path is provided here.

import { StrictMode, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { RichDocumentView } from "../components/RichDocumentView";
import "../App.css";

const DOCUMENT_ID = "KRD-00000000000000000000000000000255";

interface DraftRecoveryHarnessState {
  documentId: string;
  dirty: boolean;
}

declare global {
  interface Window {
    __HS_EDITOR_DRAFT_RECOVERY_HARNESS__?: DraftRecoveryHarnessState;
  }
}

function HarnessShell() {
  const [dirty, setDirty] = useState(false);

  useEffect(() => {
    window.__HS_EDITOR_DRAFT_RECOVERY_HARNESS__ = { documentId: DOCUMENT_ID, dirty };
  }, [dirty]);

  return (
    <main data-testid="editor-draft-recovery-harness-root">
      <RichDocumentView documentId={DOCUMENT_ID} onDirtyChange={setDirty} />
    </main>
  );
}

const root = createRoot(document.getElementById("root")!);
root.render(
  <StrictMode>
    <HarnessShell />
  </StrictMode>,
);
