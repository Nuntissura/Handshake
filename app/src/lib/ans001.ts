export type WorkMode = "Strict" | "Free" | "Fasttrack" | "Brainstorm" | "Data";

export type Layer = "L1" | "L2" | "L3" | "L4";

export type Answer = {
  content: string;
  addresses_all_questions: boolean;
  clarifying_question?: string | null;
};

export type ScopeBoundary = {
  included: string[];
  excluded: string[];
};

export type Assumption = {
  what: string;
  why: string;
  impact_if_wrong: string;
};

export type IntentConfirmation = {
  understood_request: string;
  scope: ScopeBoundary;
  assumptions: Assumption[];
};

export type ModeContext = {
  mode: WorkMode;
  determinism: boolean;
  can_edit: boolean;
  layer: Layer;
};

export type ChangeType = "Create" | "Modify" | "Delete" | "Reference";

export type PlannedOperation = {
  operation_type: unknown;
  target: string;
  description: string;
  reversible: boolean;
};

export type AffectedEntity = {
  entity_id: string;
  entity_type: string;
  change_type: ChangeType;
};

export type OperationPlan = {
  operations: PlannedOperation[];
  affected: AffectedEntity[];
  validation: unknown;
  abortable: boolean;
};

export type Alternative = {
  description: string;
  why_better: string;
  tradeoffs: string[];
  effort_delta: "Less" | "Same" | "More" | "Unknown";
};

export type Conflict = {
  description: string;
  between: [string, string];
  resolution_options: string[];
  recommended?: string | null;
};

export type Risk = {
  description: string;
  severity: unknown;
  mitigation?: string | null;
  affects: string[];
};

export type Finding = {
  what: string;
  significance: unknown;
  action_needed: boolean;
};

export type ProactiveSurfacing = {
  risks: Risk[];
  conflicts: Conflict[];
  alternatives: Alternative[];
  findings: Finding[];
};

export type Actor = "User" | "Assistant" | "Either" | { External: string };

export type NextAction = {
  description: string;
  who: Actor;
  urgency: unknown;
};

export type NextSteps = {
  immediate: NextAction[];
  future: string[];
  blockers: string[];
};

export type ResponseBehaviorContract = {
  answer: Answer;
  intent: IntentConfirmation;
  mode_context: ModeContext;
  operation_plan?: OperationPlan | null;
  proactive: ProactiveSurfacing;
  next_steps?: NextSteps | null;
};

