import { ReactNode, useMemo, useState } from "react";

export type CommandPaletteAction = {
  id: string;
  label: string;
  description?: string;
  keywords?: string[];
  disabled?: boolean;
};

type Props = {
  open: boolean;
  title?: string;
  actions: CommandPaletteAction[];
  footer?: ReactNode;
  onAction: (actionId: string) => void;
  onClose: () => void;
};

function matchesQuery(action: CommandPaletteAction, query: string) {
  const q = query.trim().toLowerCase();
  if (q.length === 0) return true;
  const haystack = [action.label, action.description, ...(action.keywords ?? [])]
    .filter(Boolean)
    .join(" ")
    .toLowerCase();
  return haystack.includes(q);
}

export function CommandPalette({ open, title = "Command Palette", actions, footer, onAction, onClose }: Props) {
  const [query, setQuery] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);

  const filtered = useMemo(() => actions.filter((a) => matchesQuery(a, query)), [actions, query]);
  const safeSelectedIndex = Math.min(Math.max(selectedIndex, 0), Math.max(filtered.length - 1, 0));

  if (!open) return null;

  const runSelected = () => {
    const action = filtered[safeSelectedIndex];
    if (!action || action.disabled) return;
    onAction(action.id);
  };

  return (
    <div
      className="command-palette-overlay"
      role="dialog"
      aria-modal="true"
      aria-label={title}
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          onClose();
          return;
        }
        if (event.key === "ArrowDown") {
          event.preventDefault();
          setSelectedIndex((i) => Math.min(Math.max(i, 0) + 1, filtered.length - 1));
          return;
        }
        if (event.key === "ArrowUp") {
          event.preventDefault();
          setSelectedIndex((i) => Math.max(Math.min(i, filtered.length - 1) - 1, 0));
          return;
        }
        if (event.key === "Enter") {
          event.preventDefault();
          runSelected();
        }
      }}
    >
      <div className="command-palette" onMouseDown={(event) => event.stopPropagation()}>
        <div className="command-palette__header">
          <div>
            <p className="drawer-eyebrow">AI Actions</p>
            <h3>{title}</h3>
          </div>
          <button type="button" className="secondary" onClick={onClose}>
            Close
          </button>
        </div>

        <input
          className="command-palette__search"
          type="text"
          value={query}
          placeholder="Search actions..."
          onChange={(event) => {
            setQuery(event.target.value);
            setSelectedIndex(0);
          }}
        />

        <ul className="command-palette__list" role="listbox" aria-label="Actions">
          {filtered.length === 0 ? (
            <li className="muted">No matching actions.</li>
          ) : (
            filtered.map((action, idx) => {
              const isSelected = idx === safeSelectedIndex;
              const disabled = Boolean(action.disabled);
              return (
                <li key={action.id}>
                  <button
                    type="button"
                    className={
                      isSelected
                        ? "command-palette__item command-palette__item--selected"
                        : "command-palette__item"
                    }
                    onMouseEnter={() => setSelectedIndex(idx)}
                    onClick={() => onAction(action.id)}
                    disabled={disabled}
                    aria-selected={isSelected}
                    role="option"
                  >
                    <div className="command-palette__item-title">
                      <strong>{action.label}</strong>
                      {action.description && <span className="muted">{action.description}</span>}
                    </div>
                  </button>
                </li>
              );
            })
          )}
        </ul>

        {footer && <div className="command-palette__footer">{footer}</div>}
      </div>
    </div>
  );
}
