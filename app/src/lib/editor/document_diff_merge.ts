import { jsonDeepEquals } from "./doc_equality";
import { makeCodeBlockAttrs, type MonacoCodeBlockAttrs } from "./code_block_serialization";

export type RichDocumentJson = {
  type: string;
  content?: unknown[];
  [key: string]: unknown;
};

export type RichDocumentDiffStatus = "unchanged" | "modified" | "added" | "removed";
export type RichDocumentDiffKind = "prose" | "code" | "unknown";

export interface RichDocumentDiffBlock {
  blockIndex: number;
  status: RichDocumentDiffStatus;
  kind: RichDocumentDiffKind;
  leftNode?: unknown;
  rightNode?: unknown;
  leftText?: string;
  rightText?: string;
  leftCode?: MonacoCodeBlockAttrs;
  rightCode?: MonacoCodeBlockAttrs;
}

export interface RichDocumentDiff {
  blocks: RichDocumentDiffBlock[];
}

export type RichDocumentMergeStatus = "local_only" | "remote_only" | "both_same" | "conflict";
export type RichDocumentMergeChoice = "local" | "remote" | "both";

export interface RichDocumentMergeBlock {
  blockIndex: number;
  status: RichDocumentMergeStatus;
  baseNode?: unknown;
  localNode?: unknown;
  remoteNode?: unknown;
  localText?: string;
  remoteText?: string;
  localCode?: MonacoCodeBlockAttrs;
  remoteCode?: MonacoCodeBlockAttrs;
}

export interface RichDocumentMergePlan {
  base: RichDocumentJson;
  local: RichDocumentJson;
  remote: RichDocumentJson;
  blocks: RichDocumentMergeBlock[];
}

export type RichDocumentMergeChoices = Record<number, RichDocumentMergeChoice>;

export function buildRichDocumentDiff(input: {
  left: RichDocumentJson;
  right: RichDocumentJson;
}): RichDocumentDiff {
  const leftBlocks = documentBlocks(input.left);
  const rightBlocks = documentBlocks(input.right);
  const blocks = alignDiffBlocks(leftBlocks, rightBlocks).map((pair, index) =>
    buildDiffBlock(index, pair.leftNode, pair.rightNode),
  );

  return { blocks };
}

export function planRichDocumentMerge(input: {
  base: RichDocumentJson;
  local: RichDocumentJson;
  remote: RichDocumentJson;
}): RichDocumentMergePlan {
  const baseBlocks = documentBlocks(input.base);
  const localBlocks = documentBlocks(input.local);
  const remoteBlocks = documentBlocks(input.remote);
  const count = Math.max(baseBlocks.length, localBlocks.length, remoteBlocks.length);
  const blocks: RichDocumentMergeBlock[] = [];

  for (let index = 0; index < count; index += 1) {
    const baseNode = baseBlocks[index];
    const localNode = localBlocks[index];
    const remoteNode = remoteBlocks[index];
    const localChanged = !jsonDeepEquals(baseNode, localNode);
    const remoteChanged = !jsonDeepEquals(baseNode, remoteNode);

    if (!localChanged && !remoteChanged) continue;

    const status = mergeStatus(localChanged, remoteChanged, localNode, remoteNode);
    blocks.push({
      blockIndex: index,
      status,
      baseNode,
      localNode,
      remoteNode,
      localText: nodeText(localNode),
      remoteText: nodeText(remoteNode),
      localCode: codeAttrs(localNode),
      remoteCode: codeAttrs(remoteNode),
    });
  }

  return { base: input.base, local: input.local, remote: input.remote, blocks };
}

export function applyRichDocumentMergePlan(
  plan: RichDocumentMergePlan,
  choices: RichDocumentMergeChoices,
): RichDocumentJson {
  const localBlocks = documentBlocks(plan.local);
  const remoteBlocks = documentBlocks(plan.remote);
  const max = Math.max(documentBlocks(plan.base).length, localBlocks.length, remoteBlocks.length);
  const byIndex = new Map(plan.blocks.map((block) => [block.blockIndex, block]));
  const merged: unknown[] = [];

  for (let index = 0; index < max; index += 1) {
    const block = byIndex.get(index);
    const selected = block ? selectedMergeNode(block, choices) : localBlocks[index] ?? remoteBlocks[index];
    if (Array.isArray(selected)) {
      merged.push(...selected.filter((node) => node !== undefined));
    } else if (selected !== undefined) {
      merged.push(selected);
    }
  }

  return { ...plan.local, content: merged };
}

function documentBlocks(doc: RichDocumentJson): unknown[] {
  return Array.isArray(doc.content) ? doc.content : [];
}

function alignDiffBlocks(leftBlocks: unknown[], rightBlocks: unknown[]): { leftNode?: unknown; rightNode?: unknown }[] {
  const matches = longestCommonBlockSubsequence(leftBlocks, rightBlocks);
  const aligned: { leftNode?: unknown; rightNode?: unknown }[] = [];
  let leftCursor = 0;
  let rightCursor = 0;

  for (const match of matches) {
    aligned.push(...alignGap(leftBlocks, rightBlocks, leftCursor, match.leftIndex, rightCursor, match.rightIndex));
    aligned.push({ leftNode: leftBlocks[match.leftIndex], rightNode: rightBlocks[match.rightIndex] });
    leftCursor = match.leftIndex + 1;
    rightCursor = match.rightIndex + 1;
  }

  aligned.push(...alignGap(leftBlocks, rightBlocks, leftCursor, leftBlocks.length, rightCursor, rightBlocks.length));
  return aligned;
}

