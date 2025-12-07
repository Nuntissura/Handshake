import "./App.css";
import { SystemStatus } from "./components/SystemStatus";
import { useState } from "react";
import { WorkspaceSidebar } from "./components/WorkspaceSidebar";
import { DocumentView } from "./components/DocumentView";
import { CanvasView } from "./components/CanvasView";
import { DebugPanel } from "./components/DebugPanel";

function App() {
  const [selectedDocumentId, setSelectedDocumentId] = useState<string | null>(null);
  const [selectedCanvasId, setSelectedCanvasId] = useState<string | null>(null);

  return (
    <main className="app-shell">
      <header className="app-header">
        <div>
          <p className="app-eyebrow">Handshake</p>
          <h1 className="app-title">Desktop Shell</h1>
          <p className="app-subtitle">Coordinator, workspaces, documents, and canvases.</p>
        </div>
        <SystemStatus />
      </header>

      <div className="app-body">
        <WorkspaceSidebar
          selectedDocumentId={selectedDocumentId}
          selectedCanvasId={selectedCanvasId}
          onSelectDocument={(id) => {
            setSelectedDocumentId(id);
            if (id !== null) setSelectedCanvasId(null);
          }}
          onSelectCanvas={(id) => {
            setSelectedCanvasId(id);
            if (id !== null) setSelectedDocumentId(null);
          }}
        />

        <div className="content-panel">
          <DebugPanel />
          {selectedDocumentId ? (
            <DocumentView documentId={selectedDocumentId} />
          ) : selectedCanvasId ? (
            <CanvasView canvasId={selectedCanvasId} />
          ) : (
            <div className="content-card">
              <h2>Welcome</h2>
              <p className="muted">
                Select or create a workspace, then add documents or canvases to view their details.
              </p>
            </div>
          )}
        </div>
      </div>
    </main>
  );
}

export default App;
