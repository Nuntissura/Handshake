export const HEURISTIC_RISK_CLASS_VALUES = [
  "NONE",
  "FUZZY_DISCRIMINATOR",
  "THRESHOLD_TUNING",
  "ADVERSARIAL_INPUT",
  "NATURAL_LANGUAGE_CLASSIFICATION",
  "PROBABILISTIC_SCORING",
  "SECRET_OR_IDENTIFIER_BOUNDARY",
  "OTHER",
];

export const HEURISTIC_REQUIRED_EVIDENCE_VALUES = [
  "CORPUS_CASES",
  "PROPERTY_TESTS",
  "NEGATIVE_COUNTEREXAMPLES",
  "BOUNDARY_MATRIX",
  "THRESHOLD_RATIONALE",
  "FALSE_POSITIVE_FALSE_NEGATIVE_PROBES",
  "ADVERSARIAL_PROBES",
];

export const HEURISTIC_STRATEGY_ESCALATION_VALUES = [
  "NONE",
  "CORPUS_EXPANSION",
  "PROPERTY_BASED_REFRAME",
  "DISCRIMINATOR_REDESIGN",
  "BOUNDARY_CONTRACT_REWRITE",
  "ALTERNATE_MODEL_REVIEW",
  "HUMAN_STOP",
];

export const HEURISTIC_RISK_REPAIR_ESCALATION_THRESHOLD = 2;

const RULES = [
  {
    riskClass: "SECRET_OR_IDENTIFIER_BOUNDARY",
    patterns: [
      /\bredact(?:ion|ed|s)?\b/i,
      /\bsecret(?:s)?\b/i,
      /\bcredential(?:s)?\b/i,
      /\btoken(?:s)?\b/i,
      /\bbase64\b/i,
      /\bentropy\b/i,
      /\bidentifier(?:s)?\b/i,
      /\bfalse\s+positive\b/i,
      /\bfalse\s+negative\b/i,
    ],
    requiredEvidence: [
      "CORPUS_CASES",
      "NEGATIVE_COUNTEREXAMPLES",
      "BOUNDARY_MATRIX",
      "FALSE_POSITIVE_FALSE_NEGATIVE_PROBES",
    ],
    strategyEscalation: "DISCRIMINATOR_REDESIGN",
  },
  {
    riskClass: "THRESHOLD_TUNING",
    patterns: [
      /\bthreshold(?:s)?\b/i,
      /\bscore(?:s|d|r)?\b/i,
      /\bratio(?:s)?\b/i,
      /\bweight(?:s|ed|ing)?\b/i,
      /\btun(?:e|ed|ing)\b/i,
    ],
    requiredEvidence: [
      "CORPUS_CASES",
      "NEGATIVE_COUNTEREXAMPLES",
      "THRESHOLD_RATIONALE",
      "FALSE_POSITIVE_FALSE_NEGATIVE_PROBES",
    ],
    strategyEscalation: "DISCRIMINATOR_REDESIGN",
  },
  {
    riskClass: "FUZZY_DISCRIMINATOR",
    patterns: [
      /\bfuzzy\b/i,
      /\bheuristic(?:s)?\b/i,
      /\bclassifier(?:s)?\b/i,
      /\bdiscriminator(?:s)?\b/i,
      /\bambiguous\b/i,
      /\bplausible\b/i,
      /\bcounterexample(?:s)?\b/i,
    ],
    requiredEvidence: [
      "CORPUS_CASES",
      "PROPERTY_TESTS",
      "NEGATIVE_COUNTEREXAMPLES",
      "BOUNDARY_MATRIX",
    ],
    strategyEscalation: "DISCRIMINATOR_REDESIGN",
  },
  {
    riskClass: "ADVERSARIAL_INPUT",
    patterns: [
      /\badversarial\b/i,
      /\bevasion\b/i,
      /\bmalformed\b/i,
      /\bpathological\b/i,
      /\battack\b/i,
    ],
    requiredEvidence: [
      "CORPUS_CASES",
      "PROPERTY_TESTS",
      "NEGATIVE_COUNTEREXAMPLES",
      "ADVERSARIAL_PROBES",
    ],
    strategyEscalation: "PROPERTY_BASED_REFRAME",
  },
  {
    riskClass: "NATURAL_LANGUAGE_CLASSIFICATION",
    patterns: [
      /\bnatural\s+language\b/i,
      /\bprose\s+(shape|classification|classifier|discriminator)\b/i,
      /\bsemantic\s+(classifier|classification|discriminator)\b/i,
      /\bmeaning\s+(classifier|classification|discriminator)\b/i,
    ],
    requiredEvidence: [
      "CORPUS_CASES",
      "NEGATIVE_COUNTEREXAMPLES",
      "BOUNDARY_MATRIX",
    ],
    strategyEscalation: "BOUNDARY_CONTRACT_REWRITE",
  },
  {
    riskClass: "PROBABILISTIC_SCORING",
    patterns: [
      /\bprobabilistic\b/i,
      /\bconfidence\b/i,
      /\branking\b/i,
      /\bsimilarity\b/i,
      /\bdistance\b/i,
    ],
    requiredEvidence: [
      "CORPUS_CASES",
      "PROPERTY_TESTS",
      "NEGATIVE_COUNTEREXAMPLES",
      "THRESHOLD_RATIONALE",
    ],
    strategyEscalation: "ALTERNATE_MODEL_REVIEW",
  },
];

