type CanvasHeaderProps = {
  title: string;
  workspaceId: string;
  createdAt: string;
  updatedAt: string;
  stats: { nodes: number; edges: number };
  lastSavedAt: string | null;
  isSaving: boolean;
  onSave: () => void;
  onReload: () => void | Promise<void>;
};

export function CanvasHeader({
  title,
  workspaceId,
  createdAt,
  updatedAt,
  stats,
  lastSavedAt,
  isSaving,
  onSave,
  onReload,
}: CanvasHeaderProps) {
  return (
    <>
      <h2>{title}</h2>
      <p className="muted">Workspace: {workspaceId}</p>
      <p className="muted">
        Created: {new Date(createdAt).toLocaleString()} - Updated: {new Date(updatedAt).toLocaleString()}
      </p>
      <div className="document-editor__actions" style={{ marginBottom: "12px", alignItems: "center" }}>
        <span className="muted">
          Nodes: {stats.nodes} • Edges: {stats.edges} {lastSavedAt ? `• Saved at ${lastSavedAt}` : ""}
        </span>
        <div style={{ display: "flex", gap: 8 }}>
          <button onClick={onSave} disabled={isSaving}>
            {isSaving ? "Saving..." : "Save Canvas"}
          </button>
          <button onClick={onReload} disabled={isSaving}>
            Reload
          </button>
        </div>
      </div>
    </>
  );
}
