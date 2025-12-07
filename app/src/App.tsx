import "./App.css";
import { SystemStatus } from "./components/SystemStatus";

function App() {
  return (
    <main className="app-shell">
      <header className="app-header">
        <div>
          <p className="app-eyebrow">Handshake</p>
          <h1 className="app-title">Desktop Shell</h1>
          <p className="app-subtitle">Coordinator and UI status at a glance.</p>
        </div>
        <SystemStatus />
      </header>
    </main>
  );
}

export default App;