function unique(values = []) {
  return [...new Set(values.map((value) => String(value || "").trim()).filter(Boolean))];
}

function parseSingleField(text, label) {
  const re = new RegExp(`^\\s*-\\s*(?:\\*\\*)?${label}(?:\\*\\*)?\\s*:\\s*(.+)\\s*$`, "mi");
  const match = String(text || "").match(re);
  return match ? match[1].trim() : "";
}

function parseListValue(value) {
  return unique(String(value || "")
    .split(/[;,|]/)
    .map((entry) => entry.trim())
    .filter(Boolean));
}

function normalizeEnum(value, allowedValues, fallback) {
  const normalized = String(value || "").trim().toUpperCase().replace(/[\s-]+/g, "_");
  return allowedValues.includes(normalized) ? normalized : fallback;
}

function normalizeYesNo(value, fallback = "") {
  const normalized = String(value || "").trim().toUpperCase();
  if (["YES", "TRUE", "Y", "1", "HEURISTIC"].includes(normalized)) return "YES";
  if (["NO", "FALSE", "N", "0", "NONE"].includes(normalized)) return "NO";
  return fallback;
}

function matchRules(text) {
  const source = String(text || "")
    .split(/\r?\n/)
    .filter((line) => !/^\s*-\s*HEURISTIC_/i.test(line))
    .join("\n");
  return RULES.map((rule) => ({
    ...rule,
    hits: rule.patterns
      .filter((pattern) => pattern.test(source))
      .map((pattern) => pattern.source),
  })).filter((rule) => rule.hits.length > 0);
}

function defaultEvidenceForClass(riskClass) {
  const rule = RULES.find((entry) => entry.riskClass === riskClass);
  return rule ? rule.requiredEvidence : ["NEGATIVE_COUNTEREXAMPLES", "BOUNDARY_MATRIX"];
}

function defaultStrategyForClass(riskClass) {
  const rule = RULES.find((entry) => entry.riskClass === riskClass);
  return rule ? rule.strategyEscalation : "DISCRIMINATOR_REDESIGN";
}

