import { useRef } from "react";
import { Excalidraw } from "@excalidraw/excalidraw";
import "@excalidraw/excalidraw/index.css";
import { ExcalidrawElement } from "@excalidraw/excalidraw/element/types";
import { BinaryFiles } from "@excalidraw/excalidraw/types";

type ExcalidrawCanvasProps = {
  initialElements: readonly ExcalidrawElement[] | null;
  initialFiles?: BinaryFiles | null;
  onChange: (elements: readonly ExcalidrawElement[], files: BinaryFiles) => void;
  readOnly?: boolean;
};

// Keeps Excalidraw mostly uncontrolled: initial data is applied once via a ref,
// subsequent edits flow upward through onChange without re-seeding the canvas.
export function ExcalidrawCanvas({
  initialElements,
  initialFiles,
  onChange,
  readOnly = false,
}: ExcalidrawCanvasProps) {
  const initialDataRef = useRef<{ elements: readonly ExcalidrawElement[]; files?: BinaryFiles } | null>(null);

  if (!initialDataRef.current) {
    initialDataRef.current = { elements: initialElements ?? [], files: initialFiles ?? undefined };
  }

  return (
    <div className="excalidraw-wrapper">
      <Excalidraw
        initialData={initialDataRef.current}
        onChange={(elements, _appState, files) => onChange(elements, files)}
        viewModeEnabled={readOnly}
      />
    </div>
  );
}
