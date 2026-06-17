import { useEffect, useState } from "react";
import { getLoomBlock, updateLoomBlock, type LoomBlock } from "../lib/api";

type Props = {
  workspaceId: string;
  blockId: string;
};

type LoomBlockUpdatedDetail = {
  workspaceId?: string | null;
  block?: LoomBlock | null;
};

function blockTitle(block: LoomBlock): string {
  return block.title?.trim() || block.original_filename?.trim() || block.block_id;
}

function optionalText(value: string | number | boolean | null | undefined): string {
  if (value === null || value === undefined || value === "") return "none";
  return String(value);
}

export function LoomBlockPanel({ workspaceId, blockId }: Props) {
  const [block, setBlock] = useState<LoomBlock | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [titleDraft, setTitleDraft] = useState("");
  const [pinnedDraft, setPinnedDraft] = useState(false);
  const [favoriteDraft, setFavoriteDraft] = useState(false);
  const [savingProperties, setSavingProperties] = useState(false);
  const [propertiesStatus, setPropertiesStatus] = useState<string | null>(null);
  const [propertiesError, setPropertiesError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    const timer = window.setTimeout(() => {
      if (cancelled) return;
      setLoading(true);
      setError(null);
      setBlock(null);

      getLoomBlock(workspaceId, blockId)
        .then((nextBlock) => {
          if (!cancelled) {
            setBlock(nextBlock);
            setTitleDraft(nextBlock.title ?? "");
            setPinnedDraft(nextBlock.pinned);
            setFavoriteDraft(nextBlock.favorite);
            setPropertiesStatus(null);
            setPropertiesError(null);
          }
        })
        .catch((err) => {
          if (!cancelled) setError(err instanceof Error ? err.message : "Loom block lookup failed");
        })
        .finally(() => {
          if (!cancelled) setLoading(false);
        });
    }, 0);

    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [workspaceId, blockId]);

  useEffect(() => {
    const handleLoomBlockUpdated = (event: Event) => {
      const detail = (event as CustomEvent<LoomBlockUpdatedDetail>).detail;
      const updatedBlock = detail?.block;
      if (!updatedBlock) return;
      if (detail?.workspaceId && detail.workspaceId !== workspaceId) return;
      if (updatedBlock.block_id !== blockId) return;
      setBlock(updatedBlock);
      setTitleDraft(updatedBlock.title ?? "");
      setPinnedDraft(updatedBlock.pinned);
      setFavoriteDraft(updatedBlock.favorite);
      setPropertiesError(null);
    };

    window.addEventListener("handshake:loom-block-updated", handleLoomBlockUpdated);
    return () => {
      window.removeEventListener("handshake:loom-block-updated", handleLoomBlockUpdated);
    };
  }, [blockId, workspaceId]);

  const saveProperties = async () => {
    if (!block) return;
    setSavingProperties(true);
    setPropertiesStatus(null);
    setPropertiesError(null);
    try {
      const updated = await updateLoomBlock(workspaceId, block.block_id, {
        title: titleDraft.trim() || null,
        pinned: pinnedDraft,
        favorite: favoriteDraft,
      });
      if (updated.pinned !== block.pinned) {
        window.dispatchEvent(
          new CustomEvent("handshake:loom-bookmarks-changed", {
            detail: { workspaceId, blockId: updated.block_id, pinned: updated.pinned },
          }),
        );
      }
      window.dispatchEvent(
        new CustomEvent("handshake:loom-block-updated", {
          detail: { workspaceId, block: updated },
        }),
      );
      setBlock(updated);
      setTitleDraft(updated.title ?? "");
      setPinnedDraft(updated.pinned);
      setFavoriteDraft(updated.favorite);
      setPropertiesStatus("Properties saved");
    } catch (err) {
      setPropertiesError(err instanceof Error ? err.message : "Properties save failed");
    } finally {
      setSavingProperties(false);
    }
  };

  if (loading) {
    return (
      <div className="content-card loom-block-panel" data-testid="loom-block-panel">
        <p>Loading Loom block...</p>
      </div>
    );
  }

  if (error || !block) {
    return (
      <div className="content-card loom-block-panel error" data-testid="loom-block-panel">
        <h2>Loom Block</h2>
        <p>{error ?? "Loom block unavailable"}</p>
      </div>
    );
  }

  return (
    <div className="content-card loom-block-panel" data-testid="loom-block-panel">
      <header className="loom-block-panel__header">
        <div>
          <p className="app-eyebrow">Loom Block</p>
          <h2>{blockTitle(block)}</h2>
        </div>
        <span className="kernel-dcc__badge">{block.content_type}</span>
      </header>
      <dl className="loom-block-panel__facts">
        <div>
          <dt>Block</dt>
          <dd>{block.block_id}</dd>
        </div>
        <div>
          <dt>Workspace</dt>
          <dd>{block.workspace_id}</dd>
        </div>
        <div>
          <dt>Document</dt>
          <dd>{optionalText(block.document_id)}</dd>
        </div>
        <div>
          <dt>Content Hash</dt>
          <dd>{optionalText(block.content_hash)}</dd>
        </div>
        <div>
          <dt>Links</dt>
          <dd>
            {block.derived.backlink_count} backlinks / {block.derived.mention_count} mentions /{" "}
            {block.derived.tag_count} tags
          </dd>
        </div>
        <div>
          <dt>Preview</dt>
          <dd>{block.derived.preview_status}</dd>
        </div>
      </dl>
      <section className="loom-block-panel__properties" data-testid="loom-block-properties">
        <h3>Properties</h3>
        <form
          onSubmit={(event) => {
            event.preventDefault();
            void saveProperties();
          }}
        >
          <label className="loom-block-panel__field">
            <span>Title</span>
            <input
              type="text"
              data-testid="loom-block-properties.title"
              value={titleDraft}
              onChange={(event) => setTitleDraft(event.currentTarget.value)}
            />
          </label>
          <label className="loom-block-panel__check">
            <input
              type="checkbox"
              data-testid="loom-block-properties.pinned"
              checked={pinnedDraft}
              onChange={(event) => setPinnedDraft(event.currentTarget.checked)}
            />
            <span>Pinned</span>
          </label>
          <label className="loom-block-panel__check">
            <input
              type="checkbox"
              data-testid="loom-block-properties.favorite"
              checked={favoriteDraft}
              onChange={(event) => setFavoriteDraft(event.currentTarget.checked)}
            />
            <span>Favorite</span>
          </label>
          <button
            type="submit"
            className="tt-button"
            data-testid="loom-block-properties.save"
            disabled={savingProperties}
          >
            {savingProperties ? "Saving..." : "Save properties"}
          </button>
        </form>
        {propertiesStatus ? (
          <p role="status" data-testid="loom-block-properties.status">
            {propertiesStatus}
          </p>
        ) : null}
        {propertiesError ? (
          <p role="alert" data-testid="loom-block-properties.error">
            {propertiesError}
          </p>
        ) : null}
      </section>
      {block.derived.full_text_index ? (
        <section className="loom-block-panel__body">
          <h3>Indexed Text</h3>
          <p>{block.derived.full_text_index}</p>
        </section>
      ) : null}
    </div>
  );
}
