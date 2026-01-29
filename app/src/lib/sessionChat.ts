import { invoke } from "@tauri-apps/api/core";
import { ResponseBehaviorContract } from "./ans001";

export type SessionChatRole = "user" | "assistant";

export type Ans001ValidationRecordV0_1 = {
  compliant: boolean;
  violation_clauses: string[];
};

export type SessionChatLogEntryV0_1 = {
  schema_version: "hsk.session_chat_log@0.1";

  session_id: string;
  turn_index: number;
  created_at_utc: string;
  message_id: string;

  role: SessionChatRole;
  model_role?: "frontend" | null;

  content: string;

  ans001?: ResponseBehaviorContract | null;
  ans001_validation?: Ans001ValidationRecordV0_1 | null;
};

export type SessionChatLogEntryV0_1Input = {
  role: SessionChatRole;
  content: string;
  model_role?: "frontend" | null;
  ans001?: ResponseBehaviorContract | null;
  ans001_validation?: Ans001ValidationRecordV0_1 | null;
  message_id?: string;
};

export async function sessionChatGetSessionId(): Promise<string> {
  return (await invoke("session_chat_get_session_id")) as string;
}

export async function sessionChatAppend(entry: SessionChatLogEntryV0_1Input): Promise<void> {
  await invoke("session_chat_append", { entry });
}

export async function sessionChatRead(sessionId: string, limit?: number): Promise<SessionChatLogEntryV0_1[]> {
  return (await invoke("session_chat_read", { session_id: sessionId, limit })) as SessionChatLogEntryV0_1[];
}

