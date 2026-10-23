import {
  DEFAULT_SETTINGS,
  loadSettings,
  saveSettings,
  applyThemeAndCaret,
  clearLastSearch,
  getIdleLauncherHeight,
  loadLastSearch,
  saveLastSearch,
  SETTINGS_STORAGE_KEY,
  LAST_SEARCH_STORAGE_KEY,
  isWelcomeCompleted,
  setWelcomeCompleted,
  WELCOME_COMPLETED_KEY,
  type SettingsCategory,
  type SettingsState,
} from "./settings";
import {
  ITEM_USAGE_STORAGE_KEY,
  getFrequentItems,
  loadUsageEntries,
  recordItemUsage,
} from "./usage";
import type { LauncherItem } from "./types";

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
  assertEqual(settings.rememberSearch, false, "remember search disabled by default");
  assertEqual(settings.resultPreview, false, "result preview disabled by default");
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

// 8. Idle launcher height calculation across all 9 density combinations
{
  // Compact layout: 52px (exact match to 52px search bar)
  assertEqual(getIdleLauncherHeight("minimal", "compact"), 52, "minimal + compact idle height matches 52px bar");
  assertEqual(getIdleLauncherHeight("normal", "compact"), 52, "normal + compact idle height matches 52px bar");
  assertEqual(getIdleLauncherHeight("detailed", "compact"), 52, "detailed + compact idle height matches 52px bar");

  // Normal layout: 64px (exact match to 64px search bar)
  assertEqual(getIdleLauncherHeight("minimal", "normal"), 64, "minimal + normal idle height matches 64px bar");
  assertEqual(getIdleLauncherHeight("normal", "normal"), 64, "normal + normal idle height matches 64px bar");
  assertEqual(getIdleLauncherHeight("detailed", "normal"), 64, "detailed + normal idle height matches 64px bar");

  // Comfortable layout: 72px (exact match to 72px search bar)
  assertEqual(getIdleLauncherHeight("minimal", "comfortable"), 72, "minimal + comfortable idle height matches 72px bar");
  assertEqual(getIdleLauncherHeight("normal", "comfortable"), 72, "normal + comfortable idle height matches 72px bar");
  assertEqual(getIdleLauncherHeight("detailed", "comfortable"), 72, "detailed + comfortable idle height matches 72px bar");

  // Verify layout density hierarchy: compact < normal < comfortable
  assertEqual(
    getIdleLauncherHeight("normal", "compact") < getIdleLauncherHeight("normal", "normal"),
    true,
    "compact is smaller than normal",
  );
  assertEqual(
    getIdleLauncherHeight("normal", "normal") < getIdleLauncherHeight("normal", "comfortable"),
    true,
    "normal is smaller than comfortable",
  );
}

// 10. Welcome completion persistence
{
  localStorage.clear();
  assertEqual(isWelcomeCompleted(), false, "initially welcome is not completed");
  setWelcomeCompleted(true);
  assertEqual(isWelcomeCompleted(), true, "welcome is completed after setting");
  assertEqual(localStorage.getItem(WELCOME_COMPLETED_KEY), "true", "storage key matches");
  setWelcomeCompleted(false);
  assertEqual(isWelcomeCompleted(), false, "welcome can be reset");
}

// 11. Welcome category typing
{
  const cat: SettingsCategory = "welcome";
  assertEqual(cat, "welcome", "welcome is a valid SettingsCategory");
}

// 12. Last search persistence
{
  localStorage.clear();
  assertEqual(loadLastSearch(), "", "no saved search initially");
  saveLastSearch("google");
  assertEqual(loadLastSearch(), "google", "last search is restored");
  assertEqual(
    localStorage.getItem(LAST_SEARCH_STORAGE_KEY),
    "google",
    "last search uses dedicated storage key",
  );
  saveLastSearch("lux launcher");
  assertEqual(loadLastSearch(), "lux launcher", "only the latest search is retained");
  saveLastSearch("   ");
  assertEqual(loadLastSearch(), "", "blank search clears the saved value");
  saveLastSearch("settings");
  clearLastSearch();
  assertEqual(loadLastSearch(), "", "saved search can be cleared explicitly");
}

// 13. Remember search and result preview setting round-trip
{
  localStorage.clear();
  const state: SettingsState = {
    ...DEFAULT_SETTINGS,
    rememberSearch: true,
    resultPreview: true,
  };
  saveSettings(state);
  const reloaded = loadSettings();
  assertEqual(reloaded.rememberSearch, true, "remember search persists");
  assertEqual(reloaded.resultPreview, true, "result preview persists");
}

// 14. Local item usage tracking
{
  localStorage.clear();
  const app: LauncherItem = {
    category: "app",
    name: "Firefox",
    subtitle: "Start Menu",
    keywords: "",
    rightLabel: "Application",
    primaryLabel: "Open Application",
    actions: [],
    path: "C:\\ProgramData\\Microsoft\\Windows\\Start Menu\\Firefox.lnk",
    location: "C:\\Program Files\\Firefox\\firefox.exe",
  };
  const folder: LauncherItem = {
    category: "file",
    name: "Projects",
    subtitle: "D:\\Projects",
    keywords: "",
    rightLabel: "Folder",
    primaryLabel: "Open Folder",
    actions: [],
    path: "D:\\Projects",
    location: "D:\\Projects",
  };
  const doc: LauncherItem = {
    category: "file",
    name: "notes.txt",
    subtitle: "D:\\notes.txt",
    keywords: "",
    rightLabel: "TXT",
    primaryLabel: "Open TXT",
    actions: [],
    path: "D:\\notes.txt",
    location: "D:\\",
  };

  recordItemUsage(app, 1000);
  recordItemUsage(folder, 2000);
  recordItemUsage(app, 3000);
  recordItemUsage(doc, 1500);

  const entries = loadUsageEntries();
  assertEqual(entries.length, 3, "usage tracks distinct items");
  assertEqual(
    entries.find((entry) => entry.name === "Firefox")?.count,
    2,
    "usage increments repeated launches",
  );
  assertEqual(
    localStorage.getItem(ITEM_USAGE_STORAGE_KEY) !== null,
    true,
    "usage is persisted locally",
  );

  const all = getFrequentItems(entries, "all", 4);
  assertEqual(all.length, 3, "all mode returns tracked items");
  assertEqual(all[0].name, "Firefox", "most launched item ranks first");
  assertEqual(getFrequentItems(entries, "apps", 4).length, 1, "apps mode filters items");
  assertEqual(
    getFrequentItems(entries, "folders", 4)[0]?.name,
    "Projects",
    "folders mode keeps only folders",
  );
}

// 15. Invalid usage data is ignored safely
{
  localStorage.clear();
  localStorage.setItem(ITEM_USAGE_STORAGE_KEY, "{not-json");
  assertEqual(loadUsageEntries().length, 0, "malformed usage data is ignored");

  localStorage.setItem(
    ITEM_USAGE_STORAGE_KEY,
    JSON.stringify([{ key: "invalid", count: -1 }]),
  );
  assertEqual(loadUsageEntries().length, 0, "invalid usage entries are discarded");
}

console.log("Settings and Density unit tests passed successfully.");
