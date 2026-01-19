import { useCallback, useMemo, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  FontManifest,
  FontRecord,
  fontsBootstrapPack,
  fontsImport,
  fontsList,
  fontsRemove,
  loadFontFaces,
} from "../lib/fonts";

export function FontManagerView() {
  const [manifest, setManifest] = useState<FontManifest | null>(null);
  const [loading, setLoading] = useState(false);
  const [importing, setImporting] = useState(false);
  const [removingId, setRemovingId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      await fontsBootstrapPack();
      const next = await fontsList();
      setManifest(next);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  const handleImport = useCallback(async () => {
    setImporting(true);
    setError(null);
    try {
      const picked = await open({
        multiple: true,
        filters: [{ name: "Fonts", extensions: ["ttf", "otf", "woff2", "woff"] }],
      });
      if (!picked) return;

      const paths = Array.isArray(picked) ? picked : [picked];
      const result = await fontsImport(paths);
      setManifest(result.manifest);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setImporting(false);
    }
  }, []);

  const handleRemove = useCallback(
    async (fontId: string) => {
      setRemovingId(fontId);
      setError(null);
      try {
        await fontsRemove(fontId);
        const next = await fontsList();
        setManifest(next);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setRemovingId(null);
      }
    },
    [setRemovingId],
  );

  const grouped = useMemo(() => {
    const fonts = manifest?.fonts ?? [];
    const bundled = fonts.filter((f) => f.source === "bundled");
    const user = fonts.filter((f) => f.source === "user");
    return { bundled, user };
  }, [manifest]);

  return (
    <div className="content-card">
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", gap: 12 }}>
        <div>
          <h2>Font Manager</h2>
          <p className="muted">
            Design Pack 40 is bundled offline and copied into <code>{"{APP_DATA}/fonts/bundled/"}</code> on first run.
          </p>
        </div>
        <div style={{ display: "flex", gap: 8 }}>
          <button onClick={() => void refresh()} disabled={loading}>
            {loading ? "Refreshing..." : "Refresh"}
          </button>
          <button onClick={() => void handleImport()} disabled={loading || importing}>
            {importing ? "Importing..." : "Import Font"}
          </button>
        </div>
      </div>

      {!manifest && (
        <div style={{ marginTop: 12 }}>
          <button onClick={() => void refresh()} disabled={loading}>
            {loading ? "Loading..." : "Load Fonts"}
          </button>
        </div>
      )}

      {error && (
        <div style={{ marginTop: 12 }}>
          <p style={{ color: "#b91c1c" }}>{error}</p>
        </div>
      )}

      {manifest && (
        <div style={{ marginTop: 12 }}>
          <p className="muted">
            Pack version: <code>{manifest.packVersion}</code> • Fonts: <code>{manifest.fonts.length}</code> • Generated:{" "}
            <code>{manifest.generatedAt}</code>
          </p>

          <FontTable
            title="Bundled"
            fonts={grouped.bundled}
            allowRemove={false}
            removingId={removingId}
            onRemove={handleRemove}
          />
          <FontTable
            title="User"
            fonts={grouped.user}
            allowRemove={true}
            removingId={removingId}
            onRemove={handleRemove}
          />
        </div>
      )}
    </div>
  );
}

function FontTable({
  title,
  fonts,
  allowRemove,
  removingId,
  onRemove,
}: {
  title: string;
  fonts: FontRecord[];
  allowRemove: boolean;
  removingId: string | null;
  onRemove: (fontId: string) => void | Promise<void>;
}) {
  return (
    <div style={{ marginTop: 16 }}>
      <h3 style={{ marginBottom: 8 }}>{title}</h3>
      {fonts.length === 0 ? (
        <p className="muted">None</p>
      ) : (
        <div style={{ overflowX: "auto" }}>
          <table style={{ width: "100%", borderCollapse: "collapse" }}>
            <thead>
              <tr>
                <th style={{ textAlign: "left", padding: "6px 8px" }}>Family</th>
                <th style={{ textAlign: "left", padding: "6px 8px" }}>Style</th>
                <th style={{ textAlign: "left", padding: "6px 8px" }}>Weight</th>
                <th style={{ textAlign: "left", padding: "6px 8px" }}>License</th>
                <th style={{ textAlign: "left", padding: "6px 8px" }}>Preview</th>
                <th style={{ textAlign: "left", padding: "6px 8px" }}>Actions</th>
              </tr>
            </thead>
            <tbody>
              {fonts.map((font) => (
                <tr key={font.id} style={{ borderTop: "1px solid rgba(0,0,0,0.08)" }}>
                  <td style={{ padding: "6px 8px" }}>
                    <code>{font.family}</code>
                  </td>
                  <td style={{ padding: "6px 8px" }}>{font.style}</td>
                  <td style={{ padding: "6px 8px" }}>{formatWeight(font.weight)}</td>
                  <td style={{ padding: "6px 8px" }}>
                    <code>{font.license.spdx}</code>
                  </td>
                  <td style={{ padding: "6px 8px" }}>
                    <button
                      onClick={() => void loadFontFaces([font])}
                      style={{ marginRight: 8 }}
                      title="Load this font for preview in the UI"
                    >
                      Load
                    </button>
                    <span style={{ fontFamily: font.family }}>
                      The quick brown fox jumps over the lazy dog.
                    </span>
                  </td>
                  <td style={{ padding: "6px 8px" }}>
                    {allowRemove ? (
                      <button
                        onClick={() => void onRemove(font.id)}
                        disabled={removingId === font.id}
                        title="Removes only from {APP_DATA}/fonts/user/"
                      >
                        {removingId === font.id ? "Removing..." : "Remove"}
                      </button>
                    ) : (
                      <span className="muted">—</span>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

function formatWeight(weight: FontRecord["weight"]): string {
  return weight.type === "variable" ? `${weight.min}-${weight.max}` : String(weight.value);
}
