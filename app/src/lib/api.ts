import { Channel, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type Json = null | boolean | number | string | Json[] | { [key: string]: Json };

export interface TemplateInfo {
  id: string;
  name: string;
  description: string;
  resumes: string[];
}

export interface ResumeInfo {
  id: string;
  template: string;
  path: string;
}

export interface Summary {
  name: string;
  root: string;
  templates: TemplateInfo[];
  resumes: ResumeInfo[];
}

export interface Diagnostic {
  severity: "error" | "warning";
  message: string;
  hints: string[];
  file: string | null;
  line: number | null;
  column: number | null;
}

export interface SchemaError {
  path: string;
  message: string;
}

/** Which profile fields each template shows (hidden = left out of that template). */
export interface ProfileVisibility {
  fields: string[];
  templates: { id: string; name: string; hidden: string[] }[];
}

export interface Preview {
  svg: string[];
  diagnostics: Diagnostic[];
}

/** AI agents installed on this machine (Claude Code, Codex, …). */
export type AgentKind = "claude" | "codex" | "gemini" | "opencode";
export type PermissionMode = "ask" | "autoEdit" | "readOnly";

export interface AgentInfo {
  kind: AgentKind;
  name: string;
  path: string | null;
  version: string | null;
}

export type ToolStatus = "running" | "done" | "failed";

/** A model an agent can run, with the reasoning efforts it accepts. */
export interface ModelOption {
  id: string;
  name: string;
  efforts: string[];
  default_effort: string | null;
}

/** A file the agent created, edited or deleted since the last keep/undo. */
export interface FileChange {
  path: string;
  before: string | null;
  after: string | null;
  diff: string;
}

/** Normalized events from any agent (see crates/agent). */
export type AgentEvent =
  | { type: "ready" }
  | { type: "text"; text: string }
  | { type: "tool"; id: string; title: string; status: ToolStatus }
  | { type: "permission"; id: string; title: string; detail: string | null }
  | { type: "turn_end"; error: string | null }
  | { type: "error"; message: string }
  | { type: "exited" };

export const api = {
  openWorkspace: (path: string, create = false) =>
    invoke<Summary>("open_workspace", { path, create }),
  summary: () => invoke<Summary>("workspace_summary"),
  getResume: (id: string) => invoke<Record<string, Json>>("get_resume", { id }),
  getSchema: (template: string) => invoke<Schema>("get_schema", { template }),
  getProfile: () => invoke<string>("get_profile"),
  saveProfile: (text: string) => invoke<void>("save_profile", { text }),
  profileVisibility: () => invoke<ProfileVisibility>("profile_visibility"),
  setProfileHidden: (template: string, hidden: string[]) =>
    invoke<void>("set_profile_hidden", { template, hidden }),
  saveResume: (id: string, value: Json) => invoke<SchemaError[]>("save_resume", { id, value }),
  newResume: (id: string, template: string) => invoke<ResumeInfo>("new_resume", { id, template }),
  duplicateResume: (id: string, from: string) => invoke<ResumeInfo>("duplicate_resume", { id, from }),
  deleteResume: (id: string) => invoke<void>("delete_resume", { id }),
  previewResume: (id: string, value: Json) => invoke<Preview>("preview_resume", { id, value }),
  exportPdf: (id: string, value: Json, path: string) =>
    invoke<Diagnostic[]>("export_pdf", { id, value, path }),
  newTemplate: (id: string, name: string, from: string | null) =>
    invoke<TemplateInfo>("new_template", { id, name, from }),
  templateFiles: (template: string) => invoke<string[]>("template_files", { template }),
  readTemplateFile: (template: string, name: string) =>
    invoke<string>("read_template_file", { template, name }),
  writeTemplateFile: (template: string, name: string, text: string) =>
    invoke<void>("write_template_file", { template, name, text }),
  previewTemplate: (template: string, files: Record<string, string>) =>
    invoke<Preview>("preview_template", { template, files }),

  listAgents: () => invoke<AgentInfo[]>("list_agents"),
  /** Starts an agent in the workspace; returns the session id. */
  agentModels: (kind: AgentKind) => invoke<ModelOption[]>("agent_models", { kind }),
  agentStart: (
    kind: AgentKind,
    mode: PermissionMode,
    model: string | null,
    effort: string | null,
    resume: string | null,
    onEvent: (e: AgentEvent) => void,
  ) => {
    const channel = new Channel<AgentEvent>();
    channel.onmessage = onEvent;
    return invoke<number>("agent_start", { kind, mode, model, effort, resume, onEvent: channel });
  },
  agentSend: (id: number, text: string) => invoke<void>("agent_send", { id, text }),
  agentRespond: (id: number, request: string, allow: boolean) =>
    invoke<void>("agent_respond", { id, request, allow }),
  agentCancel: (id: number) => invoke<void>("agent_cancel", { id }),
  agentStop: (id: number) => invoke<void>("agent_stop", { id }),
  agentChanges: (id: number) => invoke<FileChange[]>("agent_changes", { id }),
  agentKeep: (id: number) => invoke<void>("agent_keep", { id }),
  agentUndo: (id: number) => invoke<string[]>("agent_undo", { id }),
  loadChat: <T>(key: string) => invoke<T[]>("load_chat", { key }),
  saveChat: <T>(key: string, messages: T[]) => invoke<void>("save_chat", { key, messages }),
  /** Files changed on disk (workspace-relative), from the app or anything else. Returns unlisten. */
  onWorkspaceChanged: (fn: (paths: string[]) => void) =>
    listen<{ paths: string[] }>("workspace-changed", (e) => fn(e.payload.paths)),
};

/** The subset of JSON Schema that the form generator understands. */
export interface Schema {
  type?: string | string[];
  title?: string;
  description?: string;
  format?: string;
  enum?: string[];
  properties?: Record<string, Schema>;
  required?: string[];
  items?: Schema;
}

/** Calls `fn` after `ms` of inactivity. */
export function debounce<A extends unknown[]>(fn: (...args: A) => void, ms: number) {
  let timer: ReturnType<typeof setTimeout> | undefined;
  return (...args: A) => {
    clearTimeout(timer);
    timer = setTimeout(() => fn(...args), ms);
  };
}

/** Turns a display name into a file-safe id: "Back-end (EN)" -> "back-end-en". */
export const slug = (s: string) =>
  s.normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "");
