<script lang="ts">
  import { onMount } from "svelte";
  import {
    clearClipboardHistory,
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
    loadSettings,
    saveSettings,
    type SettingsCategory,
    type SettingsState,
  } from "./settings";
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

  async function handleClearClipboard(): Promise<void> {
    try {
      await clearClipboardHistory();
      clipboardCount = 0;
    } catch {
      // Ignore outside Tauri
    }
  }

  onMount(() => {
    applyThemeAndCaret(settings);
    void loadFeatures();
    void loadShortcutsStatus();
    void loadFiltersConfig();
    void loadFilteredHistory();

    void getIndexStats()
      .then((stats) => {
        indexStats = stats;
      })
      .catch(() => {});

    let unlistenIndexing: (() => void) | undefined;
    let unlistenOpen: (() => void) | undefined;
    let unlistenFeatures: (() => void) | undefined;

    void onOpenSettings(() => {
      settings = loadSettings();
      applyThemeAndCaret(settings);
      void loadFeatures();
      void loadShortcutsStatus();
      void loadFiltersConfig();
      void loadFilteredHistory();
      void getIndexStats()
        .then((stats) => {
          indexStats = stats;
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

<div class="settings-win" role="dialog" aria-label="Lux Settings">
  <div class="stb" data-tauri-drag-region>
    <div class="stb-title">
      <div class="logo">
        <svg viewBox="0 0 64 64" width="13" height="13" fill="currentColor">
          <path d="M 10 8 H 22 V 40 L 46 16 H 58 L 34 40 H 58 V 52 H 10 Z" />
        </svg>
      </div>
      <span>Lux Settings</span>
    </div>
    <button
      id="sx"
      type="button"
      aria-label="Close settings"
      onclick={handleClose}
    >✕</button>
  </div>

  <div class="set-tabs" role="tablist" aria-label="Settings categories">
    <button
      type="button"
      role="tab"
      class:active={activeCategory === "general"}
      aria-selected={activeCategory === "general"}
      onclick={() => (activeCategory = "general")}
    >
      General
    </button>
    <button
      type="button"
      role="tab"
      class:active={activeCategory === "appearance"}
      aria-selected={activeCategory === "appearance"}
      onclick={() => (activeCategory = "appearance")}
    >
      Appearance
    </button>
    <button
      type="button"
      role="tab"
      class:active={activeCategory === "search"}
      aria-selected={activeCategory === "search"}
      onclick={() => (activeCategory = "search")}
    >
      Search
    </button>
    <button
      type="button"
      role="tab"
      class:active={activeCategory === "clipboard"}
      aria-selected={activeCategory === "clipboard"}
      onclick={() => (activeCategory = "clipboard")}
    >
      Clipboard
    </button>
    <button
      type="button"
      role="tab"
      class:active={activeCategory === "features"}
      aria-selected={activeCategory === "features"}
      onclick={() => (activeCategory = "features")}
    >
      Features
    </button>
    <button
      type="button"
      role="tab"
      class:active={activeCategory === "help"}
      aria-selected={activeCategory === "help"}
      onclick={() => (activeCategory = "help")}
    >
      Help &amp; Shortcuts
    </button>
    <button
      type="button"
      role="tab"
      class:active={activeCategory === "about"}
      aria-selected={activeCategory === "about"}
      onclick={() => (activeCategory = "about")}
    >
      About
    </button>
  </div>

  <div class="set-body">
    {#if activeCategory === "general"}
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
    {:else if activeCategory === "appearance"}
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
    {:else if activeCategory === "search"}
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
    {:else if activeCategory === "clipboard"}
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
            onclick={handleClearClipboard}
          >
            Clear History
          </button>
        </div>
      {/if}
    {:else if activeCategory === "features"}
      <div class="srow">
        <div>
          <b>Preview Panel</b>
          <small class="desc">Show details, metadata, and quick actions for the selected item</small>
        </div>
        <button
          class="sw"
          type="button"
          role="switch"
          aria-checked={settings.preview}
          aria-label="Preview Panel"
          onclick={() => setSetting("preview", !settings.preview)}
        ></button>
      </div>

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
            <span class="help-keys"><kbd>Ctrl</kbd> + <kbd>O</kbd></span>
          </div>
          <div class="help-row">
            <span class="help-desc">Close / Return to All Categories</span>
            <span class="help-keys"><kbd>Esc</kbd></span>
          </div>
        </div>

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
      </div>
    {:else if activeCategory === "about"}
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
              <span class="about-ver">v0.1.0-beta.1</span>
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
    {/if}
  </div>
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

  .stb button:hover {
    background: var(--sel);
    color: var(--fg);
  }

  .set-tabs {
    display: flex;
    gap: 3px;
    padding: 6px 10px;
    background: var(--chip);
    border-bottom: 1px solid var(--line);
    flex-shrink: 0;
    width: 100%;
    max-width: 100%;
    overflow-x: auto;
    scrollbar-width: none;
  }

  .set-tabs::-webkit-scrollbar {
    display: none;
  }

  .set-tabs button {
    flex: 1 1 0%;
    min-width: 0;
    border: 0;
    background: none;
    color: var(--mut);
    font:
      500 12px Inter,
      "Segoe UI",
      system-ui,
      sans-serif;
    padding: 6px 6px;
    border-radius: 6px;
    cursor: pointer;
    white-space: nowrap;
    text-align: center;
    text-overflow: ellipsis;
    overflow: hidden;
    transition: all 0.15s ease;
  }

  .set-tabs button:hover {
    color: var(--fg);
  }

  .set-tabs button.active {
    background: var(--bg);
    color: var(--fg);
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.12);
  }

  .set-body {
    flex: 1 1 0%;
    min-height: 0;
    max-height: 100%;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 10px 16px;
    scrollbar-width: thin;
    width: 100%;
    max-width: 100%;
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

  :global(:root[data-layout-density="compact"]) .set-tabs button {
    padding: 4px 6px;
    font-size: 11px;
  }

  :global(:root[data-layout-density="compact"]) .set-body {
    padding: 8px 12px;
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

  :global(:root[data-layout-density="comfortable"]) .set-tabs button {
    padding: 7px 8px;
    font-size: 12px;
  }

  :global(:root[data-layout-density="comfortable"]) .set-body {
    padding: 14px 16px;
  }

  :global(:root[data-layout-density="comfortable"]) .seg button {
    padding: 5px 9px;
  }

  :global(:root[data-layout-density="comfortable"]) .shortcut-input {
    width: 120px;
    padding: 6px 8px;
  }
</style>
