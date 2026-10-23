import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";

export type SettingsCategory =
  | "welcome"
  | "general"
  | "appearance"
  | "search"
  | "clipboard"
  | "features"
  | "help"
  | "about";

export type InformationDensity = "minimal" | "normal" | "detailed";
export type LayoutDensity = "compact" | "normal" | "comfortable";

export interface SettingsState {
  theme: "system" | "light" | "dark";
  layout: "list" | "grid";
  informationDensity: InformationDensity;
  layoutDensity: LayoutDensity;
  preview: boolean;
  login: boolean;
  progressBar: boolean;
  caret: boolean;
  fileSearch: boolean;
  rememberSearch: boolean;
  resultPreview: boolean;
  clipboardHistory: boolean;
  flowPrefixCompatibility: boolean;
  flowPrefixes: string[];
  inlineAutocomplete: boolean;
  showFrequent: boolean;
}

export const DEFAULT_SETTINGS: SettingsState = {
  theme: "system",
  layout: "list",
  informationDensity: "normal",
  layoutDensity: "normal",
  preview: true,
  login: false,
  progressBar: true,
  caret: true,
  fileSearch: true,
  rememberSearch: false,
  resultPreview: false,
  clipboardHistory: true,
  flowPrefixCompatibility: true,
  flowPrefixes: ["et"],
  inlineAutocomplete: true,
  showFrequent: true,
};

export const SETTINGS_STORAGE_KEY = "lux-settings";
export const LAST_SEARCH_STORAGE_KEY = "lux-last-search";
export const EVENT_SETTINGS_CHANGED = "lux://settings-changed";

export function loadSettings(): SettingsState {
  if (typeof localStorage === "undefined") {
    return { ...DEFAULT_SETTINGS };
  }
  try {
    const raw = localStorage.getItem(SETTINGS_STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      // Backward compatibility mapping for old interfaceStyle
      if (parsed.interfaceStyle && !parsed.informationDensity) {
        parsed.informationDensity = parsed.interfaceStyle;
      }
      if (parsed.interfaceStyle && !parsed.layoutDensity) {
        parsed.layoutDensity =
          parsed.interfaceStyle === "minimal"
            ? "compact"
            : parsed.interfaceStyle === "detailed"
              ? "comfortable"
              : "normal";
      }
      return { ...DEFAULT_SETTINGS, ...parsed };
    }
  } catch {
    // Ignore JSON parse errors and return defaults
  }
  return { ...DEFAULT_SETTINGS };
}

export function saveSettings(settings: SettingsState): void {
  if (typeof localStorage === "undefined") return;
  try {
    localStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(settings));
  } catch {
    // Ignore storage quota or disabled storage errors
  }
  void emit(EVENT_SETTINGS_CHANGED, settings).catch(() => {});
}

export function loadLastSearch(): string {
  if (typeof localStorage === "undefined") return "";
  try {
    return localStorage.getItem(LAST_SEARCH_STORAGE_KEY) ?? "";
  } catch {
    return "";
  }
}

export function saveLastSearch(query: string): void {
  if (typeof localStorage === "undefined") return;
  try {
    if (query.trim().length === 0) {
      localStorage.removeItem(LAST_SEARCH_STORAGE_KEY);
    } else {
      localStorage.setItem(LAST_SEARCH_STORAGE_KEY, query);
    }
  } catch {
    // Ignore storage quota or disabled storage errors
  }
}

export function clearLastSearch(): void {
  if (typeof localStorage === "undefined") return;
  try {
    localStorage.removeItem(LAST_SEARCH_STORAGE_KEY);
  } catch {
    // Ignore storage quota or disabled storage errors
  }
}

export function applyThemeAndCaret(settings: SettingsState): void {
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  if (settings.theme === "system") {
    delete root.dataset.theme;
  } else {
    root.dataset.theme = settings.theme;
  }
  delete root.dataset.style;
  root.dataset.infoDensity = settings.informationDensity || "normal";
  root.dataset.layoutDensity = settings.layoutDensity || "normal";
  root.style.setProperty(
    "--caret-color",
    settings.caret ? "var(--fg)" : "transparent",
  );
}

export async function onSettingsChanged(
  handler: (settings: SettingsState) => void,
): Promise<UnlistenFn> {
  return listen<SettingsState>(EVENT_SETTINGS_CHANGED, (event) => {
    handler(event.payload);
  });
}

export function getIdleLauncherHeight(
  _infoDensity: InformationDensity,
  layoutDensity: LayoutDensity,
): number {
  switch (layoutDensity) {
    case "compact":
      return 52;
    case "comfortable":
      return 72;
    case "normal":
    default:
      return 64;
  }
}

export const WELCOME_COMPLETED_KEY = "lux-welcome-completed";

export function isWelcomeCompleted(): boolean {
  if (typeof localStorage === "undefined") return false;
  try {
    return localStorage.getItem(WELCOME_COMPLETED_KEY) === "true";
  } catch {
    return false;
  }
}

export function setWelcomeCompleted(completed: boolean): void {
  if (typeof localStorage === "undefined") return;
  try {
    localStorage.setItem(WELCOME_COMPLETED_KEY, completed ? "true" : "false");
  } catch {}
}