export function classifyHeuristicRiskText(text = "", explicitFields = {}) {
  const explicitRisk = normalizeYesNo(
    explicitFields.heuristicRisk
      ?? parseSingleField(text, "HEURISTIC_RISK"),
  );
  const explicitClass = normalizeEnum(
    explicitFields.heuristicRiskClass
      ?? parseSingleField(text, "HEURISTIC_RISK_CLASS"),
    HEURISTIC_RISK_CLASS_VALUES,
    "",
  );
  const explicitEvidence = parseListValue(
    explicitFields.requiredEvidence
      ?? parseSingleField(text, "HEURISTIC_REQUIRED_EVIDENCE"),
  ).map((entry) => normalizeEnum(entry, HEURISTIC_REQUIRED_EVIDENCE_VALUES, ""))
    .filter(Boolean);
  const explicitStrategy = normalizeEnum(
    explicitFields.strategyEscalation
      ?? parseSingleField(text, "HEURISTIC_STRATEGY_ESCALATION"),
    HEURISTIC_STRATEGY_ESCALATION_VALUES,
    "",
  );

  const matches = matchRules(text);
  const heuristicRisk = explicitRisk === "YES" || (explicitRisk !== "NO" && matches.length > 0);
  if (!heuristicRisk) {
    return {
      heuristic_risk: "NO",
      heuristic_risk_class: "NONE",
      required_evidence: [],
      strategy_escalation: "NONE",
      repair_cycle_strategy_threshold: 0,
      reasons: explicitRisk === "NO" ? ["explicit HEURISTIC_RISK=NO"] : [],
      summary: "HEURISTIC_RISK=NO",
    };
  }

  const selectedClass = explicitClass && explicitClass !== "NONE"
    ? explicitClass
    : matches[0]?.riskClass || "OTHER";
  const evidence = unique([
    ...matches.flatMap((entry) => entry.requiredEvidence),
    ...defaultEvidenceForClass(selectedClass),
    ...explicitEvidence,
  ]).filter((entry) => HEURISTIC_REQUIRED_EVIDENCE_VALUES.includes(entry));
  const strategy = explicitStrategy && explicitStrategy !== "NONE"
    ? explicitStrategy
    : defaultStrategyForClass(selectedClass);
  const reasons = unique([
    explicitRisk === "YES" ? "explicit HEURISTIC_RISK=YES" : "",
    ...matches.map((entry) => `matched ${entry.riskClass}`),
  ]);

  return {
    heuristic_risk: "YES",
    heuristic_risk_class: selectedClass,
    required_evidence: evidence,
    strategy_escalation: strategy,
    repair_cycle_strategy_threshold: HEURISTIC_RISK_REPAIR_ESCALATION_THRESHOLD,
    reasons,
    summary: `HEURISTIC_RISK=YES class=${selectedClass} strategy=${strategy}`,
  };
}

export function heuristicRiskContractFields(classification = {}) {
  if (String(classification?.heuristic_risk || "").toUpperCase() !== "YES") return {};
  return {
    heuristic_risk: "YES",
    heuristic_risk_class: normalizeEnum(
      classification.heuristic_risk_class,
      HEURISTIC_RISK_CLASS_VALUES,
      "OTHER",
    ),
    required_evidence: Array.isArray(classification.required_evidence)
      ? classification.required_evidence.filter((entry) => HEURISTIC_REQUIRED_EVIDENCE_VALUES.includes(entry))
      : [],
    strategy_escalation: normalizeEnum(
      classification.strategy_escalation,
      HEURISTIC_STRATEGY_ESCALATION_VALUES,
      "DISCRIMINATOR_REDESIGN",
    ),
    repair_cycle_strategy_threshold: Number.isInteger(classification.repair_cycle_strategy_threshold)
      && classification.repair_cycle_strategy_threshold > 0
      ? classification.repair_cycle_strategy_threshold
      : HEURISTIC_RISK_REPAIR_ESCALATION_THRESHOLD,
  };
}

export function mergeHeuristicRiskContract(microtaskContract = {}, classification = {}) {
  const fields = heuristicRiskContractFields(classification);
  if (Object.keys(fields).length === 0) return microtaskContract;
  return {
    ...microtaskContract,
    ...fields,
    required_evidence: unique([
      ...(Array.isArray(microtaskContract.required_evidence) ? microtaskContract.required_evidence : []),
      ...fields.required_evidence,
    ]),
  };
}

export function isHeuristicRiskContract(value = {}) {
  return String(value?.heuristic_risk || "").trim().toUpperCase() === "YES";
}

export function summarizeHeuristicRiskContract(value = {}) {
  if (!isHeuristicRiskContract(value)) return "HEURISTIC_RISK=NO";
  const evidence = Array.isArray(value.required_evidence) && value.required_evidence.length > 0
    ? value.required_evidence.join(",")
    : "UNSPECIFIED";
  return [
    `HEURISTIC_RISK=YES`,
    `class=${value.heuristic_risk_class || "OTHER"}`,
    `evidence=${evidence}`,
    `strategy=${value.strategy_escalation || "DISCRIMINATOR_REDESIGN"}`,
    `threshold=${value.repair_cycle_strategy_threshold || HEURISTIC_RISK_REPAIR_ESCALATION_THRESHOLD}`,
  ].join(" ");
}
