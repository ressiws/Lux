import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";

export type SettingsCategory = "general" | "appearance" | "search" | "features" | "shortcuts";

export interface SettingsState {
  theme: "system" | "light" | "dark";
  layout: "list" | "grid";
  preview: boolean;
  login: boolean;
  progressBar: boolean;
  caret: boolean;
  fileSearch: boolean;
  clipboardHistory: boolean;
  flowPrefixCompatibility: boolean;
  flowPrefixes: string[];
}

export const DEFAULT_SETTINGS: SettingsState = {
  theme: "system",
  layout: "list",
  preview: true,
  login: false,
  progressBar: true,
  caret: true,
  fileSearch: true,
  clipboardHistory: true,
  flowPrefixCompatibility: true,
  flowPrefixes: ["et"],
};

export const SETTINGS_STORAGE_KEY = "lux-settings";
export const EVENT_SETTINGS_CHANGED = "lux://settings-changed";

export function loadSettings(): SettingsState {
  if (typeof localStorage === "undefined") {
    return { ...DEFAULT_SETTINGS };
  }
  try {
    const raw = localStorage.getItem(SETTINGS_STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
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

export function applyThemeAndCaret(settings: SettingsState): void {
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  if (settings.theme === "system") {
    delete root.dataset.theme;
  } else {
    root.dataset.theme = settings.theme;
  }
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
