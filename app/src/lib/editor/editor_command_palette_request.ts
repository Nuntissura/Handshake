export type EditorCommandPaletteRequest = {
  paneId: string;
  documentId?: string;
  requestId: number;
  query: string;
};
