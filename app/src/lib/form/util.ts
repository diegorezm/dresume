import type { Json, Schema } from "../api";

export type Obj = Record<string, Json>;

export const typeOf = (s: Schema) => (Array.isArray(s.type) ? s.type[0] : s.type);

export function emptyFor(s: Schema): Json {
  const t = typeOf(s);
  if (t === "object") {
    // Pre-fill required fields so a new item shows its inputs right away.
    const out: Obj = {};
    for (const key of s.required ?? []) out[key] = emptyFor(s.properties?.[key] ?? {});
    return out;
  }
  if (t === "array") return [];
  if (t === "number" || t === "integer") return 0;
  if (t === "boolean") return false;
  return "";
}

export const humanize = (key: string) => key.replace(/[_-]+/g, " ").replace(/^\w/, (c) => c.toUpperCase());

/** "Experience" -> "experience", "Projects" -> "project"; `items.title` wins when set. */
export function singular(s: Schema, key: string) {
  if (s.items?.title) return s.items.title.toLowerCase();
  const word = (s.title ?? humanize(key)).toLowerCase();
  if (word.endsWith("ies")) return word.slice(0, -3) + "y";
  if (word.endsWith("s") && !word.endsWith("ss")) return word.slice(0, -1);
  return word;
}

/** How to edit an array of strings: chips for short values, a bullet list otherwise. */
export function stringListStyle(s: Schema): "tags" | "bullets" {
  return s.format === "tags" ? "tags" : "bullets";
}

const DATE_KEYS = /^(period|date|dates|year|years|when|duration)$/i;

/** Card heading for an object item: its first two text fields, plus a date-like field. */
export function itemSummary(item: Json, s: Schema): { title: string; meta: string } {
  const obj = (item ?? {}) as Obj;
  const keys = Object.keys(s.properties ?? {});
  const text = (k: string) => (typeof obj[k] === "string" || typeof obj[k] === "number" ? String(obj[k]).trim() : "");
  const dateKey = keys.find((k) => DATE_KEYS.test(k));
  const parts = keys
    .filter((k) => k !== dateKey && typeOf(s.properties![k]) !== "array" && s.properties![k].format !== "textarea")
    .map(text)
    .filter(Boolean)
    .slice(0, 2);
  return { title: parts.join(" — "), meta: dateKey ? text(dateKey) : "" };
}
