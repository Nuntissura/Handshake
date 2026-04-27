const TRUST_LEVELS = new Set(["informational", "required", "advisory"]);

function normalizeNonEmpty(value, fieldName) {
  const normalized = String(value ?? "").trim();
  if (!normalized) {
    throw new Error(`buildEphemeralContextBlock: ${fieldName} is required`);
  }
  return normalized;
}

function escapeXmlAttribute(value) {
  return String(value ?? "")
    .replace(/&/g, "&amp;")
    .replace(/"/g, "&quot;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

export function normalizeEphemeralTrust(value = "informational") {
  const normalized = String(value || "informational").trim().toLowerCase();
  if (!TRUST_LEVELS.has(normalized)) {
    throw new Error(`buildEphemeralContextBlock: unknown trust level ${value}`);
  }
  return normalized;
}

export function buildEphemeralContextBlock({ source, trust = "informational", body } = {}) {
  const normalizedSource = normalizeNonEmpty(source, "source");
  const normalizedTrust = normalizeEphemeralTrust(trust);
  const normalizedBody = normalizeNonEmpty(body, "body");

  return [
    `[INFORMATIONAL - not user input. Source: ${normalizedSource}. Trust: ${normalizedTrust}.]`,
    `<governance-context source="${escapeXmlAttribute(normalizedSource)}" trust="${normalizedTrust}">`,
    normalizedBody,
    `</governance-context>`,
  ].join("\n");
}
