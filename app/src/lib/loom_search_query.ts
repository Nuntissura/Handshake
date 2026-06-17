import type { LoomGraphSearchSourceKind } from "./api";

const LOOM_SEARCH_SOURCE_KINDS: LoomGraphSearchSourceKind[] = [
  "loom_block",
  "file",
  "tag_hub",
  "document",
  "symbol",
  "work_packet",
  "micro_task",
  "user_manual_page",
  "wiki_page",
];

const LOOM_SEARCH_SOURCE_KIND_SET = new Set<string>(LOOM_SEARCH_SOURCE_KINDS);
const OPERATOR_PATTERN = /^([a-zA-Z_][a-zA-Z0-9_]*):(.*)$/;

export type ParsedLoomSearchOperators = {
  q: string;
  tagIds: string[];
  mentionIds: string[];
  sourceKinds: LoomGraphSearchSourceKind[];
  path?: string;
  errors: string[];
};

type QueryToken = {
  raw: string;
  value: string;
};

function splitQueryTokens(query: string): QueryToken[] {
  const tokens: QueryToken[] = [];
  let current = "";
  let quoted = false;

  for (const ch of query) {
    if (ch === '"') {
      quoted = !quoted;
      current += ch;
      continue;
    }
    if (/\s/.test(ch) && !quoted) {
      if (current.trim()) tokens.push({ raw: current, value: unquote(current) });
      current = "";
      continue;
    }
    current += ch;
  }

  if (current.trim()) tokens.push({ raw: current, value: unquote(current) });
  return tokens;
}

function unquote(value: string): string {
  const trimmed = value.trim();
  if (trimmed.length >= 2 && trimmed.startsWith('"') && trimmed.endsWith('"')) {
    return trimmed.slice(1, -1);
  }
  return trimmed;
}

function splitOperatorValues(value: string): string[] {
  return unquote(value)
    .split(",")
    .map((item) => item.trim())
    .filter(Boolean);
}

function unique(values: string[]): string[] {
  return [...new Set(values)];
}

export function parseLoomSearchOperators(query: string): ParsedLoomSearchOperators {
  const freeText: string[] = [];
  const tagIds: string[] = [];
  const mentionIds: string[] = [];
  const sourceKinds: LoomGraphSearchSourceKind[] = [];
  const errors: string[] = [];
  let path: string | undefined;

  for (const token of splitQueryTokens(query)) {
    const match = token.raw.match(OPERATOR_PATTERN);
    if (!match) {
      freeText.push(token.value);
      continue;
    }

    const operator = match[1].toLowerCase();
    const operand = match[2];
    if (operator === "tag") {
      tagIds.push(...splitOperatorValues(operand).map((value) => value.replace(/^#/, "")));
    } else if (operator === "mention") {
      mentionIds.push(...splitOperatorValues(operand));
    } else if (operator === "path" || operator === "folder") {
      path = unquote(operand).trim() || path;
    } else if (operator === "kind") {
      for (const value of splitOperatorValues(operand)) {
        if (LOOM_SEARCH_SOURCE_KIND_SET.has(value)) {
          sourceKinds.push(value as LoomGraphSearchSourceKind);
        } else {
          errors.push(`Invalid kind operator: ${value}`);
        }
      }
    } else {
      freeText.push(token.value);
    }
  }

  return {
    q: freeText.join(" ").trim(),
    tagIds: unique(tagIds),
    mentionIds: unique(mentionIds),
    sourceKinds: unique(sourceKinds) as LoomGraphSearchSourceKind[],
    path,
    errors,
  };
}
