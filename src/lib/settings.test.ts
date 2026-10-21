import {
  DEFAULT_SETTINGS,
  loadSettings,
  saveSettings,
  applyThemeAndCaret,
  SETTINGS_STORAGE_KEY,
  type SettingsState,
} from "./settings";

function assertEqual<T>(actual: T, expected: T, message?: string): void {
  if (actual !== expected) {
    throw new Error(
      `${message ? message + ": " : ""}expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`,
    );
  }
}

// Mock localStorage for node environment
const storage: Record<string, string> = {};
(globalThis as unknown as { localStorage: Storage }).localStorage = {
  getItem: (key: string) => storage[key] ?? null,
  setItem: (key: string, value: string) => {
    storage[key] = value;
  },
  removeItem: (key: string) => {
    delete storage[key];
  },
  clear: () => {
    for (const k in storage) delete storage[k];
  },
  key: (i: number) => Object.keys(storage)[i] ?? null,
  length: Object.keys(storage).length,
};

console.log("Running Settings and Density unit tests...");

// 1. Default settings
{
  localStorage.clear();
  const settings = loadSettings();
  assertEqual(settings.informationDensity, "normal", "default information density");
  assertEqual(settings.layoutDensity, "normal", "default layout density");
  assertEqual(settings.flowPrefixCompatibility, true, "default prefix compat");
  assertEqual(settings.flowPrefixes[0], "et", "default prefix list contains et");
}

// 2. Backward compatibility with legacy interfaceStyle "minimal"
{
  localStorage.clear();
  localStorage.setItem(
    SETTINGS_STORAGE_KEY,
    JSON.stringify({ interfaceStyle: "minimal" }),
  );
  const loaded = loadSettings();
  assertEqual(loaded.informationDensity, "minimal", "migrated info density from minimal");
  assertEqual(loaded.layoutDensity, "compact", "migrated layout density from minimal");
}

// 3. Backward compatibility with legacy interfaceStyle "detailed"
{
  localStorage.clear();
  localStorage.setItem(
    SETTINGS_STORAGE_KEY,
    JSON.stringify({ interfaceStyle: "detailed" }),
  );
  const loaded = loadSettings();
  assertEqual(loaded.informationDensity, "detailed", "migrated info density from detailed");
  assertEqual(loaded.layoutDensity, "comfortable", "migrated layout density from detailed");
}

// 4. Backward compatibility with legacy interfaceStyle "normal"
{
  localStorage.clear();
  localStorage.setItem(
    SETTINGS_STORAGE_KEY,
    JSON.stringify({ interfaceStyle: "normal" }),
  );
  const loaded = loadSettings();
  assertEqual(loaded.informationDensity, "normal", "migrated info density from normal");
  assertEqual(loaded.layoutDensity, "normal", "migrated layout density from normal");
}

// 5. Orthogonality: Minimal information density with Comfortable layout density
{
  localStorage.clear();
  const state: SettingsState = {
    ...DEFAULT_SETTINGS,
    informationDensity: "minimal",
    layoutDensity: "comfortable",
  };
  saveSettings(state);
  const reloaded = loadSettings();
  assertEqual(reloaded.informationDensity, "minimal", "independent minimal info density");
  assertEqual(reloaded.layoutDensity, "comfortable", "independent comfortable layout density");
}

// 6. Orthogonality: Detailed information density with Compact layout density
{
  localStorage.clear();
  const state: SettingsState = {
    ...DEFAULT_SETTINGS,
    informationDensity: "detailed",
    layoutDensity: "compact",
  };
  saveSettings(state);
  const reloaded = loadSettings();
  assertEqual(reloaded.informationDensity, "detailed", "independent detailed info density");
  assertEqual(reloaded.layoutDensity, "compact", "independent compact layout density");
}

// 7. applyThemeAndCaret DOM binding
{
  const mockDataset: Record<string, string> = { style: "legacy" };
  const mockStyle = {
    setProperty: (prop: string, val: string) => {
      mockStyleProps[prop] = val;
    },
  };
  const mockStyleProps: Record<string, string> = {};

  (globalThis as unknown as { document: unknown }).document = {
    documentElement: {
      dataset: mockDataset,
      style: mockStyle,
    },
  };

  const state: SettingsState = {
    ...DEFAULT_SETTINGS,
    theme: "dark",
    informationDensity: "minimal",
    layoutDensity: "comfortable",
    caret: true,
  };
  applyThemeAndCaret(state);

  assertEqual(mockDataset.theme, "dark", "theme dataset");
  assertEqual(mockDataset.infoDensity, "minimal", "infoDensity dataset");
  assertEqual(mockDataset.layoutDensity, "comfortable", "layoutDensity dataset");
  assertEqual(mockDataset.style, undefined, "legacy style dataset removed");
  assertEqual(mockStyleProps["--caret-color"], "var(--fg)", "caret visible");
}

console.log("Settings and Density unit tests passed successfully.");
