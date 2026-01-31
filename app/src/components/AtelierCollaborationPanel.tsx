type Props = {
  open: boolean;
  onClose: () => void;
  selectionText: string;
  roles: string[];
};

export function AtelierCollaborationPanel({ open, onClose, selectionText, roles }: Props) {
  if (!open) return null;

  const selectionPreview = selectionText.trim();

  return (
    <aside className="atelier-collab-panel" aria-label="Atelier collaboration panel">
      <div className="atelier-collab-panel__header">
        <h3 className="atelier-collab-panel__title">Atelier Collaboration</h3>
        <button type="button" className="atelier-collab-panel__close" onClick={onClose} aria-label="Close">
          ×
        </button>
      </div>

      <section className="atelier-collab-panel__section">
        <h4 className="atelier-collab-panel__section-title">Selection</h4>
        {selectionPreview.length === 0 ? (
          <p className="muted">No selection.</p>
        ) : (
          <pre className="atelier-collab-panel__selection">{selectionPreview}</pre>
        )}
      </section>

      <section className="atelier-collab-panel__section">
        <h4 className="atelier-collab-panel__section-title">Roles</h4>
        {roles.length === 0 ? (
          <p className="muted">No roles configured.</p>
        ) : (
          <ul className="atelier-collab-panel__roles">
            {roles.map((role) => (
              <li key={role} className="atelier-collab-panel__role">
                {role}
              </li>
            ))}
          </ul>
        )}
      </section>
    </aside>
  );
}
