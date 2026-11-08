import { invoke } from "@tauri-apps/api/core";

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
