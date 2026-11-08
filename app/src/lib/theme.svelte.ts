import { invoke } from "@tauri-apps/api/core";
import { i18n, systemLocale, type Locale } from "./i18n.svelte";

/**
 * A theme is a set of colors. Users add their own as JSON files in the
 * app's `themes/` config folder, using this exact shape.
 */
export interface Theme {
  id: string;
  name: string;
  dark: boolean;
  colors: {
    /** Main content background. */
    bg: string;
    sidebar: string;
    /** Cards and panels. */
    surface: string;
    /** Inputs, hover states. */
    surface2: string;
    /** Behind the PDF preview pages. */
    canvas: string;
    border: string;
    text: string;
    muted: string;
    accent: string;
    /** Text on top of `accent`. */
    accentText: string;
    danger: string;
    warn: string;
  };
}

export type FontChoice = "figtree" | "inter" | "system" | "serif";
export type Radius = "sharp" | "soft" | "round";

export interface Settings {
  theme: string;
  /** Overrides the theme's accent when set. */
  accent: string | null;
  font: FontChoice;
  /** UI scale, 0.85–1.2. */
  scale: number;
  radius: Radius;
  lastWorkspace: string | null;
  /** UI language; `null` follows the system. */
  locale: Locale | null;
}

export const BUILTIN_THEMES: Theme[] = [
  {
    id: "paper",
    name: "Paper",
    dark: false,
    colors: {
      bg: "#f3eee8",
      sidebar: "#ebe4dc",
      surface: "#f9f6f2",
      surface2: "#e9e2da",
      canvas: "#e6ded5",
      border: "#e0d7cd",
      text: "#2b2622",
      muted: "#8a8079",
      accent: "#e07a4f",
      accentText: "#ffffff",
      danger: "#c8473b",
      warn: "#b7791f",
    },
  },
  {
    id: "paper-night",
    name: "Paper Night",
    dark: true,
    colors: {
      bg: "#211e1b",
      sidebar: "#1a1816",
      surface: "#2a2623",
      surface2: "#35302c",
      canvas: "#161412",
      border: "#3a3430",
      text: "#ede6df",
      muted: "#9d938a",
      accent: "#ef8c61",
      accentText: "#1a1512",
      danger: "#ef6b5e",
      warn: "#e2a84a",
    },
  },
  {
    id: "sage",
    name: "Sage",
    dark: false,
    colors: {
      bg: "#eef1ea",
      sidebar: "#e3e8de",
      surface: "#f7f9f4",
      surface2: "#e1e7db",
      canvas: "#dde3d6",
      border: "#d6ddcf",
      text: "#232a22",
      muted: "#7a8574",
      accent: "#5f8f5a",
      accentText: "#ffffff",
      danger: "#c0473d",
      warn: "#a87a1f",
    },
  },
  {
    id: "slate",
    name: "Slate",
    dark: false,
    colors: {
      bg: "#f4f5f7",
      sidebar: "#eceef2",
      surface: "#ffffff",
      surface2: "#e8eaef",
      canvas: "#e2e5eb",
      border: "#dfe2e8",
      text: "#1d2230",
      muted: "#6b7285",
      accent: "#3b6fe0",
      accentText: "#ffffff",
      danger: "#d33f3f",
      warn: "#b7791f",
    },
  },
  {
    id: "midnight",
    name: "Midnight",
    dark: true,
    colors: {
      bg: "#16181d",
      sidebar: "#111317",
      surface: "#1e2128",
      surface2: "#272b34",
      canvas: "#0d0f12",
      border: "#2c313b",
      text: "#e6e9ef",
      muted: "#8b93a3",
      accent: "#7aa2f7",
      accentText: "#0e1220",
      danger: "#f7768e",
      warn: "#e0af68",
    },
  },
  {
    id: "rose",
    name: "Rosé",
    dark: false,
    colors: {
      bg: "#f7eef0",
      sidebar: "#efe3e6",
      surface: "#fcf7f8",
      surface2: "#ede0e3",
      canvas: "#e9dadd",
      border: "#e6d6da",
      text: "#2e2327",
      muted: "#8f7c82",
      accent: "#c75b7a",
      accentText: "#ffffff",
      danger: "#c8473b",
      warn: "#b7791f",
    },
  },
];

