export function splitFenceAwareLines(input = "") {
  const lines = String(input || "").split("\n");
  let inFence = false;
  return lines.map((line) => {
    const fenceLine = /^\s*```/.test(line);
    const entry = { line, inFence, fenceLine };
    if (fenceLine) inFence = !inFence;
    return entry;
  });
}

export function transformOutsideFences(input = "", transformLine) {
  const output = splitFenceAwareLines(input).map((entry) => {
    if (entry.inFence || entry.fenceLine) return entry.line;
    return transformLine(entry.line);
  }).join("\n");
  return output;
}

export function resultFor(name, input, output, reason = "") {
  return {
    output,
    applied: output !== String(input || ""),
    reason: output !== String(input || "") ? (reason || name) : "",
  };
}

