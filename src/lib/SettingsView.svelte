<script lang="ts">
  import { onMount } from "svelte";
  import {
    showLauncher,
    clearClipboardHistory,
    completeFirstRun,
    isFirstRun,
    onShowWelcome,
    copyClipboardEntry,
    deleteClipboardEntry,
    getClipboardFiltersConfig,
    getClipboardHistory,
    getFeaturesConfig,
    getFilteredClipboardHistory,
    getIndexStats,
    getShortcutsStatus,
    hideSettings,
    onFeaturesChanged,
    onIndexingStatus,
    onOpenSettings,
    reindex,
    saveClipboardFiltersConfig,
    setFeatureEnabled,
    updateShortcuts,
    validateClipboardFilter,
  } from "./tauri";
  import {
    applyThemeAndCaret,
    clearLastSearch,
    isWelcomeCompleted,
    loadSettings,
    saveSettings,
    setWelcomeCompleted,
    type SettingsCategory,
    type SettingsState,
  } from "./settings";
  import ConfirmDialog from "./ConfirmDialog.svelte";
  import type {
    ClipboardEntry,
    ClipboardFiltersConfig,
    CustomFilter,
    FeaturesConfig,
    FilterValidationResult,
    IndexStats,
    ShortcutConfigStatus,
  } from "./types";

  let activeCategory = $state<SettingsCategory>("general");
  let activeSubtab = $state<string>("preferences");

  interface SubtabDef {
    id: string;
    label: string;
  }

  const SUBTABS: Record<SettingsCategory, SubtabDef[]> = {
    welcome: [{ id: "guide", label: "Quick Start" }],
    general: [
      { id: "preferences", label: "Preferences" },
      { id: "shortcuts", label: "Shortcuts" },
    ],
    appearance: [
      { id: "theme", label: "Theme" },
      { id: "layout", label: "Density & Layout" },
    ],
    search: [
      { id: "index", label: "Overview & Index" },
      { id: "prefixes", label: "Prefixes" },
    ],
    clipboard: [
      { id: "general", label: "General" },
      { id: "filters", label: "Privacy Filters" },
      { id: "inspection", label: "Safe Inspection" },
    ],
    features: [{ id: "features", label: "Core Features" }],
    help: [
      { id: "shortcuts", label: "Keyboard Shortcuts" },
      { id: "tips", label: "Tips & Tricks" },
    ],
    about: [
      { id: "about", label: "About Lux" },
      { id: "diagnostics", label: "Diagnostics" },
    ],
  };

  const CATEGORIES: { id: SettingsCategory; label: string }[] = [
    { id: "welcome", label: "Welcome" },
    { id: "general", label: "General" },
    { id: "appearance", label: "Appearance" },
    { id: "search", label: "Search" },
    { id: "clipboard", label: "Clipboard" },
    { id: "features", label: "Features" },
    { id: "help", label: "Help & Tips" },
    { id: "about", label: "About" },
  ];

  let currentSubtabs = $derived(SUBTABS[activeCategory] || []);

  function selectCategory(cat: SettingsCategory): void {
    if (activeCategory === "welcome" && cat !== "welcome") {
      setWelcomeCompleted(true);
      void completeFirstRun();
    }
    activeCategory = cat;
    const subs = SUBTABS[cat];
    activeSubtab = subs && subs.length > 0 ? subs[0].id : "";
  }

  async function handleGetStarted(): Promise<void> {
    setWelcomeCompleted(true);
    await completeFirstRun();
    await hideSettings();
    await showLauncher();
  }

  async function handleExploreSettings(): Promise<void> {
    setWelcomeCompleted(true);
    await completeFirstRun();
    selectCategory("general");
  }

  let settings = $state<SettingsState>(loadSettings());
  let featuresConfig = $state<FeaturesConfig>({ clipboardHistory: true });

  let isIndexing = $state(false);
  let indexStats = $state<IndexStats>({
    totalApps: 0,
    totalFiles: 0,
    isIndexed: false,
  });

  let launcherShortcutInput = $state("Alt+Space");
  let clipboardShortcutInput = $state("Alt+Shift+V");
  let shortcutsStatus = $state<ShortcutConfigStatus | null>(null);
  let shortcutMessage = $state("");
  let shortcutIsError = $state(false);
  let launcherSaveSuccess = $state(false);
  let clipboardSaveSuccess = $state(false);
  let launcherSaveTimer: ReturnType<typeof setTimeout> | undefined;
  let clipboardSaveTimer: ReturnType<typeof setTimeout> | undefined;
  let isSavingShortcuts = $state(false);
  let clipboardCount = $state(0);
  let newPrefixInput = $state("");

  function addPrefix(): void {
    const trimmed = newPrefixInput.trim().toLowerCase();
    if (!trimmed) return;
    if (!settings.flowPrefixes.includes(trimmed)) {
      setSetting("flowPrefixes", [...settings.flowPrefixes, trimmed]);
    }
    newPrefixInput = "";
  }

  function removePrefix(prefixToRemove: string): void {
    setSetting(
      "flowPrefixes",
      settings.flowPrefixes.filter((p) => p !== prefixToRemove),
    );
  }

  let filtersConfig = $state<ClipboardFiltersConfig>({
    mode: "ignore",
    filterSecrets: true,
    filterCamelCase: false,
    filterCreditCards: true,
    customFilters: [],
  });

  let customNameInput = $state("");
  let customPatternInput = $state("");
  let customIsRegex = $state(false);
  let testStringInput = $state("");
  let validationResult = $state<FilterValidationResult | null>(null);
  let isValidating = $state(false);

  let showSafeInspection = $state(false);
  let filteredHistory = $state<ClipboardEntry[]>([]);
  let revealedEntries = $state<Record<string, boolean>>({});

  async function loadFiltersConfig(): Promise<void> {
    try {
      filtersConfig = await getClipboardFiltersConfig();
    } catch {
      // Ignore outside Tauri
    }
  }

  async function updateFilters(newConfig: ClipboardFiltersConfig): Promise<void> {
    filtersConfig = newConfig;
    try {
      await saveClipboardFiltersConfig(newConfig);
    } catch (e) {
      console.error("Failed to save filters config", e);
    }
  }

  async function testCurrentFilter(): Promise<void> {
    const pattern = customPatternInput.trim();
    if (!pattern) return;
    isValidating = true;
    try {
      validationResult = await validateClipboardFilter(
        pattern,
        customIsRegex,
        testStringInput.trim() ? testStringInput : undefined,
      );
    } catch (err) {
      validationResult = { valid: false, error: String(err), matchesTest: null };
    } finally {
      isValidating = false;
    }
  }

  function addCustomFilter(): void {
    const pattern = customPatternInput.trim();
    if (!pattern) return;
    const newFilter: CustomFilter = {
      id: "cf_" + Date.now(),
      name: customNameInput.trim() || pattern,
      pattern,
      isRegex: customIsRegex,
      enabled: true,
    };
    void updateFilters({
      ...filtersConfig,
      customFilters: [...filtersConfig.customFilters, newFilter],
    });
    customNameInput = "";
    customPatternInput = "";
    customIsRegex = false;
    testStringInput = "";
    validationResult = null;
  }

  function toggleCustomFilter(id: string): void {
    void updateFilters({
      ...filtersConfig,
      customFilters: filtersConfig.customFilters.map((f) =>
        f.id === id ? { ...f, enabled: !f.enabled } : f,
      ),
    });
  }

  function removeCustomFilter(id: string): void {
    void updateFilters({
      ...filtersConfig,
      customFilters: filtersConfig.customFilters.filter((f) => f.id !== id),
    });
  }

  async function loadFilteredHistory(): Promise<void> {
    try {
      filteredHistory = await getFilteredClipboardHistory();
    } catch {
      // Ignore
    }
  }

  function toggleReveal(id: string): void {
    revealedEntries[id] = !revealedEntries[id];
  }

  async function handleDeleteFiltered(id: string): Promise<void> {
    try {
      await deleteClipboardEntry(id);
      filteredHistory = filteredHistory.filter((e) => e.id !== id);
    } catch {
      // Ignore
    }
  }

  async function handleCopyFiltered(id: string): Promise<void> {
    try {
      await copyClipboardEntry(id);
    } catch {
      // Ignore
    }
  }

  async function loadFeatures(): Promise<void> {
    try {
      featuresConfig = await getFeaturesConfig();
      if (settings.clipboardHistory !== featuresConfig.clipboardHistory) {
        settings.clipboardHistory = featuresConfig.clipboardHistory;
        saveSettings(settings);
      }
    } catch {
      // Ignore outside Tauri
    }
  }

  async function handleToggleClipboardHistory(enabled: boolean): Promise<void> {
    try {
      featuresConfig = await setFeatureEnabled("clipboardHistory", enabled);
      setSetting("clipboardHistory", enabled);
      await loadShortcutsStatus();
    } catch (err) {
      shortcutMessage = String(err);
      shortcutIsError = true;
    }
  }

  function setSetting<K extends keyof SettingsState>(
    key: K,
    value: SettingsState[K],
  ): void {
    settings[key] = value;
    saveSettings(settings);
    applyThemeAndCaret(settings);
  }

  function toggleRememberSearch(): void {
    const enabled = !settings.rememberSearch;
    if (!enabled) {
      clearLastSearch();
    }
    setSetting("rememberSearch", enabled);
  }

  async function handleClose(): Promise<void> {
    try {
      await hideSettings();
    } catch {
      // Ignore if outside Tauri
    }
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      void handleClose();
    }
  }

  async function triggerReindex(): Promise<void> {
    isIndexing = true;
    try {
      await reindex();
    } catch {
      isIndexing = false;
    }
  }

  async function loadShortcutsStatus(): Promise<void> {
    try {
      const status = await getShortcutsStatus();
      shortcutsStatus = status;
      launcherShortcutInput = status.launcher.shortcut;
      clipboardShortcutInput = status.clipboard.shortcut;
    } catch {
      // Ignore outside Tauri
    }
    try {
      const history = await getClipboardHistory();
      clipboardCount = history.length;
    } catch {
      // Ignore outside Tauri
    }
  }

  async function applyShortcuts(target?: "launcher" | "clipboard"): Promise<void> {
    if (isSavingShortcuts) return;
    isSavingShortcuts = true;
    shortcutMessage = "";
    shortcutIsError = false;
    try {
      const updated = await updateShortcuts(
        launcherShortcutInput.trim(),
        clipboardShortcutInput.trim(),
      );
      shortcutsStatus = updated;
      launcherShortcutInput = updated.launcher.shortcut;
      clipboardShortcutInput = updated.clipboard.shortcut;
      if (updated.launcher.error || updated.clipboard.error) {
        shortcutIsError = true;
        shortcutMessage =
          updated.launcher.error ||
          updated.clipboard.error ||
          "Failed to register shortcut";
      } else {
        shortcutIsError = false;
        shortcutMessage = "";

        if (!target || target === "launcher") {
          launcherSaveSuccess = true;
          clearTimeout(launcherSaveTimer);
          launcherSaveTimer = setTimeout(() => {
            launcherSaveSuccess = false;
          }, 1500);
        }
        if (!target || target === "clipboard") {
          clipboardSaveSuccess = true;
          clearTimeout(clipboardSaveTimer);
          clipboardSaveTimer = setTimeout(() => {
            clipboardSaveSuccess = false;
          }, 1500);
        }
      }
    } catch (err) {
      shortcutIsError = true;
      shortcutMessage = String(err);
    } finally {
      isSavingShortcuts = false;
    }
  }

  let isClearingClipboard = $state(false);
  let showClearConfirmDialog = $state(false);

  async function handleConfirmClearClipboard(): Promise<void> {
    isClearingClipboard = true;
    try {
      await clearClipboardHistory();
      clipboardCount = 0;
    } catch {
      // Ignore outside Tauri
    } finally {
      isClearingClipboard = false;
      showClearConfirmDialog = false;
    }
  }

  onMount(() => {
    applyThemeAndCaret(settings);
    const handleContextMenu = (e: MouseEvent) => {
      const target = e.target as HTMLElement | null;
      if (target?.closest("input, textarea, [contenteditable='true']")) return;
      e.preventDefault();
    };
    window.addEventListener("contextmenu", handleContextMenu);

    void isFirstRun().then((isFirst) => {
      if (isFirst && !isWelcomeCompleted()) {
        activeCategory = "welcome";
        activeSubtab = "guide";
      }
    });

    let unlistenWelcome: (() => void) | undefined;
    void onShowWelcome(() => {
      activeCategory = "welcome";
      activeSubtab = "guide";
    }).then((unlisten) => {
      unlistenWelcome = unlisten;
    });
    void loadFeatures();
    void loadShortcutsStatus();
    void loadFiltersConfig();
    void loadFilteredHistory();

    void getIndexStats()
      .then((stats) => {
        indexStats = stats;
        if (typeof stats.isIndexing === "boolean") {
          isIndexing = stats.isIndexing;
        }
      })
      .catch(() => {});

    let unlistenIndexing: (() => void) | undefined;
    let unlistenOpen: (() => void) | undefined;
    let unlistenFeatures: (() => void) | undefined;

    void onOpenSettings(() => {
      if (activeCategory === "welcome") {
        activeCategory = "general";
        activeSubtab = "preferences";
      }
      settings = loadSettings();
      applyThemeAndCaret(settings);
      void loadFeatures();
      void loadShortcutsStatus();
      void loadFiltersConfig();
      void loadFilteredHistory();
      void getIndexStats()
        .then((stats) => {
          indexStats = stats;
          if (typeof stats.isIndexing === "boolean") {
            isIndexing = stats.isIndexing;
          }
        })
        .catch(() => {});
    }).then((unlisten) => {
      unlistenOpen = unlisten;
    });

    void onFeaturesChanged((newFeatures) => {
      featuresConfig = newFeatures;
      settings.clipboardHistory = newFeatures.clipboardHistory;
      saveSettings(settings);
      void loadShortcutsStatus();
    }).then((unlisten) => {
      unlistenFeatures = unlisten;
    });

    void onIndexingStatus((payload) => {
      isIndexing = payload.isIndexing;
      if (!payload.isIndexing) {
        void getIndexStats()
          .then((stats) => {
            indexStats = stats;
          })
          .catch(() => {});
      }
    }).then((unlisten) => {
      unlistenIndexing = unlisten;
    });

    window.addEventListener("keydown", handleKeydown);

    return () => {
      window.removeEventListener("keydown", handleKeydown);
      window.removeEventListener("contextmenu", handleContextMenu);
      unlistenWelcome?.();
      clearTimeout(launcherSaveTimer);
      clearTimeout(clipboardSaveTimer);
      if (unlistenIndexing) {
        unlistenIndexing();
      }
      if (unlistenOpen) {
        unlistenOpen();
      }
      if (unlistenFeatures) {
        unlistenFeatures();
      }
    };
  });
