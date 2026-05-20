import { useState } from "react";

// MT-129: Consent prompt modal surfaced on first cloud send for a
// (session, lane) pair when settings.exec_policy.
// cloud_consent_per_session=true. Per MT-128 ConsentGate the
// operator decision is cached for the session; this modal is the
// UI capture point for that decision. GLOBAL-PRODUCTION
// discipline: technical language only, no moralising copy.

type Props = {
  open: boolean;
  sessionId: string;
  lane: string;
  modelName: string;
  promptPreview?: string;
  onApprove: () => void | Promise<void>;
  onDeny: () => void | Promise<void>;
};

export function ConsentPromptModal({
  open,
  sessionId,
  lane,
  modelName,
  promptPreview,
  onApprove,
  onDeny,
}: Props) {
  const [busy, setBusy] = useState(false);

  if (!open) return null;

  const handleApprove = async () => {
    setBusy(true);
    try {
      await onApprove();
    } finally {
      setBusy(false);
    }
  };

  const handleDeny = async () => {
    setBusy(true);
    try {
      await onDeny();
    } finally {
      setBusy(false);
    }
  };

  return (
    <div
      className="consent-prompt-modal__backdrop"
      data-testid="consent-prompt-modal"
      role="dialog"
      aria-modal="true"
      aria-labelledby="consent-prompt-modal-title"
    >
      <div className="consent-prompt-modal__dialog">
        <h3 id="consent-prompt-modal-title">Cloud send confirmation</h3>
        <p data-testid="consent-prompt-modal.summary">
          First cloud call this session through lane <code>{lane}</code>{" "}
          (model <code>{modelName}</code>, session <code>{sessionId}</code>).
          Approve sends this request and any subsequent calls in this session
          for this lane without re-prompting.
        </p>
        {promptPreview ? (
          <details data-testid="consent-prompt-modal.preview">
            <summary>Prompt preview (operator-authored, verbatim)</summary>
            <pre>{promptPreview}</pre>
          </details>
        ) : null}
        <div className="consent-prompt-modal__actions">
          <button
            type="button"
            onClick={() => void handleApprove()}
            disabled={busy}
            data-testid="consent-prompt-modal.approve"
          >
            Approve cloud send
          </button>
          <button
            type="button"
            onClick={() => void handleDeny()}
            disabled={busy}
            data-testid="consent-prompt-modal.deny"
          >
            Deny
          </button>
        </div>
        <p className="muted" data-testid="consent-prompt-modal.scope-note">
          Decision is per-session per-lane (MT-128 ConsentGate). It does not
          carry to other sessions or other lanes.
        </p>
      </div>
    </div>
  );
}
