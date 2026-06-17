export type EditorFindOptions = {
  query: string;
  caseSensitive: boolean;
  wholeWord: boolean;
  isRegex: boolean;
};

export type EditorFindRequest = EditorFindOptions & {
  requestId: number;
};