function alignGap(
  leftBlocks: unknown[],
  rightBlocks: unknown[],
  leftFrom: number,
  leftTo: number,
  rightFrom: number,
  rightTo: number,
): { leftNode?: unknown; rightNode?: unknown }[] {
  const aligned: { leftNode?: unknown; rightNode?: unknown }[] = [];
  const paired = Math.min(leftTo - leftFrom, rightTo - rightFrom);
  for (let offset = 0; offset < paired; offset += 1) {
    aligned.push({ leftNode: leftBlocks[leftFrom + offset], rightNode: rightBlocks[rightFrom + offset] });
  }
  for (let index = leftFrom + paired; index < leftTo; index += 1) {
    aligned.push({ leftNode: leftBlocks[index] });
  }
  for (let index = rightFrom + paired; index < rightTo; index += 1) {
    aligned.push({ rightNode: rightBlocks[index] });
  }
  return aligned;
}

function longestCommonBlockSubsequence(
  leftBlocks: unknown[],
  rightBlocks: unknown[],
): { leftIndex: number; rightIndex: number }[] {
  const dp = Array.from({ length: leftBlocks.length + 1 }, () => Array<number>(rightBlocks.length + 1).fill(0));
  for (let left = leftBlocks.length - 1; left >= 0; left -= 1) {
    for (let right = rightBlocks.length - 1; right >= 0; right -= 1) {
      dp[left][right] = jsonDeepEquals(leftBlocks[left], rightBlocks[right])
        ? dp[left + 1][right + 1] + 1
        : Math.max(dp[left + 1][right], dp[left][right + 1]);
    }
  }

  const matches: { leftIndex: number; rightIndex: number }[] = [];
  let left = 0;
  let right = 0;
  while (left < leftBlocks.length && right < rightBlocks.length) {
    if (jsonDeepEquals(leftBlocks[left], rightBlocks[right])) {
      matches.push({ leftIndex: left, rightIndex: right });
      left += 1;
      right += 1;
    } else if (dp[left + 1][right] >= dp[left][right + 1]) {
      left += 1;
    } else {
      right += 1;
    }
  }
  return matches;
}

function buildDiffBlock(index: number, leftNode: unknown, rightNode: unknown): RichDocumentDiffBlock {
  const status = diffStatus(leftNode, rightNode);
  return {
    blockIndex: index,
    status,
    kind: diffKind(leftNode, rightNode),
    leftNode,
    rightNode,
    leftText: nodeText(leftNode),
    rightText: nodeText(rightNode),
    leftCode: codeAttrs(leftNode),
    rightCode: codeAttrs(rightNode),
  };
}

function diffStatus(leftNode: unknown, rightNode: unknown): RichDocumentDiffStatus {
  if (leftNode === undefined) return "added";
  if (rightNode === undefined) return "removed";
  return jsonDeepEquals(leftNode, rightNode) ? "unchanged" : "modified";
}

function mergeStatus(
  localChanged: boolean,
  remoteChanged: boolean,
  localNode: unknown,
  remoteNode: unknown,
): RichDocumentMergeStatus {
  if (localChanged && remoteChanged) {
    return jsonDeepEquals(localNode, remoteNode) ? "both_same" : "conflict";
  }
  return localChanged ? "local_only" : "remote_only";
}

function selectedMergeNode(block: RichDocumentMergeBlock, choices: RichDocumentMergeChoices): unknown | unknown[] {
  if (block.status === "local_only" || block.status === "both_same") return block.localNode;
  if (block.status === "remote_only") return block.remoteNode;

  const choice = choices[block.blockIndex];
  if (!choice) {
    throw new Error(`Unresolved conflict at block ${block.blockIndex}`);
  }
  if (choice === "local") return block.localNode;
  if (choice === "remote") return block.remoteNode;
  return [block.localNode, block.remoteNode];
}

function diffKind(leftNode: unknown, rightNode: unknown): RichDocumentDiffKind {
  if (isCodeBlock(leftNode) || isCodeBlock(rightNode)) return "code";
  if (isProseBlock(leftNode) || isProseBlock(rightNode)) return "prose";
  return "unknown";
}

function isCodeBlock(node: unknown): boolean {
  return isRecord(node) && node.type === "monacoCodeBlock";
}

function isProseBlock(node: unknown): boolean {
  return isRecord(node) && typeof node.type === "string" && node.type !== "monacoCodeBlock";
}

function codeAttrs(node: unknown): MonacoCodeBlockAttrs | undefined {
  if (!isCodeBlock(node) || !isRecord(node)) return undefined;
  const attrs = isRecord(node.attrs) ? node.attrs : {};
  return makeCodeBlockAttrs(String(attrs.language ?? ""), String(attrs.code ?? ""));
}

function nodeText(node: unknown): string | undefined {
  if (node === undefined) return undefined;
  if (!isRecord(node)) return "";
  if (typeof node.text === "string") return node.text;
  if (isCodeBlock(node)) return codeAttrs(node)?.code ?? "";
  const children = node.content;
  if (!Array.isArray(children)) return "";
  return children.map((child) => nodeText(child) ?? "").join("");
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}