</script>

<div class="settings-win" role="dialog" aria-label="Lux Main Menu">
  <div class="stb" data-tauri-drag-region>
    <div class="stb-title">
      <div class="logo">
        <svg viewBox="0 0 64 64" width="13" height="13" fill="currentColor">
          <path d="M 10 8 H 22 V 40 L 46 16 H 58 L 34 40 H 58 V 52 H 10 Z" />
        </svg>
      </div>
      <span>Lux Main Menu</span>
    </div>
    <button
      id="sx"
      type="button"
      aria-label="Close menu"
      onclick={handleClose}
    >✕</button>
  </div>

  <div class="main-layout">
    <div class="sidebar" role="tablist" aria-label="Main menu categories">
      {#each CATEGORIES as cat (cat.id)}
        <button
          type="button"
          role="tab"
          class="sidebar-item"
          class:active={activeCategory === cat.id}
          aria-selected={activeCategory === cat.id}
          onclick={() => selectCategory(cat.id)}
        >
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            {#if cat.id === "welcome"}
              <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z"/>
            {:else if cat.id === "general"}
              <circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/>
            {:else if cat.id === "appearance"}
              <circle cx="13.5" cy="6.5" r=".5"/><circle cx="17.5" cy="10.5" r=".5"/><circle cx="8.5" cy="7.5" r=".5"/><circle cx="6.5" cy="12.5" r=".5"/><path d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.926 0 1.648-.746 1.648-1.688 0-.437-.18-.835-.437-1.125-.29-.289-.438-.652-.438-1.125a1.64 1.64 0 0 1 1.668-1.668h1.996c3.051 0 5.563-2.512 5.563-5.563C22 6.5 17.5 2 12 2z"/>
            {:else if cat.id === "search"}
              <circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
            {:else if cat.id === "clipboard"}
              <path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"/><rect x="8" y="2" width="8" height="4" rx="1" ry="1"/>
            {:else if cat.id === "features"}
              <rect x="1" y="5" width="22" height="14" rx="7" ry="7"/><circle cx="16" cy="12" r="3"/>
            {:else if cat.id === "help"}
              <circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/><line x1="12" y1="17" x2="12.01" y2="17"/>
            {:else if cat.id === "about"}
              <circle cx="12" cy="12" r="10"/><line x1="12" y1="16" x2="12" y2="12"/><line x1="12" y1="8" x2="12.01" y2="8"/>
            {/if}
          </svg>
          <span>{cat.label}</span>
        </button>
      {/each}
    </div>

    <div class="main-content">
      {#if currentSubtabs.length > 1}
        <div class="subtabs-bar" role="tablist" aria-label="Category subtabs">
          {#each currentSubtabs as sub (sub.id)}
            <button
              type="button"
              role="tab"
              class="subtab-btn"
              class:active={activeSubtab === sub.id}
              aria-selected={activeSubtab === sub.id}
              onclick={() => (activeSubtab = sub.id)}
            >
              {sub.label}
            </button>
          {/each}
        </div>
      {/if}

      <div class="content-scroll">
    {#if activeCategory === "welcome"}
        <div class="welcome-container">
          <div class="welcome-hero">
            <div class="welcome-logo-badge">
              <svg viewBox="0 0 64 64" width="28" height="28" fill="currentColor">
                <path d="M 10 8 H 22 V 40 L 46 16 H 58 L 34 40 H 58 V 52 H 10 Z" />
              </svg>
            </div>
            <h2 class="welcome-title">Welcome to Lux</h2>
            <p class="welcome-desc">
              A blazingly fast, keyboard-first desktop launcher and workspace companion.
            </p>
          </div>

          <div class="welcome-grid">
            <div class="w-card">
              <div class="w-card-header">
                <span class="w-card-badge">Launcher</span>
                <div class="w-shortcut"><kbd>Alt</kbd> + <kbd>Space</kbd></div>
              </div>
              <p>Find apps, search indexed files, calculate expressions, convert colors, and browse the web with zero delay.</p>
            </div>

            <div class="w-card">
              <div class="w-card-header">
                <span class="w-card-badge">Clipboard History</span>
                <div class="w-shortcut"><kbd>Alt</kbd> + <kbd>Shift</kbd> + <kbd>V</kbd></div>
              </div>
              <p>Recall copied text, code snippets, colors, and images with automatic secret and credential privacy masking.</p>
            </div>

            <div class="w-card">
              <div class="w-card-header">
                <span class="w-card-badge">Quick Look Preview</span>
                <div class="w-shortcut"><kbd>Space</kbd></div>
              </div>
              <p>Preview any selected document, PDF, markdown, code file, archive, or image directly without opening heavy apps.</p>
            </div>

            <div class="w-card">
              <div class="w-card-header">
                <span class="w-card-badge">Contextual Actions</span>
                <div class="w-shortcut"><kbd>Ctrl</kbd> + <kbd>K</kbd> / <kbd>Right Click</kbd></div>
              </div>
              <p>Right-click any item or press Ctrl+K to reveal in File Explorer, copy paths, run as administrator, or execute actions.</p>
            </div>
          </div>

          <div class="welcome-actions">
            <button type="button" class="btn-subtle" onclick={handleExploreSettings}>
              Configure Preferences
            </button>
            <button type="button" class="btn-primary" onclick={handleGetStarted}>
              Get Started
            </button>
          </div>
        </div>
      {:else if activeCategory === "general"}
      {#if activeSubtab === "preferences"}
        <div class="srow">
          <div>
            <b>Launch on Startup</b>
          <small class="desc">Start Lux automatically when logging into Windows</small>
        </div>
        <button
          class="sw"
          type="button"
          role="switch"
          aria-checked={settings.login}
          aria-label="Launch on Startup"
          onclick={() => setSetting("login", !settings.login)}
        ></button>
      </div>

      {:else if activeSubtab === "shortcuts"}
        <div class="srow">
          <div>
            <b>Launcher Global Shortcut</b>
          <small class="desc">Show or hide the command launcher</small>
        </div>
        <div class="shortcut-group">
          <input
            id="launcher-shortcut"
            type="text"
            class="shortcut-input"
            bind:value={launcherShortcutInput}
            onblur={() => void applyShortcuts("launcher")}
            onkeydown={(e) => {
              if (e.key === "Enter") void applyShortcuts("launcher");
            }}
            placeholder="e.g. Alt+Space"
            aria-label="Launcher Global Shortcut"
          />
          <button
            class="btn-sub"
            class:success={launcherSaveSuccess}
            type="button"
            disabled={isSavingShortcuts}
            aria-label={launcherSaveSuccess ? "Shortcut saved successfully" : "Save shortcut"}
            onclick={() => void applyShortcuts("launcher")}
          >
            {#if launcherSaveSuccess}
              <svg
                width="13"
                height="13"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2.5"
                stroke-linecap="round"
                stroke-linejoin="round"
                aria-hidden="true"
              >
                <polyline points="20 6 9 17 4 12" />
              </svg>
              <span>Saved</span>
            {:else}
              <span>Save</span>
            {/if}
          </button>
        </div>
      </div>

      {#if shortcutMessage && shortcutIsError && activeCategory === "general"}
        <div class="shortcut-status-banner error">
          {shortcutMessage}
        </div>
      {/if}

      <div class="srow">
        <div>
          <b>Default Behavior</b>
          <small class="desc">Fast centered launcher window, auto-dismiss when focus is lost</small>
        </div>
        <span class="badge">Standard</span>
      </div>
      {/if}
    {:else if activeCategory === "appearance"}
      {#if activeSubtab === "layout"}
        <div class="srow">
          <div>
            <b>Information Density</b>
          <small class="desc">
            {#if settings.informationDensity === "minimal"}
              Minimal: Essential information only, hiding hints and secondary metadata
            {:else if settings.informationDensity === "detailed"}
              Detailed: Richer descriptions, full paths, and navigation hints
            {:else}
              Normal: Balanced display with standard metadata and hints
            {/if}
          </small>
        </div>
        <div class="seg" role="group" aria-label="Information Density">
          <button
            type="button"
            aria-pressed={settings.informationDensity === "minimal"}
            onclick={() => setSetting("informationDensity", "minimal")}
          >
            Minimal
          </button>
          <button
            type="button"
            aria-pressed={settings.informationDensity === "normal"}
            onclick={() => setSetting("informationDensity", "normal")}
          >
            Normal
          </button>
          <button
            type="button"
            aria-pressed={settings.informationDensity === "detailed"}
            onclick={() => setSetting("informationDensity", "detailed")}
          >
            Detailed
          </button>
        </div>
      </div>

      <div class="srow">
        <div>
          <b>Layout Density</b>
          <small class="desc">
            {#if settings.layoutDensity === "compact"}
              Compact: Tighter row heights, compact control targets, reduced padding
            {:else if settings.layoutDensity === "comfortable"}
              Comfortable: Spacious rows, generous padding, larger touch/click targets
            {:else}
              Normal: Balanced standard spacing and dimensions
            {/if}
          </small>
        </div>
        <div class="seg" role="group" aria-label="Layout Density">
          <button
            type="button"
            aria-pressed={settings.layoutDensity === "compact"}
            onclick={() => setSetting("layoutDensity", "compact")}
          >
            Compact
          </button>
          <button
            type="button"
            aria-pressed={settings.layoutDensity === "normal"}
            onclick={() => setSetting("layoutDensity", "normal")}
          >
            Normal
          </button>
          <button
            type="button"
            aria-pressed={settings.layoutDensity === "comfortable"}
            onclick={() => setSetting("layoutDensity", "comfortable")}
          >
            Comfortable
          </button>
        </div>
      </div>

      {:else if activeSubtab === "theme"}
        <div class="srow">
        <div><b>Theme</b></div>
        <div class="seg" role="group" aria-label="Theme">
          <button
            type="button"
            aria-pressed={settings.theme === "system"}
            onclick={() => setSetting("theme", "system")}
          >
            System
          </button>
          <button
            type="button"
            aria-pressed={settings.theme === "light"}
            onclick={() => setSetting("theme", "light")}
          >
            Light
          </button>
          <button
            type="button"
            aria-pressed={settings.theme === "dark"}
            onclick={() => setSetting("theme", "dark")}
          >
            Dark
          </button>
        </div>
      </div>

      <div class="srow">
        <div>
          <b>Results Layout</b>
          <small class="desc">Display search results as a list or grid</small>
        </div>
        <div class="seg" role="group" aria-label="Results Layout">
          <button
            type="button"
            aria-pressed={settings.layout === "list"}
            onclick={() => setSetting("layout", "list")}
          >
            List
          </button>
          <button
            type="button"
            aria-pressed={settings.layout === "grid"}
            onclick={() => setSetting("layout", "grid")}
          >
            Grid
          </button>
        </div>
      </div>

      <div class="srow">
        <div>
          <b>Text Caret</b>
          <small class="desc">Show blinking cursor in search bar</small>
        </div>
        <button
          class="sw"
          type="button"
          role="switch"
          aria-checked={settings.caret}
          aria-label="Text Caret"
          onclick={() => setSetting("caret", !settings.caret)}
        ></button>
      </div>
      {/if}
    {:else if activeCategory === "search"}
      {#if activeSubtab === "index"}
        <div class="srow">
          <div>
            <b>File &amp; Folder Search</b>
          <small class="desc">Search user folders and all indexed drives</small>
        </div>
        <button
          class="sw"
          type="button"
          role="switch"
          aria-checked={settings.fileSearch}
          aria-label="File and folder search"
          onclick={() => setSetting("fileSearch", !settings.fileSearch)}
        ></button>
      </div>

        <div class="srow">
          <div>
            <b>Remember last search</b>
            <small class="desc">Restore the most recent query when the launcher reopens</small>
          </div>
          <button
            class="sw"
            type="button"
            role="switch"
            aria-checked={settings.rememberSearch}
            aria-label="Remember last search"
            onclick={toggleRememberSearch}
          ></button>
        </div>

        <div class="srow">
          <div>
            <b>Show result preview</b>
            <small class="desc">Display size, location, and type for the selected result</small>
          </div>
          <button
            class="sw"
            type="button"
            role="switch"
            aria-checked={settings.resultPreview}
            aria-label="Show result preview"
            onclick={() => setSetting("resultPreview", !settings.resultPreview)}
          ></button>
        </div>

        <div class="srow">
          <div>
            <b>Show inline autocomplete</b>
            <small class="desc">Show completion suggestion in the search bar</small>
          </div>
          <button
            class="sw"
            type="button"
            role="switch"
            aria-checked={settings.inlineAutocomplete}
            aria-label="Show inline autocomplete"
            onclick={() => setSetting("inlineAutocomplete", !settings.inlineAutocomplete)}
          ></button>
        </div>

        <div class="srow">
          <div>
            <b>Show Frequently Used</b>
            <small class="desc">Show recently used apps and folders when search is empty</small>
          </div>
          <button
            class="sw"
            type="button"
            role="switch"
            aria-checked={settings.showFrequent}
            aria-label="Show Frequently Used"
            onclick={() => setSetting("showFrequent", !settings.showFrequent)}
          ></button>
        </div>

        <div class="srow">
          <div>
            <b>File Previews (Quick Look)</b>
            <small class="desc">Instant file preview window triggered by Space</small>
          </div>
          <button
            class="sw"
            type="button"
            role="switch"
            aria-checked={settings.preview}
            aria-label="File previews"
            onclick={() => setSetting("preview", !settings.preview)}
          ></button>
        </div>

        <div class="srow">
          <div>
            <b>System Index</b>
            <small class="desc">
              {#if isIndexing}
                Indexing applications and files…
              {:else if indexStats.isIndexed}
                {indexStats.totalApps} apps, {indexStats.totalFiles} files indexed
              {:else}
                Index ready
              {/if}
            </small>
          </div>
          <button
            class="btn-sub"
            type="button"
            disabled={isIndexing}
            onclick={triggerReindex}
          >
            {isIndexing ? "Indexing…" : "Re-index now"}
          </button>
        </div>
      {:else if activeSubtab === "prefixes"}
        <div class="srow">
          <div>
            <b>Flow Launcher Prefix Compatibility</b>
          <small class="desc">Ignore leading prefix tokens like "et" when searching</small>
        </div>
        <button
          class="sw"
          type="button"
          role="switch"
          aria-checked={settings.flowPrefixCompatibility}
          aria-label="Flow Launcher prefix compatibility"
          onclick={() =>
            setSetting(
              "flowPrefixCompatibility",
              !settings.flowPrefixCompatibility,
            )}
        ></button>
      </div>

      {#if settings.flowPrefixCompatibility}
        <div class="prefix-container">
          <div class="prefix-list">
            {#each settings.flowPrefixes as prefix}
              <span class="prefix-tag">
                <span>{prefix}</span>
                <button
                  type="button"
                  class="prefix-remove-btn"
                  title="Remove prefix"
                  aria-label="Remove prefix {prefix}"
                  onclick={() => removePrefix(prefix)}
                >
                  &times;
                </button>
              </span>
            {/each}
          </div>
          <div class="prefix-add-row">
            <input
              type="text"
              class="prefix-input"
              placeholder="Add prefix (e.g. et)"
              bind:value={newPrefixInput}
              onkeydown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  addPrefix();
                }
              }}
            />
            <button
              type="button"
              class="btn-sub"
              disabled={!newPrefixInput.trim()}
              onclick={addPrefix}
            >
              Add
            </button>
          </div>
        </div>
      {/if}
      {/if}
    {:else if activeCategory === "clipboard"}
      {#if activeSubtab === "general"}
        <div class="srow">
          <div>
            <b>Clipboard History</b>
          <small class="desc">Monitor, store and search recent clipboard entries</small>
        </div>
        <button
          class="sw"
          type="button"
          role="switch"
          aria-checked={featuresConfig.clipboardHistory}
          aria-label="Clipboard History"
          onclick={() =>
            void handleToggleClipboardHistory(!featuresConfig.clipboardHistory)}
        ></button>
      </div>

      {#if featuresConfig.clipboardHistory}
        <div class="srow">
          <div>
            <b>Clipboard History Shortcut</b>
            <small class="desc">Global hotkey to open clipboard history directly</small>
          </div>
          <div class="shortcut-group">
            <input
              id="clipboard-shortcut"
              type="text"
              class="shortcut-input"
              bind:value={clipboardShortcutInput}
              onblur={() => void applyShortcuts("clipboard")}
              onkeydown={(e) => {
                if (e.key === "Enter") void applyShortcuts("clipboard");
              }}
              placeholder="e.g. Alt+Shift+V"
              aria-label="Clipboard History Shortcut"
            />
            <button
              class="btn-sub"
              class:success={clipboardSaveSuccess}
              type="button"
              disabled={isSavingShortcuts}
              aria-label={clipboardSaveSuccess ? "Shortcut saved successfully" : "Save shortcut"}
              onclick={() => void applyShortcuts("clipboard")}
            >
              {#if clipboardSaveSuccess}
                <svg
                  width="13"
                  height="13"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2.5"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  aria-hidden="true"
                >
                  <polyline points="20 6 9 17 4 12" />
                </svg>
                <span>Saved</span>
              {:else}
                <span>Save</span>
              {/if}
            </button>
          </div>
        </div>

        {#if clipboardShortcutInput.toLowerCase().includes("win") && clipboardShortcutInput.toLowerCase().includes("v")}
          <div class="shortcut-win-v-notice">
            ⚠️ <strong>Win + V</strong> is reserved exclusively by Windows for its native clipboard manager and cannot be registered safely by third-party desktop applications. Use <strong>Alt + Shift + V</strong> or another modifier combination.
          </div>
        {/if}

        {#if shortcutMessage && shortcutIsError && activeCategory === "clipboard"}
          <div class="shortcut-status-banner error">
            {shortcutMessage}
          </div>
        {/if}

        <!-- Exclusion Filter Mode -->
        <div class="srow">
          <div>
            <b>Clipboard Filter Mode</b>
            <small class="desc">
              {#if filtersConfig.mode === "ignore"}
                Ignore completely: matching items are never captured or saved
              {:else}
                Hide filtered entries: captured privately and masked from normal history
              {/if}
            </small>
          </div>
          <div class="seg" role="radiogroup" aria-label="Exclusion filter mode">
            <button
              type="button"
              role="radio"
              aria-checked={filtersConfig.mode === "ignore"}
              onclick={() =>
                void updateFilters({ ...filtersConfig, mode: "ignore" })}
            >
              Ignore
            </button>
            <button
              type="button"
              role="radio"
              aria-checked={filtersConfig.mode === "hide"}
              onclick={() => {
                void updateFilters({ ...filtersConfig, mode: "hide" });
                void loadFilteredHistory();
              }}
            >
              Hide
            </button>
          </div>
        </div>

        <!-- Predefined Filters -->
        <div class="srow">
          <div>
            <b>Filter Secrets &amp; Tokens</b>
            <small class="desc">API keys, private keys, passwords, auth tokens</small>
          </div>
          <button
            class="sw"
            type="button"
            role="switch"
            aria-checked={filtersConfig.filterSecrets}
            aria-label="Filter secrets and tokens"
            onclick={() =>
              void updateFilters({
                ...filtersConfig,
                filterSecrets: !filtersConfig.filterSecrets,
              })}
          ></button>
        </div>

        <div class="srow">
          <div>
            <b>Filter camelCase Identifiers</b>
            <small class="desc">Exclude camelCase code words and variables</small>
          </div>
          <button
            class="sw"
            type="button"
            role="switch"
            aria-checked={filtersConfig.filterCamelCase}
            aria-label="Filter camelCase"
            onclick={() =>
              void updateFilters({
                ...filtersConfig,
                filterCamelCase: !filtersConfig.filterCamelCase,
              })}
          ></button>
        </div>

        <div class="srow">
          <div>
            <b>Filter Credit Card Numbers</b>
            <small class="desc">Match Visa, MasterCard, Amex, and Discover patterns</small>
          </div>
          <button
            class="sw"
            type="button"
            role="switch"
            aria-checked={filtersConfig.filterCreditCards}
            aria-label="Filter credit cards"
            onclick={() =>
              void updateFilters({
                ...filtersConfig,
                filterCreditCards: !filtersConfig.filterCreditCards,
              })}
          ></button>
        </div>

        <!-- Custom Exclusion Filters -->
        <div class="filter-section">
          <div class="filter-section-header">
            <b>Custom Filters</b>
            <small class="desc">Add custom regex or literal strings to exclude</small>
          </div>

          {#if filtersConfig.customFilters.length > 0}
            <div class="custom-filters-list">
              {#each filtersConfig.customFilters as filter (filter.id)}
                <div class="custom-filter-item">
                  <div class="custom-filter-info">
                    <span class="custom-filter-name">{filter.name}</span>
                    <span class="custom-filter-pattern">{filter.pattern}</span>
                    <span class="custom-filter-badge">{filter.isRegex ? "Regex" : "Literal"}</span>
                  </div>
                  <div class="custom-filter-actions">
                    <button
                      class="sw"
                      type="button"
                      role="switch"
                      aria-checked={filter.enabled}
                      aria-label="Toggle filter {filter.name}"
                      onclick={() => toggleCustomFilter(filter.id)}
                    ></button>
                    <button
                      type="button"
                      class="filter-remove-btn"
                      title="Delete filter"
                      aria-label="Delete filter {filter.name}"
                      onclick={() => removeCustomFilter(filter.id)}
                    >
                      &times;
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          {/if}

          <!-- Add / Test form -->
          <div class="add-filter-card">
            <div class="filter-inputs-row">
              <input
                type="text"
                class="filter-text-input filter-name-input"
                placeholder="Filter Name (optional)"
                bind:value={customNameInput}
              />
              <input
                type="text"
                class="filter-text-input filter-pattern-input flex-1"
                placeholder="Pattern (e.g. ^SECRET-.*)"
                bind:value={customPatternInput}
                oninput={() => {
                  validationResult = null;
                }}
              />
              <label class="filter-checkbox-label">
                <input type="checkbox" bind:checked={customIsRegex} />
                <span>Regex</span>
              </label>
            </div>

            <div class="filter-test-row">
              <input
                type="text"
                class="filter-text-input filter-test-input flex-1"
                placeholder="Test string against pattern..."
                bind:value={testStringInput}
                oninput={() => {
                  validationResult = null;
                }}
              />
              <button
                type="button"
                class="btn-sub"
                disabled={!customPatternInput.trim() || isValidating}
                onclick={testCurrentFilter}
              >
                Test
              </button>
              <button
                type="button"
                class="btn-sub"
                disabled={!customPatternInput.trim() || (validationResult !== null && !validationResult.valid)}
                onclick={addCustomFilter}
              >
                Add Filter
              </button>
            </div>

            {#if validationResult}
              <div
                class="validation-banner"
                class:error={!validationResult.valid}
                class:match={validationResult.valid && validationResult.matchesTest === true}
                class:nomatch={validationResult.valid && validationResult.matchesTest === false}
              >
                {#if !validationResult.valid}
                  Invalid Regex: {validationResult.error}
                {:else if validationResult.matchesTest === true}
                  Pattern matches test input!
                {:else if validationResult.matchesTest === false}
                  Pattern is valid, but did not match test input.
                {:else}
                  Pattern is valid!
                {/if}
              </div>
            {/if}
          </div>
        </div>

        <!-- Safe Inspection for Hide Mode -->
        {#if filtersConfig.mode === "hide"}
          <div class="srow">
            <div>
              <b>Safe Inspection</b>
              <small class="desc">{filteredHistory.length} hidden filtered entries safely stored</small>
            </div>
            <button
              type="button"
              class="btn-sub"
              onclick={() => {
                showSafeInspection = !showSafeInspection;
                if (showSafeInspection) void loadFilteredHistory();
              }}
            >
              {showSafeInspection ? "Close Inspection" : "Inspect Filtered Entries"}
            </button>
          </div>

          {#if showSafeInspection}
            <div class="safe-inspection-container">
              {#if filteredHistory.length === 0}
                <div class="safe-inspection-empty">
                  No hidden filtered entries in history.
                </div>
              {:else}
                <div class="safe-inspection-list">
                  {#each filteredHistory as entry (entry.id)}
                    <div class="safe-inspection-item">
                      <div class="safe-inspection-details">
                        <div class="safe-inspection-meta">
                          <span class="safe-filter-badge">{entry.matchedFilter || "Filtered"}</span>
                          <span class="safe-time">{new Date(entry.createdAt).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}</span>
                          {#if entry.sourceApp}
                            <span class="safe-app">{entry.sourceApp}</span>
                          {/if}
                        </div>
                        <div class="safe-inspection-content">
                          {#if revealedEntries[entry.id]}
                            <span class="safe-revealed-text">{entry.name}</span>
                          {:else}
                            <span class="safe-masked-text">••••••••••••••••••••</span>
                          {/if}
                        </div>
                      </div>
                      <div class="safe-inspection-actions">
                        <button
                          type="button"
                          class="btn-sub btn-sm"
                          onclick={() => toggleReveal(entry.id)}
                        >
                          {revealedEntries[entry.id] ? "Hide" : "Reveal"}
                        </button>
                        <button
                          type="button"
                          class="btn-sub btn-sm"
                          title="Copy to clipboard"
                          onclick={() => void handleCopyFiltered(entry.id)}
                        >
                          Copy
                        </button>
                        <button
                          type="button"
                          class="btn-sub btn-sm danger"
                          title="Delete from history"
                          onclick={() => void handleDeleteFiltered(entry.id)}
                        >
                          Delete
                        </button>
                      </div>
                    </div>
                  {/each}
                </div>
              {/if}
            </div>
          {/if}
        {/if}

        <div class="srow storage-row">
          <div>
            <b>Clipboard History Storage</b>
            <small class="desc">{clipboardCount} {clipboardCount === 1 ? "entry" : "entries"} stored locally</small>
          </div>
          <button
            class="btn-sub danger"
            type="button"
            onclick={() => {
              showClearConfirmDialog = true;
            }}
          >
            Clear History
          </button>
        </div>
      {/if}
      {/if}
    {:else if activeCategory === "features"}
      <div class="srow">
        <div>
          <b>Indexing Progress Bar</b>
          <small class="desc">Show subtle progress indicator while background indexing</small>
        </div>
        <button
          class="sw"
          type="button"
          role="switch"
          aria-checked={settings.progressBar}
          aria-label="Indexing progress bar"
          onclick={() => setSetting("progressBar", !settings.progressBar)}
        ></button>
      </div>

      <div class="srow">
        <div>
          <b>System Index</b>
          <small class="desc">
            {#if isIndexing}
              Indexing applications and files…
            {:else if indexStats.isIndexed}
              {indexStats.totalApps} apps, {indexStats.totalFiles} files indexed
            {:else}
              Index ready
            {/if}
          </small>
        </div>
        <button
          class="btn-sub"
          type="button"
          disabled={isIndexing}
          onclick={triggerReindex}
        >
          {isIndexing ? "Indexing…" : "Re-index now"}
        </button>
      </div>
    {:else if activeCategory === "help"}
      <div class="help-section">
        {#if activeSubtab === "shortcuts"}
          <h4 class="help-heading">Keyboard Shortcuts</h4>
        <div class="help-table">
          <div class="help-row">
            <span class="help-desc">Toggle Lux Launcher</span>
            <span class="help-keys"><kbd>Alt</kbd> + <kbd>Space</kbd></span>
          </div>
          <div class="help-row">
            <span class="help-desc">Open Clipboard History</span>
            <span class="help-keys"><kbd>Alt</kbd> + <kbd>Shift</kbd> + <kbd>V</kbd></span>
          </div>
          <div class="help-row">
            <span class="help-desc">Open Selected Item / Copy Result</span>
            <span class="help-keys"><kbd>↵ Enter</kbd></span>
          </div>
          <div class="help-row">
            <span class="help-desc">Item Actions Popover</span>
            <span class="help-keys"><kbd>Ctrl</kbd> + <kbd>K</kbd></span>
          </div>
          <div class="help-row">
            <span class="help-desc">Run Application as Administrator</span>
            <span class="help-keys"><kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>↵</kbd></span>
          </div>
          <div class="help-row">
            <span class="help-desc">Reveal Item in File Explorer</span>
            <span class="help-keys"><kbd>Ctrl</kbd> + <kbd>↵</kbd></span>
          </div>
          <div class="help-row">
            <span class="help-desc">Close / Return to All Categories</span>
            <span class="help-keys"><kbd>Esc</kbd></span>
          </div>
        </div>
        {:else if activeSubtab === "tips"}
          <h4 class="help-heading">Quick Answer Examples</h4>
        <p class="help-intro">Type these directly into the search bar for instant offline answers without pressing Enter:</p>
        <div class="help-grid">
          <div class="help-card">
            <span class="help-cat">Calculations &amp; Math</span>
            <code>(128 * 1.19) / 3</code>
            <code>sqrt(144) + 2^8</code>
            <code>15% of 240</code>
          </div>
          <div class="help-card">
            <span class="help-cat">Taxes &amp; Split</span>
            <code>VAT 23% on 100</code>
            <code>remove 23% VAT from 123</code>
            <code>split 87.50 between 4</code>
          </div>
          <div class="help-card">
            <span class="help-cat">Time Zones &amp; World Time</span>
            <code>time in Tokyo</code>
            <code>10 PM here to London</code>
            <code>time difference Lisbon London</code>
          </div>
          <div class="help-card">
            <span class="help-cat">Dates &amp; Durations</span>
            <code>90 days from today</code>
            <code>days until Christmas</code>
            <code>days between 12 Oct and 21 Oct</code>
          </div>
          <div class="help-card">
            <span class="help-cat">Unit Conversions</span>
            <code>100 km to miles</code>
            <code>100 C to F</code>
            <code>1 GiB to MB (IEC to SI)</code>
          </div>
          <div class="help-card">
            <span class="help-cat">Currency, Colors &amp; Passwords</span>
            <code>25 EUR to USD</code>
            <code>#798F78 to RGB</code>
            <code>generate password 24</code>
          </div>
        </div>
        {/if}
      </div>
    {:else if activeCategory === "about"}
      {#if activeSubtab === "about"}
        <div class="about-card">
        <div class="about-header">
          <div class="logo about-logo">
            <svg viewBox="0 0 64 64" width="24" height="24" fill="currentColor">
              <path d="M 10 8 H 22 V 40 L 46 16 H 58 L 34 40 H 58 V 52 H 10 Z" />
            </svg>
          </div>
          <div>
            <div class="about-title-row">
              <span class="about-title">Lux</span>
              <span class="about-ver">v0.1.0-beta.3</span>
            </div>
            <p class="about-desc">
              Fast, keyboard-first desktop launcher and productivity tool for Windows.
            </p>
          </div>
        </div>

        <div class="about-specs">
          <div class="about-spec-row">
            <span class="spec-label">Runtime</span>
            <span class="spec-val">Tauri 2 · Rust</span>
          </div>
          <div class="about-spec-row">
            <span class="spec-label">Frontend</span>
            <span class="spec-val">SvelteKit · TypeScript</span>
          </div>
          <div class="about-spec-row">
            <span class="spec-label">Privacy</span>
            <span class="spec-val">100% Local-First · No Telemetry</span>
          </div>
          <div class="about-spec-row">
            <span class="spec-label">License</span>
            <span class="spec-val">Open Source (MIT)</span>
          </div>
        </div>
        </div>
      {:else if activeSubtab === "diagnostics"}
        <div class="about-card">
          <h4 style="margin: 0 0 12px; font-size: 13px; font-weight: 600; color: var(--fg);">System &amp; Engine Diagnostics</h4>
          <div class="about-specs">
            <div class="about-spec-row">
              <span class="spec-label">Search Index State</span>
              <span class="spec-val">{indexStats.isIndexed ? "Ready" : "Initializing"}</span>
            </div>
            <div class="about-spec-row">
              <span class="spec-label">Indexed Applications</span>
              <span class="spec-val">{indexStats.totalApps}</span>
            </div>
            <div class="about-spec-row">
              <span class="spec-label">Indexed Files</span>
              <span class="spec-val">{indexStats.totalFiles}</span>
            </div>
            <div class="about-spec-row">
              <span class="spec-label">Clipboard Monitored</span>
              <span class="spec-val">{featuresConfig.clipboardHistory ? "Active" : "Disabled"}</span>
            </div>
            <div class="about-spec-row">
              <span class="spec-label">Privacy Filters Mode</span>
              <span class="spec-val">{filtersConfig.mode}</span>
            </div>
            <div class="about-spec-row">
              <span class="spec-label">Launcher Window Mode</span>
              <span class="spec-val">{settings.layout}</span>
            </div>
          </div>
        </div>
      {/if}
    {/if}
      </div>
    </div>
  </div>

  <ConfirmDialog
    open={showClearConfirmDialog}
    title="Clear Clipboard History?"
    message="Are you sure you want to clear your local clipboard history? Pinned items will be preserved."
    confirmLabel="Clear History"
    isDanger={true}
    isSubmitting={isClearingClipboard}
    onConfirm={handleConfirmClearClipboard}
    onCancel={() => {
      showClearConfirmDialog = false;
    }}
  />
</div>

<style>
  .settings-win,
  .settings-win * {
    box-sizing: border-box;
  }

  .settings-win {
    width: 100vw;
    height: 100vh;
    max-width: 100%;
    max-height: 100%;
    background: var(--bg);
    -webkit-backdrop-filter: blur(24px) saturate(1.4);
    backdrop-filter: blur(24px) saturate(1.4);
    border-radius: 14px;
    box-shadow: var(--shadow), inset 0 1px 0 var(--hi);
    overflow: hidden;
    position: relative;
    display: flex;
    flex-direction: column;
    user-select: none;
  }

  .stb {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 16px;
    font-weight: 600;
    font-size: 13px;
    border-bottom: 1px solid var(--line);
    background: var(--chip);
    flex-shrink: 0;
    width: 100%;
  }

  .stb-title {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--fg);
    min-width: 0;
  }

  .stb-title span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .logo {
    width: 22px;
    height: 22px;
    border-radius: 6px;
    background: var(--fg);
    color: var(--bg);
    display: grid;
    place-items: center;
    flex-shrink: 0;
  }

  .stb button {
    background: none;
    border: 0;
    color: var(--mut);
    cursor: pointer;
    border-radius: 6px;
    padding: 4px 8px;
    font-size: 13px;
    line-height: 1;
    flex-shrink: 0;
    transition: all 0.15s ease;
  }


  .main-layout {
    display: flex;
    flex-direction: row;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .sidebar {
    width: 175px;
    background: rgba(0, 0, 0, 0.08);
    border-right: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 10px 8px;
    flex-shrink: 0;
    overflow-y: auto;
  }

  :global([data-theme="light"]) .sidebar {
    background: rgba(0, 0, 0, 0.03);
  }

  .sidebar-item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border: none;
    background: none;
    color: var(--mut);
    border-radius: 8px;
    cursor: pointer;
    font-size: 13px;
    font-weight: 500;
    text-align: left;
    transition: all 0.15s ease;
    user-select: none;
  }

  .sidebar-item:hover {
    color: var(--fg);
    background: var(--chip);
  }

  .sidebar-item.active {
    color: var(--fg);
    background: var(--sel);
    box-shadow: inset 0 0 0 1px var(--line);
    font-weight: 600;
  }

  .sidebar-item svg {
    flex-shrink: 0;
    opacity: 0.8;
  }

  .sidebar-item.active svg {
    opacity: 1;
  }

  .main-content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: transparent;
  }

  .subtabs-bar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 16px;
    background: var(--chip);
    border-bottom: 1px solid var(--line);
    flex-shrink: 0;
    overflow-x: auto;
  }

  .subtab-btn {
    border: none;
    background: none;
    color: var(--mut);
    font-size: 12px;
    font-weight: 500;
    padding: 5px 12px;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s ease;
    white-space: nowrap;
  }

  .subtab-btn:hover {
    color: var(--fg);
  }

  .subtab-btn.active {
    background: var(--bg);
    color: var(--fg);
    font-weight: 600;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }

  .content-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 16px 20px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  /* Welcome Screen */
  .welcome-container {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 8px 4px 16px;
  }

  .welcome-hero {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 8px;
    padding: 10px 0 6px;
  }

  .welcome-logo-badge {
    width: 48px;
    height: 48px;
    border-radius: 12px;
    background: var(--fg);
    color: var(--bg);
    display: grid;
    place-items: center;
    margin-bottom: 4px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
  }

  .welcome-title {
    margin: 0;
    font-size: 20px;
    font-weight: 700;
    color: var(--fg);
    letter-spacing: -0.02em;
  }

  .welcome-desc {
    margin: 0;
    font-size: 13px;
    color: var(--mut);
    max-width: 420px;
    line-height: 1.45;
  }

  .welcome-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }

  .w-card {
    background: var(--chip);
    border: 1px solid var(--line);
    border-radius: 10px;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    transition: border-color 0.15s ease;
  }

  .w-card:hover {
    border-color: var(--mut);
  }

  .w-card-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 6px;
  }

  .w-card-badge {
    font-size: 12px;
    font-weight: 600;
    color: var(--fg);
  }

  .w-shortcut kbd {
    font-family: inherit;
    font-size: 10.5px;
    background: var(--bg);
    border: 1px solid var(--line);
    border-radius: 4px;
    padding: 2px 5px;
    color: var(--fg);
    font-weight: 500;
  }

  .w-card p {
    margin: 0;
    font-size: 11.5px;
    color: var(--mut);
    line-height: 1.4;
  }

  .welcome-actions {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 10px;
    margin-top: 8px;
    padding-top: 12px;
    border-top: 1px solid var(--line);
  }

  .btn-subtle {
    background: none;
    border: 1px solid var(--line);
    color: var(--fg);
    padding: 7px 14px;
    border-radius: 7px;
    font-size: 12.5px;
    cursor: pointer;
    font-weight: 500;
    transition: all 0.15s ease;
  }

  .btn-subtle:hover {
    background: var(--chip);
  }

  .btn-primary {
    background: var(--fg);
    color: var(--bg);
    border: none;
    padding: 7px 16px;
    border-radius: 7px;
    font-size: 12.5px;
    cursor: pointer;
    font-weight: 600;
    transition: opacity 0.15s ease;
  }

  .btn-primary:hover {
    opacity: 0.9;
  }

  .stb button:hover {
    background: var(--sel);
    color: var(--fg);
  }



  .srow {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 11px 4px;
    border-bottom: 1px solid var(--line);
    gap: 12px;
    width: 100%;
    min-width: 0;
  }

  .srow:last-child {
    border-bottom: 0;
  }

  .srow > div:first-child {
    flex: 1 1 0%;
    min-width: 0;
  }

  .storage-row {
    margin-top: 12px;
    border-top: 1px solid var(--line);
    padding-top: 12px;
  }

  .srow b {
    display: block;
    font-size: 13px;
    font-weight: 500;
    color: var(--fg);
    word-break: break-word;
    overflow-wrap: break-word;
  }

  .srow small {
    display: block;
    color: var(--dim);
    font-size: 11.5px;
    margin-top: 2px;
    word-break: break-word;
    overflow-wrap: break-word;
    line-height: 1.35;
  }

  .sw {
    width: 36px;
    height: 20px;
    border-radius: 10px;
    background: var(--chip);
    border: 1px solid var(--line);
    position: relative;
    cursor: pointer;
    flex-shrink: 0;
    transition: background 0.15s ease;
    padding: 0;
  }

  .sw[aria-checked="true"] {
    background: #34c759;
    border-color: #34c759;
  }

  .sw::after {
    content: "";
    position: absolute;
    top: 2px;
    left: 2px;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
    transition: transform 0.15s ease;
  }

  .sw[aria-checked="true"]::after {
    transform: translateX(16px);
  }

  .seg {
    display: flex;
    background: var(--chip);
    border-radius: 8px;
    padding: 2px;
    gap: 2px;
    flex-shrink: 0;
    max-width: 100%;
  }

  .seg button {
    border: 0;
    background: none;
    font:
      500 11.5px Inter,
      "Segoe UI",
      system-ui,
      sans-serif;
    color: var(--mut);
    padding: 4px 8px;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s ease;
    white-space: nowrap;
    flex-shrink: 0;
  }

  .seg button:hover {
    color: var(--fg);
  }

  .seg button[aria-pressed="true"],
  .seg button[aria-checked="true"] {
    background: var(--bg);
    color: var(--fg);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
  }

  .btn-sub {
    border: 1px solid var(--line);
    background: var(--chip);
    color: var(--fg);
    font:
      500 12px Inter,
      "Segoe UI",
      system-ui,
      sans-serif;
    padding: 5px 11px;
    border-radius: 6px;
    cursor: pointer;
    flex-shrink: 0;
    white-space: nowrap;
    transition: all 0.15s ease;
  }

  .btn-sub:hover:not(:disabled) {
    background: var(--sel);
  }

  .btn-sub:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-sub.danger {
    color: #e5484d;
  }

  .btn-sub.danger:hover {
    background: rgba(229, 72, 77, 0.12);
  }

  .btn-sub.success {
    background: #2e7d32 !important;
    border-color: #2e7d32 !important;
    color: #ffffff !important;
  }

  .btn-sub.success:hover {
    background: #256628 !important;
  }

  @media (prefers-reduced-motion: reduce) {
    .btn-sub {
      transition: none !important;
    }
  }

  .shortcut-group {
    display: flex;
    gap: 6px;
    align-items: center;
    flex-shrink: 0;
    max-width: 100%;
  }

  .shortcut-input {
    background: var(--chip);
    border: 1px solid var(--line);
    border-radius: 6px;
    color: var(--fg);
    font-family: monospace;
    font-size: 12px;
    padding: 5px 8px;
    width: 115px;
    text-align: center;
    outline: none;
    transition: border-color 0.15s;
  }

  .shortcut-input:focus {
    border-color: var(--acc, #e5484d);
  }

  .shortcut-win-v-notice {
    font-size: 11.5px;
    line-height: 1.4;
    color: #d97706;
    background: rgba(217, 119, 6, 0.1);
    border: 1px solid rgba(217, 119, 6, 0.25);
    border-radius: 6px;
    padding: 8px 10px;
    margin: 8px 0;
    word-break: break-word;
    overflow-wrap: break-word;
    width: 100%;
  }

  .shortcut-status-banner {
    font-size: 12px;
    color: #34c759;
    background: rgba(52, 199, 89, 0.1);
    border: 1px solid rgba(52, 199, 89, 0.25);
    border-radius: 6px;
    padding: 6px 10px;
    margin: 8px 0;
    word-break: break-word;
    overflow-wrap: break-word;
    width: 100%;
  }

  .shortcut-status-banner.error {
    color: #e5484d;
    background: rgba(229, 72, 77, 0.1);
    border-color: rgba(229, 72, 77, 0.25);
  }

  .prefix-container {
    padding: 8px 4px 12px;
    border-bottom: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: 100%;
    min-width: 0;
  }

  .prefix-list {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    width: 100%;
  }

  .prefix-tag {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    background: var(--chip);
    border: 1px solid var(--line);
    border-radius: 6px;
    padding: 3px 8px;
    font-size: 12px;
    color: var(--fg);
    font-family: monospace;
    max-width: 100%;
  }

  .prefix-tag span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 160px;
  }

  .prefix-remove-btn {
    background: none;
    border: none;
    color: var(--mut);
    cursor: pointer;
    font-size: 14px;
    line-height: 1;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: color 0.15s ease;
  }

  .prefix-remove-btn:hover {
    color: #e5484d;
  }

  .prefix-add-row {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
    width: 100%;
  }

  .prefix-input {
    background: var(--chip);
    border: 1px solid var(--line);
    border-radius: 6px;
    color: var(--fg);
    font-size: 12px;
    padding: 6px 10px;
    outline: none;
    transition: border-color 0.15s;
    width: 150px;
    max-width: 100%;
  }

  .prefix-input:focus {
    border-color: var(--acc, #e5484d);
  }

  .filter-section {
    padding: 10px 4px;
    border-bottom: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    gap: 10px;
    width: 100%;
    min-width: 0;
  }

  .filter-section-header b {
    display: block;
    font-size: 13px;
    font-weight: 500;
    color: var(--fg);
    word-break: break-word;
  }

  .filter-section-header small {
    display: block;
    color: var(--dim);
    font-size: 11.5px;
    margin-top: 2px;
    word-break: break-word;
  }

  .custom-filters-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    width: 100%;
  }

  .custom-filter-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: var(--chip);
    border: 1px solid var(--line);
    border-radius: 6px;
    padding: 6px 10px;
    gap: 8px;
    width: 100%;
    min-width: 0;
  }

  .custom-filter-info {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    flex: 1 1 0%;
    overflow: hidden;
  }

  .custom-filter-name {
    font-size: 12px;
    font-weight: 500;
    color: var(--fg);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 110px;
    flex-shrink: 0;
  }

  .custom-filter-pattern {
    font-family: monospace;
    font-size: 11.5px;
    color: var(--dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1 1 0%;
    min-width: 0;
  }

  .custom-filter-badge {
    font-size: 10px;
    background: var(--kbd);
    color: var(--mut);
    padding: 1px 5px;
    border-radius: 4px;
    text-transform: uppercase;
    flex-shrink: 0;
  }

  .custom-filter-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .filter-remove-btn {
    background: none;
    border: none;
    color: var(--mut);
    font-size: 16px;
    line-height: 1;
    cursor: pointer;
    padding: 0 4px;
    transition: color 0.15s ease;
  }

  .filter-remove-btn:hover {
    color: #e5484d;
  }

  .add-filter-card {
    display: flex;
    flex-direction: column;
    gap: 8px;
    background: rgba(0, 0, 0, 0.03);
    border: 1px dashed var(--line);
    border-radius: 6px;
    padding: 10px;
    width: 100%;
    min-width: 0;
  }

  .filter-inputs-row,
  .filter-test-row {
    display: flex;
    gap: 8px;
    align-items: center;
    width: 100%;
    min-width: 0;
  }

  .filter-inputs-row {
    flex-wrap: wrap;
  }

  .flex-1 {
    flex: 1 1 0%;
  }

  .filter-name-input {
    flex: 1 1 110px;
    min-width: 0;
  }

  .filter-pattern-input {
    flex: 2 1 140px;
    min-width: 0;
  }

  .filter-test-input {
    flex: 1 1 0%;
    min-width: 0;
  }

  .filter-text-input {
    background: var(--chip);
    border: 1px solid var(--line);
    border-radius: 6px;
    color: var(--fg);
    font-size: 12px;
    padding: 6px 8px;
    outline: none;
    max-width: 100%;
    transition: border-color 0.15s;
  }

  .filter-text-input:focus {
    border-color: var(--acc, #e5484d);
  }

  .filter-checkbox-label {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 12px;
    color: var(--fg);
    cursor: pointer;
    user-select: none;
    flex-shrink: 0;
  }

  .validation-banner {
    font-size: 11.5px;
    padding: 6px 10px;
    border-radius: 6px;
    background: var(--chip);
    color: var(--fg);
    width: 100%;
    word-break: break-word;
    overflow-wrap: break-word;
  }

  .validation-banner.match {
    color: #34c759;
    background: rgba(52, 199, 89, 0.1);
    border: 1px solid rgba(52, 199, 89, 0.25);
  }

  .validation-banner.nomatch {
    color: #d97706;
    background: rgba(217, 119, 6, 0.1);
    border: 1px solid rgba(217, 119, 6, 0.25);
  }

  .validation-banner.error {
    color: #e5484d;
    background: rgba(229, 72, 77, 0.1);
    border: 1px solid rgba(229, 72, 77, 0.25);
  }

  .safe-inspection-container {
    padding: 8px 4px;
    border-bottom: 1px solid var(--line);
    width: 100%;
    min-width: 0;
  }

  .safe-inspection-empty {
    font-size: 12px;
    color: var(--dim);
    font-style: italic;
    padding: 6px 0;
  }

  .safe-inspection-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 200px;
    overflow-y: auto;
    overflow-x: hidden;
    width: 100%;
  }

  .safe-inspection-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: var(--chip);
    border: 1px solid var(--line);
    border-radius: 6px;
    padding: 6px 8px;
    gap: 8px;
    width: 100%;
    min-width: 0;
  }

  .safe-inspection-details {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1 1 0%;
  }

  .safe-inspection-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
    color: var(--dim);
  }

  .safe-filter-badge {
    background: rgba(229, 72, 77, 0.15);
    color: #e5484d;
    padding: 1px 5px;
    border-radius: 4px;
    font-weight: 500;
  }

  .safe-inspection-content {
    font-family: monospace;
    font-size: 11.5px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    width: 100%;
    min-width: 0;
  }

  .safe-masked-text {
    letter-spacing: 2px;
    color: var(--mut);
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 100%;
  }

  .safe-revealed-text {
    color: var(--fg);
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 100%;
  }

  .safe-inspection-actions {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }

  .btn-sm {
    padding: 3px 7px;
    font-size: 11px;
  }

  .badge {
    background: var(--kbd);
    color: var(--mut);
    font:
      500 11px Inter,
      "Segoe UI",
      system-ui,
      sans-serif;
    padding: 3px 8px;
    border-radius: 6px;
    border: 1px solid var(--line);
    flex-shrink: 0;
  }

  .help-section {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 8px 4px 16px;
    width: 100%;
    min-width: 0;
  }

  .help-heading {
    font-size: 13px;
    font-weight: 600;
    margin: 4px 0 0;
    color: var(--fg);
  }

  .help-intro {
    font-size: 11.5px;
    color: var(--mut);
    margin: -6px 0 2px;
  }

  .help-table {
    display: flex;
    flex-direction: column;
    background: var(--sel);
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 8px 12px;
  }

  .help-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 5px 0;
  }

  .help-row:not(:last-child) {
    border-bottom: 1px solid var(--line);
  }

  .help-desc {
    font-size: 12px;
    color: var(--fg);
  }

  .help-keys {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  .help-keys kbd {
    background: var(--kbd);
    color: var(--fg);
    border: 1px solid var(--line);
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 10.5px;
    font-family: inherit;
  }

  .help-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 10px;
  }

  .help-card {
    display: flex;
    flex-direction: column;
    gap: 6px;
    background: var(--sel);
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 10px 12px;
  }

  .help-cat {
    font-size: 11px;
    font-weight: 600;
    color: var(--accent, #4a8fd8);
    margin-bottom: 2px;
  }

  .help-card code {
    font-size: 11px;
    background: var(--chip);
    color: var(--fg);
    padding: 3px 6px;
    border-radius: 4px;
    font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    border: 1px solid var(--line);
  }

  .about-card {
    padding: 12px 4px;
    display: flex;
    flex-direction: column;
    gap: 16px;
    width: 100%;
    min-width: 0;
  }

  .about-header {
    display: flex;
    gap: 14px;
    align-items: center;
    min-width: 0;
  }

  .about-logo {
    width: 44px;
    height: 44px;
    border-radius: 10px;
    flex-shrink: 0;
  }

  .about-title-row {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }

  .about-title {
    font-size: 19px;
    font-weight: 700;
    color: var(--fg);
  }

  .about-ver {
    font-size: 12px;
    color: var(--dim);
    font-family: monospace;
  }

  .about-desc {
    margin: 4px 0 0;
    font-size: 12px;
    color: var(--mut);
    line-height: 1.4;
    word-break: break-word;
  }

  .about-specs {
    display: flex;
    flex-direction: column;
    gap: 8px;
    background: var(--chip);
    border: 1px solid var(--line);
    border-radius: 8px;
    padding: 10px 12px;
    width: 100%;
  }

  .about-spec-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 12px;
    gap: 8px;
    min-width: 0;
  }

  .spec-label {
    color: var(--dim);
    flex-shrink: 0;
  }

  .spec-val {
    color: var(--fg);
    font-weight: 500;
    text-align: right;
    word-break: break-word;
    min-width: 0;
  }

  /* Information Density: Minimal hides secondary description helpers */
  :global(:root[data-info-density="minimal"]) .srow small.desc {
    display: none;
  }

  /* Information Density: Detailed emphasizes metadata and descriptions */
  :global(:root[data-info-density="detailed"]) .srow b {
    font-size: 13.5px;
  }

  :global(:root[data-info-density="detailed"]) .srow small {
    font-size: 12px;
  }

  /* Layout Density: Compact */
  :global(:root[data-layout-density="compact"]) .srow {
    padding: 7px 4px;
    gap: 8px;
  }

  :global(:root[data-layout-density="compact"]) .stb {
    height: 36px;
    padding: 0 12px;
  }





  :global(:root[data-layout-density="compact"]) .shortcut-input {
    width: 105px;
    padding: 4px 6px;
    font-size: 11px;
  }

  /* Layout Density: Comfortable */
  :global(:root[data-layout-density="comfortable"]) .srow {
    padding: 14px 4px;
    gap: 14px;
  }

  :global(:root[data-layout-density="comfortable"]) .stb {
    height: 48px;
    padding: 0 16px;
  }





  :global(:root[data-layout-density="comfortable"]) .seg button {
    padding: 5px 9px;
  }

  :global(:root[data-layout-density="comfortable"]) .shortcut-input {
    width: 120px;
    padding: 6px 8px;
  }
</style>
