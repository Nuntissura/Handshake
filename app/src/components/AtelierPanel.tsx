import React, { useEffect, useState } from "react";
import {
  AtelierCommandCorpusEntry,
  AtelierIntakeBatch,
  AtelierIntakeItems,
  AtelierOverview,
  AtelierStealthWindow,
  getAtelierIntakeItems,
  getAtelierOverview,
  listAtelierCommandCorpus,
  listAtelierIntakeBatches,
  listAtelierStealthWindows,
  openAtelierIntakeBatch,
} from "../lib/api";

type Section = "overview" | "intake" | "corpus" | "stealth";

const SECTIONS: { id: Section; label: string }[] = [
  { id: "overview", label: "Overview" },
  { id: "intake", label: "Intake" },
  { id: "corpus", label: "Command Corpus" },
  { id: "stealth", label: "Stealth Windows" },
];

function errorMessage(err: unknown, fallback: string): string {
  return err instanceof Error ? err.message : fallback;
}

function generateIdempotencyKey(): string {
  // crypto.randomUUID is collision-proof; the backend treats idempotency_key as
  // a uniqueness key (ON CONFLICT), so a Math.random collision would silently
  // return a pre-existing batch instead of opening a new one.
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return `ui-${crypto.randomUUID()}`;
  }
  return `ui-${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

const OverviewSection: React.FC = () => {
  const [data, setData] = useState<AtelierOverview | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    getAtelierOverview()
      .then((overview) => {
        if (!cancelled) {
          setData(overview);
          setError(null);
        }
      })
      .catch((err) => {
        if (!cancelled) setError(errorMessage(err, "Failed to load overview"));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  if (loading) {
    return (
      <p data-testid="atelier-overview-loading" data-stable-id="atelier-overview-loading">
        Loading overview...
      </p>
    );
  }
  if (error) {
    return (
      <p
        className="error"
        data-testid="atelier-overview-error"
        data-stable-id="atelier-overview-error"
      >
        Error: {error}
      </p>
    );
  }
  if (!data) {
    return <p className="muted">No overview data.</p>;
  }

  return (
    <div>
      <h3>Tables</h3>
      <div className="table-scroll">
        <table
          className="data-table"
          data-testid="atelier-overview-table"
          data-stable-id="atelier-overview-table"
        >
          <thead>
            <tr>
              <th>Name</th>
              <th>Rows</th>
            </tr>
          </thead>
          <tbody>
            {data.tables.map((table) => (
              <tr key={table.name}>
                <td>{table.name}</td>
                <td>{table.rows}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <h3 style={{ marginTop: 16 }}>Event Families</h3>
      <div className="table-scroll">
        <table className="data-table">
          <thead>
            <tr>
              <th>Family</th>
              <th>Count</th>
            </tr>
          </thead>
          <tbody>
            {data.event_families.map((family) => (
              <tr key={family.family}>
                <td>{family.family}</td>
                <td>{family.count}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
};

const IntakeSection: React.FC = () => {
  const [batches, setBatches] = useState<AtelierIntakeBatch[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [sourceLabel, setSourceLabel] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [selectedBatchId, setSelectedBatchId] = useState<string | null>(null);
  const [items, setItems] = useState<AtelierIntakeItems | null>(null);
  const [itemsLoading, setItemsLoading] = useState(false);
  const [itemsError, setItemsError] = useState<string | null>(null);

  const loadBatches = async () => {
    setLoading(true);
    try {
      const data = await listAtelierIntakeBatches();
      setBatches(data);
      setError(null);
    } catch (err) {
      setError(errorMessage(err, "Failed to load batches"));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    listAtelierIntakeBatches()
      .then((data) => {
        if (!cancelled) {
          setBatches(data);
          setError(null);
        }
      })
      .catch((err) => {
        if (!cancelled) setError(errorMessage(err, "Failed to load batches"));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const openBatch = async () => {
    setSubmitting(true);
    setError(null);
    try {
      await openAtelierIntakeBatch(generateIdempotencyKey(), sourceLabel);
      setSourceLabel("");
      await loadBatches();
    } catch (err) {
      setError(errorMessage(err, "Failed to open batch"));
    } finally {
      setSubmitting(false);
    }
  };

  const loadItems = async (batchId: string) => {
    setSelectedBatchId(batchId);
    setItems(null);
    setItemsError(null);
    setItemsLoading(true);
    try {
      const data = await getAtelierIntakeItems(batchId);
      setItems(data);
    } catch (err) {
      setItemsError(errorMessage(err, "Failed to load items"));
    } finally {
      setItemsLoading(false);
    }
  };

  return (
    <div>
      <div className="filter-actions" style={{ marginBottom: 12 }}>
        <input
          placeholder="Source label"
          value={sourceLabel}
          onChange={(e) => setSourceLabel(e.target.value)}
          data-testid="atelier-intake-source-label"
          data-stable-id="atelier-intake-source-label"
        />
        <button
          type="button"
          className="primary"
          disabled={submitting}
          onClick={openBatch}
          data-testid="atelier-intake-open"
          data-stable-id="atelier-intake-open"
        >
          Open Batch
        </button>
      </div>

      {error && <p className="error small">Error: {error}</p>}

      <h3>Batches</h3>
      {loading && batches.length === 0 ? (
        <p>Loading batches...</p>
      ) : (
        <div
          className="table-scroll"
          data-testid="atelier-intake-batches"
          data-stable-id="atelier-intake-batches"
        >
          <table className="data-table">
            <thead>
              <tr>
                <th>Batch ID</th>
                <th>Source Label</th>
                <th>Status</th>
                <th>Created (UTC)</th>
              </tr>
            </thead>
            <tbody>
              {batches.map((batch) => (
                <tr
                  key={batch.batch_id}
                  className={`clickable-row ${
                    selectedBatchId === batch.batch_id ? "job-card--active" : ""
                  }`}
                  onClick={() => loadItems(batch.batch_id)}
                >
                  <td>{batch.batch_id}</td>
                  <td>{batch.source_label}</td>
                  <td>{batch.status}</td>
                  <td className="muted">{batch.created_at_utc}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {selectedBatchId && (
        <div
          style={{ marginTop: 16 }}
          data-testid="atelier-intake-items"
          data-stable-id="atelier-intake-items"
        >
          <h3>Items for {selectedBatchId}</h3>
          {itemsLoading ? (
            <p>Loading items...</p>
          ) : itemsError ? (
            <p className="error small">Error: {itemsError}</p>
          ) : items ? (
            <>
              <ul className="meta-list">
                <li>New: {items.lane_counts.new}</li>
                <li>Accepted: {items.lane_counts.accepted}</li>
                <li>Rejected: {items.lane_counts.rejected}</li>
                <li>Deferred: {items.lane_counts.deferred}</li>
              </ul>
              <div className="table-scroll">
                <table className="data-table">
                  <thead>
                    <tr>
                      <th>Item ID</th>
                      <th>File Name</th>
                      <th>Source Path</th>
                      <th>Lane</th>
                      <th>Bytes</th>
                    </tr>
                  </thead>
                  <tbody>
                    {items.items.map((item) => (
                      <tr key={item.item_id}>
                        <td>{item.item_id}</td>
                        <td>{item.file_name}</td>
                        <td className="muted">{item.source_path}</td>
                        <td>{item.lane}</td>
                        <td>{item.byte_len}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </>
          ) : (
            <p className="muted">No items.</p>
          )}
        </div>
      )}
    </div>
  );
};

const CommandCorpusSection: React.FC = () => {
  const [entries, setEntries] = useState<AtelierCommandCorpusEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    listAtelierCommandCorpus()
      .then((data) => {
        if (!cancelled) {
          setEntries(data);
          setError(null);
        }
      })
      .catch((err) => {
        if (!cancelled) setError(errorMessage(err, "Failed to load command corpus"));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  if (loading) {
    return <p>Loading command corpus...</p>;
  }
  if (error) {
    return <p className="error small">Error: {error}</p>;
  }

  return (
    <div className="table-scroll">
      <table
        className="data-table"
        data-testid="atelier-corpus-table"
        data-stable-id="atelier-corpus-table"
      >
        <thead>
          <tr>
            <th>Entry ID</th>
            <th>Action ID</th>
            <th>Owner</th>
            <th>Execution Class</th>
            <th>Foreground</th>
            <th>Manual Anchor</th>
          </tr>
        </thead>
        <tbody>
          {entries.map((entry) => (
            <tr key={entry.entry_id}>
              <td>{entry.entry_id}</td>
              <td>{entry.action_id}</td>
              <td>{entry.owner}</td>
              <td>{entry.execution_class}</td>
              <td>{entry.foreground_flag ? "yes" : "no"}</td>
              <td className="muted">{entry.manual_anchor}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
};

const StealthWindowsSection: React.FC = () => {
  const [windows, setWindows] = useState<AtelierStealthWindow[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    listAtelierStealthWindows()
      .then((data) => {
        if (!cancelled) {
          setWindows(data);
          setError(null);
        }
      })
      .catch((err) => {
        if (!cancelled) setError(errorMessage(err, "Failed to load stealth windows"));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  if (loading) {
    return <p>Loading stealth windows...</p>;
  }
  if (error) {
    return <p className="error small">Error: {error}</p>;
  }

  return (
    <div className="table-scroll">
      <table
        className="data-table"
        data-testid="atelier-stealth-table"
        data-stable-id="atelier-stealth-table"
      >
        <thead>
          <tr>
            <th>Window Ref ID</th>
            <th>Owner Actor</th>
            <th>Title</th>
            <th>Visibility</th>
            <th>Status</th>
            <th>Revision</th>
          </tr>
        </thead>
        <tbody>
          {windows.map((win) => (
            <tr key={win.window_ref_id}>
              <td>{win.window_ref_id}</td>
              <td>{win.owner_actor}</td>
              <td>{win.title}</td>
              <td>{win.visibility}</td>
              <td>{win.status}</td>
              <td>{win.revision}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
};

const AtelierPanel: React.FC = () => {
  const [activeSection, setActiveSection] = useState<Section>("overview");

  return (
    <div className="content-card" data-testid="atelier-panel" data-stable-id="atelier-panel">
      <div className="card-header">
        <div>
          <h2>Atelier</h2>
          <p className="muted">
            Navigate the WP-KERNEL-005 atelier domain: overview, intake batches, command corpus, and
            stealth windows.
          </p>
        </div>
      </div>

      <div className="jobs-layout">
        <div className="jobs-list">
          {SECTIONS.map((section) => {
            const selected = activeSection === section.id;
            return (
              <button
                key={section.id}
                type="button"
                className={`job-card ${selected ? "job-card--active" : ""}`}
                aria-current={selected ? "page" : undefined}
                onClick={() => setActiveSection(section.id)}
                data-testid={`atelier-subnav-${section.id}`}
                data-stable-id={`atelier-subnav-${section.id}`}
              >
                {section.label}
              </button>
            );
          })}
        </div>

        <div className="job-inspector">
          {activeSection === "overview" && (
            <div
              data-testid="atelier-section-overview"
              data-stable-id="atelier-section-overview"
            >
              <OverviewSection />
            </div>
          )}
          {activeSection === "intake" && (
            <div data-testid="atelier-section-intake" data-stable-id="atelier-section-intake">
              <IntakeSection />
            </div>
          )}
          {activeSection === "corpus" && (
            <div data-testid="atelier-section-corpus" data-stable-id="atelier-section-corpus">
              <CommandCorpusSection />
            </div>
          )}
          {activeSection === "stealth" && (
            <div data-testid="atelier-section-stealth" data-stable-id="atelier-section-stealth">
              <StealthWindowsSection />
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default AtelierPanel;