export const DEFAULT_SETTINGS: Settings = {
  theme: "paper",
  accent: null,
  font: "figtree",
  scale: 1,
  radius: "soft",
  lastWorkspace: null,
  locale: null,
};

const FONTS: Record<FontChoice, string> = {
  figtree: `"Figtree Variable", system-ui, sans-serif`,
  inter: `"Inter Variable", system-ui, sans-serif`,
  system: `system-ui, -apple-system, "Segoe UI", sans-serif`,
  serif: `"Iowan Old Style", "Palatino Linotype", Georgia, serif`,
};

const RADII: Record<Radius, number> = { sharp: 4, soft: 12, round: 18 };

class Appearance {
  settings = $state<Settings>({ ...DEFAULT_SETTINGS });
  custom = $state<Theme[]>([]);

  themes = $derived([...BUILTIN_THEMES, ...this.custom]);
  theme = $derived(this.themes.find((t) => t.id === this.settings.theme) ?? BUILTIN_THEMES[0]);
  dark = $derived(this.theme.dark);

  async load() {
    try {
      const out = await invoke<{ settings: Partial<Settings>; themes: Theme[] }>("load_settings");
      this.settings = { ...DEFAULT_SETTINGS, ...out.settings };
      this.custom = out.themes.filter(isTheme);
    } catch (e) {
      console.error("loading settings failed", e);
    }
    this.apply();
  }

  async update(patch: Partial<Settings>) {
    this.settings = { ...this.settings, ...patch };
    this.apply();
    await invoke("save_settings", { settings: $state.snapshot(this.settings) });
  }

  /** Saves the current theme with the accent override baked in, as a new custom theme. */
  async saveAsCustom(name: string) {
    const id = name.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "") || "custom";
    const base = $state.snapshot(this.theme);
    const theme: Theme = {
      ...base,
      id,
      name,
      colors: { ...base.colors, accent: this.settings.accent ?? base.colors.accent },
    };
    await invoke("save_theme", { theme });
    this.custom = [...this.custom.filter((t) => t.id !== id), theme];
    await this.update({ theme: id, accent: null });
  }

  apply() {
    const root = document.documentElement;
    const c = this.theme.colors;
    const vars: Record<string, string> = {
      "--bg": c.bg,
      "--sidebar": c.sidebar,
      "--surface": c.surface,
      "--surface-2": c.surface2,
      "--canvas": c.canvas,
      "--border": c.border,
      "--text": c.text,
      "--muted": c.muted,
      "--accent": this.settings.accent ?? c.accent,
      "--accent-text": c.accentText,
      "--danger": c.danger,
      "--warn": c.warn,
      "--font": FONTS[this.settings.font],
      "--radius": `${RADII[this.settings.radius]}px`,
      "--radius-sm": `${Math.round(RADII[this.settings.radius] * 0.6)}px`,
      "--shadow": this.theme.dark
        ? "0 1px 2px rgb(0 0 0 / 0.4), 0 4px 16px rgb(0 0 0 / 0.25)"
        : "0 1px 2px rgb(60 40 20 / 0.06), 0 4px 18px rgb(60 40 20 / 0.06)",
    };
    for (const [k, v] of Object.entries(vars)) root.style.setProperty(k, v);
    root.style.fontSize = `${14 * this.settings.scale}px`;
    root.style.colorScheme = this.theme.dark ? "dark" : "light";
    i18n.locale = this.settings.locale ?? systemLocale();
    root.lang = i18n.locale;
  }
}

function isTheme(t: unknown): t is Theme {
  const x = t as Theme;
  return typeof x?.id === "string" && typeof x?.name === "string" && typeof x?.colors?.bg === "string";
}

export const appearance = new Appearance();
