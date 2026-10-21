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

  async function applyShortcuts(): Promise<void> {
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
        shortcutMessage = "Shortcuts saved and active";
      }
    } catch (err) {
      shortcutIsError = true;
      shortcutMessage = String(err);
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
      class:active={activeCategory === "features"}
      aria-selected={activeCategory === "features"}
      onclick={() => (activeCategory = "features")}
    >
      Features
    </button>
    <button
      type="button"
      role="tab"
      class:active={activeCategory === "shortcuts"}
      aria-selected={activeCategory === "shortcuts"}
      onclick={() => (activeCategory = "shortcuts")}
    >
      Shortcuts
    </button>
  </div>

  <div class="set-body">
    {#if activeCategory === "general"}
      <div class="srow">
        <div>
          <b>Launch on Startup</b>
          <small>Start Lux automatically when logging in</small>
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
          <b>System Index</b>
          <small>
            {#if isIndexing}
              Indexing applications and files…
            {:else if indexStats.isIndexed}
              {indexStats.totalApps} apps, {indexStats.totalFiles} files
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
      <div class="srow">
        <div>
          <b>Global Shortcut</b>
          <small>Show or hide Lux</small>
        </div>
        <span class="k"><kbd>Alt</kbd><kbd>Space</kbd></span>
      </div>
    {:else if activeCategory === "appearance"}
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
          <small>Display results as a list or grid</small>
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
          <b>Interface Style</b>
          <small>
            {#if settings.interfaceStyle === "minimal"}
              Compact single-line items, no footer noise, maximum density
            {:else if settings.interfaceStyle === "detailed"}
              Richer metadata, larger touch targets, full status information
            {:else}
              Balanced layout with standard spacing and previews
            {/if}
          </small>
        </div>
        <div class="seg" role="group" aria-label="Interface Style">
          <button
            type="button"
            aria-pressed={settings.interfaceStyle === "minimal"}
            onclick={() => setSetting("interfaceStyle", "minimal")}
          >
            Minimal
          </button>
          <button
            type="button"
            aria-pressed={settings.interfaceStyle === "normal"}
            onclick={() => setSetting("interfaceStyle", "normal")}
          >
            Normal
          </button>
          <button
            type="button"
            aria-pressed={settings.interfaceStyle === "detailed"}
            onclick={() => setSetting("interfaceStyle", "detailed")}
          >
            Detailed
          </button>
        </div>
      </div>
      <div class="srow">
        <div>
          <b>Text Caret</b>
          <small>Show blinking cursor in search bar</small>
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
          <small>Search user folders and all indexed drives</small>
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
          <small>Ignore leading prefix tokens like "et" when searching</small>
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
    {:else if activeCategory === "features"}
      <div class="srow">
        <div>
          <b>Clipboard History</b>
          <small>Monitor, store and search recent clipboard entries</small>
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
        <!-- Exclusion Filter Mode -->
        <div class="srow">
          <div>
            <b>Clipboard Filter Mode</b>
            <small>
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
            <small>API keys, private keys, passwords, auth tokens</small>
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
            <small>Exclude camelCase code words and variables</small>
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
            <small>Match Visa, MasterCard, Amex, and Discover patterns</small>
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
            <small>Add custom regex or literal strings to exclude</small>
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
                class="filter-text-input"
                placeholder="Filter Name (optional)"
                bind:value={customNameInput}
              />
              <input
                type="text"
                class="filter-text-input flex-1"
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
                class="filter-text-input flex-1"
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
              <small>{filteredHistory.length} hidden filtered entries safely stored</small>
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
      {/if}
      <div class="srow">
        <div>
          <b>Preview Panel</b>
          <small>Show details and metadata for selected item</small>
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
          <small>Show progress indicator while background indexing</small>
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
    {:else if activeCategory === "shortcuts"}
      <div class="srow">
        <div>
          <b>Launcher Shortcut</b>
          <small>Global hotkey to show or hide the command launcher</small>
        </div>
        <input
          class="shortcut-input"
          bind:value={launcherShortcutInput}
          onblur={applyShortcuts}
          onkeydown={(e) => {
            if (e.key === "Enter") applyShortcuts();
          }}
          placeholder="e.g. Alt+Space"
        />
      </div>
      <div class="srow">
        <div>
          <b>Clipboard History Shortcut</b>
          <small>
            {#if !featuresConfig.clipboardHistory}
              <span style="color: var(--mut);">Inactive (Clipboard History is disabled)</span>
            {:else}
              Global hotkey to open clipboard history directly
            {/if}
          </small>
        </div>
        <input
          class="shortcut-input"
          bind:value={clipboardShortcutInput}
          disabled={!featuresConfig.clipboardHistory}
          onblur={applyShortcuts}
          onkeydown={(e) => {
            if (e.key === "Enter") applyShortcuts();
          }}
          placeholder="e.g. Alt+Shift+V"
        />
      </div>

      {#if !featuresConfig.clipboardHistory}
        <div class="shortcut-status-banner">
          ℹ️ Clipboard History is disabled in Features. Its global keyboard shortcut is inactive.
        </div>
      {/if}

      {#if (launcherShortcutInput.toLowerCase().includes("win") && launcherShortcutInput.toLowerCase().includes("v")) || (clipboardShortcutInput.toLowerCase().includes("win") && clipboardShortcutInput.toLowerCase().includes("v"))}
        <div class="shortcut-win-v-notice">
          ⚠️ <strong>Win + V</strong> is reserved exclusively by Windows for its native clipboard manager and cannot be registered safely by third-party desktop applications. Use <strong>Alt + Shift + V</strong> or another modifier combination.
        </div>
      {/if}

      {#if shortcutMessage}
        <div class="shortcut-status-banner" class:error={shortcutIsError}>
          {shortcutMessage}
        </div>
      {/if}

      <div class="srow">
        <div>
          <b>Apply Changes</b>
          <small>Re-register global shortcuts at runtime</small>
        </div>
        <button
          class="btn-sub"
          type="button"
          onclick={applyShortcuts}
        >
          Save Shortcuts
        </button>
      </div>

      <div class="srow storage-row">
        <div>
          <b>Clipboard History Storage</b>
          <small>{clipboardCount} {clipboardCount === 1 ? "entry" : "entries"} stored locally</small>
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
  </div>
</div>

<style>
  .settings-win {
    width: 100vw;
    height: 100vh;
    box-sizing: border-box;
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
    padding: 12px 16px;
    font-weight: 600;
    font-size: 13px;
    border-bottom: 1px solid var(--line);
    background: var(--chip);
    flex-shrink: 0;
  }

  .stb-title {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--fg);
  }

  .logo {
    width: 22px;
    height: 22px;
    border-radius: 6px;
    background: var(--fg);
    color: var(--bg);
    display: grid;
    place-items: center;
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
    transition: all 0.15s ease;
  }

  .stb button:hover {
    background: var(--sel);
    color: var(--fg);
  }

  .set-tabs {
    display: flex;
    gap: 4px;
    padding: 8px 12px;
    background: var(--chip);
    border-bottom: 1px solid var(--line);
    flex-shrink: 0;
  }

  .set-tabs button {
    flex: 1;
    border: 0;
    background: none;
    color: var(--mut);
    font:
      500 12px Inter,
      "Segoe UI",
      system-ui,
      sans-serif;
    padding: 6px 8px;
    border-radius: 6px;
    cursor: pointer;
    white-space: nowrap;
    text-align: center;
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
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 12px 16px;
    scrollbar-width: thin;
  }

  .srow {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 4px;
    border-bottom: 1px solid var(--line);
    gap: 16px;
  }

  .srow:last-child {
    border-bottom: 0;
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
  }

  .srow small {
    display: block;
    color: var(--dim);
    font-size: 11.5px;
    margin-top: 2px;
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
  }

  .seg button {
    border: 0;
    background: none;
    font:
      500 12px Inter,
      "Segoe UI",
      system-ui,
      sans-serif;
    color: var(--mut);
    padding: 4px 10px;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s ease;
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
    padding: 6px 12px;
    border-radius: 6px;
    cursor: pointer;
    flex-shrink: 0;
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

  .shortcut-input {
    background: var(--chip);
    border: 1px solid var(--line);
    border-radius: 6px;
    color: var(--fg);
    font-family: monospace;
    font-size: 12px;
    padding: 6px 10px;
    width: 140px;
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
  }

  .shortcut-status-banner {
    font-size: 12px;
    color: #34c759;
    background: rgba(52, 199, 89, 0.1);
    border: 1px solid rgba(52, 199, 89, 0.25);
    border-radius: 6px;
    padding: 6px 10px;
    margin: 8px 0;
  }

  .shortcut-status-banner.error {
    color: #e5484d;
    background: rgba(229, 72, 77, 0.1);
    border-color: rgba(229, 72, 77, 0.25);
  }

  .k {
    display: inline-flex;
    gap: 4px;
    align-items: center;
  }

  kbd {
    background: var(--kbd);
    color: var(--mut);
    font:
      500 11px Inter,
      "Segoe UI",
      system-ui,
      sans-serif;
    padding: 2px 6px;
    border-radius: 4px;
    box-shadow: inset 0 -1px 0 var(--line);
  }

  .prefix-container {
    padding: 8px 4px 12px;
    border-bottom: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .prefix-list {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
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
    width: 160px;
  }

  .prefix-input:focus {
    border-color: var(--acc, #e5484d);
  }

  .filter-section {
    padding: 12px 4px;
    border-bottom: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .filter-section-header b {
    display: block;
    font-size: 13px;
    font-weight: 500;
    color: var(--fg);
  }

  .filter-section-header small {
    display: block;
    color: var(--dim);
    font-size: 11.5px;
    margin-top: 2px;
  }

  .custom-filters-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .custom-filter-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: var(--chip);
    border: 1px solid var(--line);
    border-radius: 6px;
    padding: 6px 10px;
  }

  .custom-filter-info {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    overflow: hidden;
  }

  .custom-filter-name {
    font-size: 12px;
    font-weight: 500;
    color: var(--fg);
  }

  .custom-filter-pattern {
    font-family: monospace;
    font-size: 11.5px;
    color: var(--dim);
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .custom-filter-badge {
    font-size: 10px;
    background: var(--kbd);
    color: var(--mut);
    padding: 1px 5px;
    border-radius: 4px;
    text-transform: uppercase;
  }

  .custom-filter-actions {
    display: flex;
    align-items: center;
    gap: 8px;
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
  }

  .filter-inputs-row,
  .filter-test-row {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .flex-1 {
    flex: 1;
  }

  .filter-text-input {
    background: var(--chip);
    border: 1px solid var(--line);
    border-radius: 6px;
    color: var(--fg);
    font-size: 12px;
    padding: 6px 10px;
    outline: none;
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
    padding: 10px 4px;
    border-bottom: 1px solid var(--line);
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
    max-height: 240px;
    overflow-y: auto;
  }

  .safe-inspection-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: var(--chip);
    border: 1px solid var(--line);
    border-radius: 6px;
    padding: 8px 10px;
    gap: 12px;
  }

  .safe-inspection-details {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 0;
    flex: 1;
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
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .safe-masked-text {
    letter-spacing: 2px;
    color: var(--mut);
  }

  .safe-revealed-text {
    color: var(--fg);
  }

  .safe-inspection-actions {
    display: flex;
    gap: 6px;
    flex-shrink: 0;
  }

  .btn-sm {
    padding: 3px 8px;
    font-size: 11px;
  }

  /* Minimal Style */
  :global(:root[data-style="minimal"]) .srow {
    padding: 8px 4px;
    gap: 10px;
  }

  :global(:root[data-style="minimal"]) .stb {
    height: 38px;
    padding: 0 14px;
  }

  :global(:root[data-style="minimal"]) .set-tabs button {
    padding: 5px 10px;
    font-size: 12px;
  }

  /* Detailed Style */
  :global(:root[data-style="detailed"]) .srow {
    padding: 16px 6px;
    gap: 20px;
  }

  :global(:root[data-style="detailed"]) .srow b {
    font-size: 14px;
  }

  :global(:root[data-style="detailed"]) .srow small {
    font-size: 12.5px;
  }

  :global(:root[data-style="detailed"]) .stb {
    height: 52px;
    padding: 0 20px;
  }

  :global(:root[data-style="detailed"]) .set-tabs button {
    padding: 8px 16px;
    font-size: 13.5px;
  }
</style>
