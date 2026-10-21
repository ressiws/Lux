import { emit, listen, type UnlistenFn } from "@tauri-apps/api/event";

export type SettingsCategory =
  | "general"
  | "appearance"
  | "search"
  | "clipboard"
  | "features"
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
  clipboardHistory: boolean;
  flowPrefixCompatibility: boolean;
  flowPrefixes: string[];
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
  infoDensity: InformationDensity,
  layoutDensity: LayoutDensity,
): number {
  if (infoDensity === "minimal") {
    switch (layoutDensity) {
      case "compact":
        return 54;
      case "comfortable":
        return 76;
      case "normal":
      default:
        return 68;
    }
  } else if (infoDensity === "detailed") {
    switch (layoutDensity) {
      case "compact":
        return 132;
      case "comfortable":
        return 172;
      case "normal":
      default:
        return 152;
    }
  } else {
    // normal info density
    switch (layoutDensity) {
      case "compact":
        return 124;
      case "comfortable":
        return 164;
      case "normal":
      default:
        return 144;
    }
  }
}
