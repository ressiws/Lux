<script lang="ts">
  import { onMount } from "svelte";
  import {
    clearClipboardHistory,
    getClipboardHistory,
    getIndexStats,
    getShortcutsStatus,
    hideSettings,
    onIndexingStatus,
    onOpenSettings,
    reindex,
    updateShortcuts,
  } from "./tauri";
  import {
    applyThemeAndCaret,
    loadSettings,
    saveSettings,
    type SettingsCategory,
    type SettingsState,
  } from "./settings";
  import type { IndexStats, ShortcutConfigStatus } from "./types";

  let activeCategory = $state<SettingsCategory>("general");
  let settings = $state<SettingsState>(loadSettings());

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
    void loadShortcutsStatus();

    void getIndexStats()
      .then((stats) => {
        indexStats = stats;
      })
      .catch(() => {});

    let unlistenIndexing: (() => void) | undefined;
    let unlistenOpen: (() => void) | undefined;

    void onOpenSettings(() => {
      settings = loadSettings();
      applyThemeAndCaret(settings);
      void loadShortcutsStatus();
      void getIndexStats()
        .then((stats) => {
          indexStats = stats;
        })
        .catch(() => {});
    }).then((unlisten) => {
      unlistenOpen = unlisten;
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
    {:else if activeCategory === "features"}
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
          <small>Global hotkey to open clipboard history directly</small>
        </div>
        <input
          class="shortcut-input"
          bind:value={clipboardShortcutInput}
          onblur={applyShortcuts}
          onkeydown={(e) => {
            if (e.key === "Enter") applyShortcuts();
          }}
          placeholder="e.g. Alt+Shift+V"
        />
      </div>

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

  .seg button[aria-pressed="true"] {
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
</style>
