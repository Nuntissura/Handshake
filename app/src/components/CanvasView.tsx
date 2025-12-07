import { useEffect, useState } from "react";
import { CanvasWithGraph, getCanvas } from "../lib/api";

type Props = {
  canvasId: string | null;
};

export function CanvasView({ canvasId }: Props) {
  const [canvas, setCanvas] = useState<CanvasWithGraph | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!canvasId) {
      setCanvas(null);
      setError(null);
      return;
    }
    const load = async () => {
      setLoading(true);
      setError(null);
      try {
        const data = await getCanvas(canvasId);
        setCanvas(data);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to load canvas");
      } finally {
        setLoading(false);
      }
    };
    void load();
  }, [canvasId]);

  if (!canvasId) {
    return (
      <div className="content-card">
        <h2>Canvas</h2>
        <p className="muted">No canvas selected.</p>
      </div>
    );
  }

  if (loading) {
    return (
      <div className="content-card">
        <h2>Canvas</h2>
        <p className="muted">Loading…</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="content-card">
        <h2>Canvas</h2>
        <p className="muted">Error: {error}</p>
      </div>
    );
  }

  if (!canvas) return null;

  return (
    <div className="content-card">
      <h2>{canvas.title}</h2>
      <p className="muted">Workspace: {canvas.workspace_id}</p>
      <p className="muted">
        Created: {new Date(canvas.created_at).toLocaleString()} — Updated: {new Date(canvas.updated_at).toLocaleString()}
      </p>
      <p>
        Nodes: {canvas.nodes.length} — Edges: {canvas.edges.length}
      </p>

      <h3>Nodes</h3>
      {canvas.nodes.length === 0 ? (
        <p className="muted">No nodes.</p>
      ) : (
        <ul className="list-inline">
          {canvas.nodes.map((node) => (
            <li key={node.id} className="content-card" style={{ padding: "10px" }}>
              <strong>{node.kind}</strong> ({node.position_x}, {node.position_y})
            </li>
          ))}
        </ul>
      )}

      <h3>Edges</h3>
      {canvas.edges.length === 0 ? (
        <p className="muted">No edges.</p>
      ) : (
        <ul className="list-inline">
          {canvas.edges.map((edge) => (
            <li key={edge.id} className="content-card" style={{ padding: "10px" }}>
              <strong>{edge.kind}</strong>: {edge.from_node_id} → {edge.to_node_id}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
