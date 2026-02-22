import { invoke } from "@tauri-apps/api/core";

export type MdSessionRecordV0 = {
  session_id: string;
  kind: string;
  label: string;
  created_at: string;
  last_used_at?: string | null;
  allow_private_network: boolean;
  cookie_jar_artifact_ref?: unknown | null;
};

export async function mdOutputRootDirGet(): Promise<string> {
  return (await invoke("md_output_root_dir_get")) as string;
}

export async function mdOutputRootDirSet(outputRootDir: string): Promise<void> {
  await invoke("md_output_root_dir_set", { output_root_dir: outputRootDir });
}

export async function mdSessionsList(): Promise<MdSessionRecordV0[]> {
  return (await invoke("md_sessions_list")) as MdSessionRecordV0[];
}

export async function mdSessionCreate(label: string): Promise<MdSessionRecordV0> {
  return (await invoke("md_session_create", { label })) as MdSessionRecordV0;
}

export async function mdSessionOpen(sessionId: string, startUrl: string): Promise<void> {
  await invoke("md_session_open", { session_id: sessionId, start_url: startUrl });
}

export async function mdSessionExportCookies(sessionId: string): Promise<string> {
  return (await invoke("md_session_export_cookies", { session_id: sessionId })) as string;
}

