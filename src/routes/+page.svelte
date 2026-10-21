<script lang="ts">
  import { onMount, tick } from "svelte";
  import ClipboardView from "$lib/ClipboardView.svelte";
  import {
    CATEGORY_ICONS,
    buildSections,
    hasOnlyFallbackSections,
    splitHighlight,
  } from "$lib/mock-data";
  import {
    getFeaturesConfig,
    getIndexStats,
    getLauncherStatus,
    hideLauncher,
    notifyFrontendReady,
    onFeaturesChanged,
    onIndexingStatus,
    onLauncherHidden,
    onLauncherShown,
    onOpenClipboard,
    onOpenLauncher,
    openSettings,
    openTarget,
    openTargetAsAdmin,
    reindex,
    revealItemInFolder,
    searchLauncher,
    setLauncherHeight,
    setWindowHeight,
  } from "$lib/tauri";
  import SettingsView from "$lib/SettingsView.svelte";
  import {
    applyThemeAndCaret,
    loadSettings,
    onSettingsChanged,
    type SettingsState,
  } from "$lib/settings";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { resolvePrefix } from "$lib/prefix";
  import type {
    FeaturesConfig,
    IndexStats,
    IndexingStatusPayload,
    ItemAction,
    LauncherItem,
    SearchResponse,
  } from "$lib/types";

  function getWindowLabel(): string {
    try {
      return getCurrentWindow().label;
    } catch {
      return "main";
    }
  }

  const windowLabel = getWindowLabel();

  let query = $state("");
  let selectedIndex = $state(0);
  let selectedActionIndex = $state(0);
  let actionsOpen = $state(false);
  let feedbackMessage = $state("");
  let shortcutWarning = $state("");
  let searchResponse = $state<SearchResponse | undefined>();
  let windowExpanded = $state(false);
  let isIndexing = $state(false);
  let indexStats = $state<IndexStats>({
    totalApps: 0,
    totalFiles: 0,
    isIndexed: false,
  });
  let appMode = $state<"launcher" | "clipboard">("launcher");
  let featuresConfig = $state<FeaturesConfig>({ clipboardHistory: true });

  let settings = $state<SettingsState>(loadSettings());

  let searchInput: HTMLInputElement | undefined = $state();
  let listContainer: HTMLDivElement | undefined = $state();
  let feedbackTimer: ReturnType<typeof setTimeout> | undefined;
  let searchSeq = 0;
  let searchDebounceTimer: ReturnType<typeof setTimeout> | undefined;
  let searchProgressTimer: ReturnType<typeof setTimeout> | undefined;
  let isSearching = $state(false);
  let searchBarVisible = $state(false);

  const prefixResolution = $derived(
    resolvePrefix(
      query,
      settings.flowPrefixCompatibility,
      settings.flowPrefixes,
    ),
  );
  const effectiveQuery = $derived(prefixResolution.effectiveQuery);
  const isIdle = $derived(effectiveQuery.trim().length === 0);
  const sections = $derived(
    buildSections(effectiveQuery, searchResponse, {
      enableFileSearch: settings.fileSearch,
    }),
  );
  const flatItems = $derived(sections.flatMap((section) => section.items));
  const selectedItem = $derived<LauncherItem | undefined>(
    flatItems[selectedIndex],
  );
  const showPreview = $derived(
    Boolean(
      selectedItem &&
        settings.preview &&
        selectedItem.category !== "calc" &&
        selectedItem.category !== "fb" &&
        !isIdle,
    ),
  );
  const showEmptyState = $derived(
    effectiveQuery.trim().length > 0 && hasOnlyFallbackSections(sections),
  );
  const displayedMessage = $derived(feedbackMessage || shortcutWarning);

  let isCollapsing = $state(false);
  let collapseTimer: ReturnType<typeof setTimeout> | undefined;

  const shouldExpand = $derived(!isIdle);

  async function syncWindowHeight(expanded: boolean): Promise<void> {
    if (windowExpanded === expanded) return;
    windowExpanded = expanded;
    try {
      await setLauncherHeight(expanded);
    } catch {
      // Ignore if outside Tauri
    }
  }

  async function switchMode(mode: "launcher" | "clipboard"): Promise<void> {
    if (mode === "clipboard" && !featuresConfig.clipboardHistory) {
      say("Clipboard History is disabled in Settings");
      return;
    }
    appMode = mode;
    if (mode === "clipboard") {
      actionsOpen = false;
      try {
        await setWindowHeight(560);
      } catch {
        // Ignore outside Tauri
      }
    } else {
      await syncWindowHeight(shouldExpand);
      focusSearchInput();
    }
  }


  $effect(() => {
    if (appMode !== "launcher") return;
    const expand = shouldExpand;
    if (!expand && windowExpanded) {
      isCollapsing = true;
      clearTimeout(collapseTimer);
      collapseTimer = setTimeout(() => {
        isCollapsing = false;
      }, 180);
    } else if (expand) {
      isCollapsing = false;
      clearTimeout(collapseTimer);
    }
    void syncWindowHeight(expand);
  });

  async function performSearch(q: string): Promise<void> {
    const seq = ++searchSeq;
    const trimmed = q.trim();
    if (trimmed.length === 0) {
      isSearching = false;
      searchBarVisible = false;
      clearTimeout(searchProgressTimer);
      searchResponse = undefined;
      return;
    }

    isSearching = true;
    clearTimeout(searchProgressTimer);
    // Avoid flash for instantaneous in-memory searches: show indicator if query takes > 35ms
    searchProgressTimer = setTimeout(() => {
      if (isSearching && seq === searchSeq) {
        searchBarVisible = true;
      }
    }, 35);

    try {
      const resp = await searchLauncher(trimmed);
      if (seq === searchSeq) {
        searchResponse = resp;
      }
    } catch {
      // Ignore if Tauri invocation is unavailable
    } finally {
      if (seq === searchSeq) {
        isSearching = false;
        searchBarVisible = false;
        clearTimeout(searchProgressTimer);
      }
    }
  }

  function focusSearchInput(): void {
    searchInput?.focus();
    requestAnimationFrame(() => {
      searchInput?.focus();
    });
  }

  function setQuery(val: string): void {
    query = val;
    selectedIndex = 0;
    selectedActionIndex = 0;
    actionsOpen = false;
    clearTimeout(searchDebounceTimer);
    clearTimeout(searchProgressTimer);
    const resolved = resolvePrefix(
      val,
      settings.flowPrefixCompatibility,
      settings.flowPrefixes,
    ).effectiveQuery;
    void performSearch(resolved);
    focusSearchInput();
  }

  function resetLauncherState(shouldFocus: boolean): void {
    actionsOpen = false;
    selectedActionIndex = 0;
    query = "";
    selectedIndex = 0;
    searchSeq++;
    isSearching = false;
    searchBarVisible = false;
    searchResponse = undefined;
    clearTimeout(feedbackTimer);
    clearTimeout(collapseTimer);
    clearTimeout(searchDebounceTimer);
    clearTimeout(searchProgressTimer);
    isCollapsing = false;
    feedbackMessage = "";
    void syncWindowHeight(false);
    if (shouldFocus) {
      focusSearchInput();
    } else {
      searchInput?.blur();
    }
  }

  function say(text: string): void {
    feedbackMessage = text;
    clearTimeout(feedbackTimer);
    feedbackTimer = setTimeout(() => {
      feedbackMessage = "";
    }, 1600);
  }

  async function copyTextToClipboard(text: string): Promise<boolean> {
    if (typeof navigator !== "undefined" && navigator.clipboard?.writeText) {
      try {
        await navigator.clipboard.writeText(text);
        return true;
      } catch {
        // Fall through to DOM copy fallback if clipboard API permission fails.
      }
    }

    if (typeof document === "undefined") {
      return false;
    }

    const textarea = document.createElement("textarea");
    textarea.value = text;
    textarea.setAttribute("readonly", "");
    textarea.style.position = "fixed";
    textarea.style.opacity = "0";
    textarea.style.pointerEvents = "none";
    document.body.appendChild(textarea);

    try {
      textarea.select();
      return document.execCommand("copy");
    } catch {
      return false;
    } finally {
      document.body.removeChild(textarea);
      focusSearchInput();
    }
  }

  async function handleOpenSettings(): Promise<void> {
    actionsOpen = false;
    try {
      await openSettings();
    } catch {
      // Ignore outside Tauri
    }
  }

  async function triggerReindex(): Promise<void> {
    isIndexing = true;
    say("Indexing applications and files…");
    try {
      await reindex();
    } catch (err) {
      say(`Failed to index: ${err}`);
      isIndexing = false;
    }
  }

  async function copyAndAnnounce(value: string, label: string): Promise<void> {
    const copied = await copyTextToClipboard(value);
    say(copied ? `Copied ${label}` : `Unable to copy ${label}`);
  }

  async function runSelected(): Promise<void> {
    const item = selectedItem;
    if (!item) {
      return;
    }
    if (item.category === "calc" && item.calc) {
      await copyAndAnnounce(item.calc.raw, item.calc.raw);
      return;
    }
    if (item.name === "Open Lux Settings" || (item.category === "cmd" && item.name.toLowerCase().includes("settings"))) {
      void handleOpenSettings();
      return;
    }
    if (item.category === "clip" || item.name.toLowerCase().includes("clipboard")) {
      if (!featuresConfig.clipboardHistory) {
        say("Clipboard History is disabled in Settings");
        return;
      }
      void switchMode("clipboard");
      return;
    }
    if (item.category === "snip") {
      say("Pasted");
      return;
    }
    if (item.category === "fb") {
      say("Searching…");
      return;
    }
    if (item.path) {
      try {
        await openTarget(item.path);
        await dismissLauncher();
        return;
      } catch (err) {
        say(`Failed to open: ${err}`);
        return;
      }
    }
    say(`Opened ${item.name}`);
  }

  function openActionsPopover(): void {
    if (!selectedItem) {
      actionsOpen = false;
      selectedActionIndex = 0;
      return;
    }
    selectedActionIndex = 0;
    actionsOpen = true;
    focusSearchInput();
  }

  function closeActionsPopover(): void {
    actionsOpen = false;
    selectedActionIndex = 0;
    focusSearchInput();
  }

  async function executeItemAction(
    item: LauncherItem,
    action?: ItemAction,
  ): Promise<void> {
    closeActionsPopover();

    if (!action || action.label === item.primaryLabel) {
      await runSelected();
      return;
    }

    if (item.category === "calc" && item.calc) {
      if (action.label === "Copy Expression") {
        await copyAndAnnounce(item.calc.expression, item.calc.expression);
        return;
      }
      if (action.label === "Paste Result to App") {
        await copyAndAnnounce(item.calc.raw, item.calc.raw);
        return;
      }
    }

    if (action.label === "Reveal in Folder" || action.label === "Show in Files") {
      if (item.path) {
        try {
          await revealItemInFolder(item.path);
          await dismissLauncher();
          return;
        } catch (err) {
          say(`Failed to reveal: ${err}`);
          return;
        }
      }
    }

    if (action.label === "Copy Path" || action.label === "Copy File Path") {
      if (item.path) {
        await copyAndAnnounce(item.path, "Path");
        return;
      }
    }

    if (action.label === "Run as Administrator") {
      if (item.path) {
        try {
          await openTargetAsAdmin(item.path);
          await dismissLauncher();
          return;
        } catch (err) {
          say(`Failed to run as admin: ${err}`);
          return;
        }
      }
    }

    say(action.label);
  }

  async function dismissLauncher(): Promise<void> {
    resetLauncherState(false);
    try {
      await hideLauncher();
    } catch {
      say("Unable to hide launcher window");
    }
  }

  async function scrollSelectedIntoView(): Promise<void> {
    await tick();
    const activeRow = listContainer?.querySelector<HTMLElement>(
      `.row-item[data-i="${selectedIndex}"]`,
    );
    if (activeRow) {
      activeRow.scrollIntoView({ block: "nearest" });
    }
  }

  function handleQueryInput(): void {
    selectedIndex = 0;
    selectedActionIndex = 0;
    actionsOpen = false;
    clearTimeout(searchDebounceTimer);
    clearTimeout(searchProgressTimer);

    const q = effectiveQuery;
    if (q.trim().length === 0) {
      isSearching = false;
      searchBarVisible = false;
      searchResponse = undefined;
      searchSeq++;
      return;
    }

    // 180ms debounce after the last keystroke
    searchDebounceTimer = setTimeout(() => {
      void performSearch(q);
    }, 180);
  }

  function nav(d: number): number {
    const n = flatItems.length;
    if (!n) return 0;
    if (
      settings.layout === "grid" &&
      selectedItem &&
      (selectedItem.category === "app" || selectedItem.category === "file")
    ) {
      const currentCategory = selectedItem.category;
      const firstIndex = flatItems.findIndex(
        (x) => x.category === currentCategory,
      );
      let lastIndex = -1;
      for (let i = flatItems.length - 1; i >= 0; i--) {
        if (flatItems[i].category === currentCategory) {
          lastIndex = i;
          break;
        }
      }
      const target = selectedIndex + d * 4;
      if (target >= firstIndex && target <= lastIndex) {
        return target;
      }
      if (
        d > 0 &&
        Math.floor((selectedIndex - firstIndex) / 4) <
          Math.floor((lastIndex - firstIndex) / 4)
      ) {
        return lastIndex;
      }
      if (d > 0) {
        return lastIndex + 1 < n ? lastIndex + 1 : selectedIndex;
      }
      return firstIndex > 0 ? firstIndex - 1 : selectedIndex;
    }
    return (selectedIndex + d + n) % n;
  }

  function handleKeydown(event: KeyboardEvent): void {
    const count = flatItems.length;

    if (
      (event.ctrlKey || event.metaKey) &&
      event.shiftKey &&
      event.key.toLowerCase() === "c" &&
      selectedItem?.category === "calc" &&
      selectedItem.calc
    ) {
      event.preventDefault();
      event.stopPropagation();
      closeActionsPopover();
      void copyAndAnnounce(
        selectedItem.calc.expression,
        selectedItem.calc.expression,
      );
      return;
    }

    if (
      (event.ctrlKey || event.metaKey) &&
      event.shiftKey &&
      event.key.toLowerCase() === "c" &&
      selectedItem?.path
    ) {
      event.preventDefault();
      event.stopPropagation();
      closeActionsPopover();
      void copyAndAnnounce(selectedItem.path, "Path");
      return;
    }

    if (actionsOpen && selectedItem) {
      const actionCount = selectedItem.actions.length;

      if (event.key === "ArrowDown") {
        event.preventDefault();
        event.stopPropagation();
        if (actionCount > 0) {
          selectedActionIndex = (selectedActionIndex + 1) % actionCount;
        }
        return;
      }

      if (event.key === "ArrowUp") {
        event.preventDefault();
        event.stopPropagation();
        if (actionCount > 0) {
          selectedActionIndex =
            (selectedActionIndex - 1 + actionCount) % actionCount;
        }
        return;
      }

      if (event.key === "Enter") {
        event.preventDefault();
        event.stopPropagation();
        const chosenAction =
          selectedItem.actions[selectedActionIndex] ?? selectedItem.actions[0];
        void executeItemAction(selectedItem, chosenAction);
        return;
      }

      if (
        event.key === "Escape" ||
        event.key === "Tab" ||
        ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k")
      ) {
        event.preventDefault();
        event.stopPropagation();
        closeActionsPopover();
        return;
      }
    }

    if (
      (event.ctrlKey || event.metaKey) &&
      event.shiftKey &&
      event.key === "Enter" &&
      selectedItem?.category === "app" &&
      selectedItem.path
    ) {
      event.preventDefault();
      try {
        void openTargetAsAdmin(selectedItem.path);
        void dismissLauncher();
      } catch (err) {
        say(`Failed to run as admin: ${err}`);
      }
      return;
    }

    if (
      (event.ctrlKey || event.metaKey) &&
      !event.shiftKey &&
      event.key === "Enter" &&
      selectedItem?.path
    ) {
      event.preventDefault();
      try {
        void revealItemInFolder(selectedItem.path);
        void dismissLauncher();
      } catch (err) {
        say(`Failed to reveal: ${err}`);
      }
      return;
    }

    if (event.key === "ArrowDown") {
      event.preventDefault();
      selectedIndex = nav(1);
      void scrollSelectedIntoView();
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      selectedIndex = nav(-1);
      void scrollSelectedIntoView();
      return;
    }

    if (
      (event.key === "ArrowRight" || event.key === "ArrowLeft") &&
      settings.layout === "grid" &&
      selectedItem &&
      (selectedItem.category === "app" || selectedItem.category === "file")
    ) {
      event.preventDefault();
      selectedIndex =
        event.key === "ArrowRight"
          ? Math.min(count - 1, selectedIndex + 1)
          : Math.max(0, selectedIndex - 1);
      void scrollSelectedIntoView();
      return;
    }

    if (event.key === "Enter") {
      event.preventDefault();
      void runSelected();
      return;
    }

    if (
      event.key === "Tab" ||
      ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k")
    ) {
      event.preventDefault();
      openActionsPopover();
      return;
    }

    if (event.key === "Escape") {
      event.preventDefault();
      if (query.trim().length > 0) {
        query = "";
        selectedIndex = 0;
        void performSearch("");
      } else {
        void dismissLauncher();
      }
    }
  }

  function handleRowHover(index: number): void {
    if (actionsOpen) {
      return;
    }
    if (selectedIndex !== index) {
      selectedIndex = index;
      selectedActionIndex = 0;
    }
  }

  function handleRowClick(index: number): void {
    if (actionsOpen) {
      closeActionsPopover();
    }
    selectedIndex = index;
    void runSelected();
  }

  function handleActionHover(index: number): void {
    if (selectedActionIndex !== index) {
      selectedActionIndex = index;
    }
  }

  function handleActionClick(index: number): void {
    if (!selectedItem) {
      return;
    }
    selectedActionIndex = index;
    void executeItemAction(selectedItem, selectedItem.actions[index]);
  }

  function toggleActionsPopover(): void {
    if (!selectedItem) {
      closeActionsPopover();
      return;
    }
    if (actionsOpen) {
      closeActionsPopover();
    } else {
      openActionsPopover();
    }
  }

  function preventFocusLossOnMouseDown(event: MouseEvent): void {
    event.preventDefault();
  }

  function isGradientThumbnail(item: LauncherItem): boolean {
    return (
      item.category !== "clip" &&
      typeof item.thumbnail === "string" &&
      item.thumbnail.includes("gradient")
    );
  }

  function isClipThumbnail(item: LauncherItem): boolean {
    return item.category === "clip" && typeof item.thumbnail === "string";
  }

  function getCategoryIconPath(item: LauncherItem): string {
    if (item.category === "file" && item.isImage) {
      return CATEGORY_ICONS.image ?? CATEGORY_ICONS.file;
    }
    if (item.rightLabel === "Folder" && CATEGORY_ICONS.folder) {
      return CATEGORY_ICONS.folder;
    }
    return CATEGORY_ICONS[item.category] ?? CATEGORY_ICONS.file;
  }

  function getMetadata(item: LauncherItem): Array<[string, string]> {
    if (item.category === "app") {
      const location = item.location || item.path || "—";
      return [
        ["Kind", "Application"],
        ["Location", location],
        ["Version", item.version || "—"],
        ["Size", item.size || "—"],
      ];
    }
    if (item.category === "file") {
      const kind = item.isImage
        ? "Image"
        : item.rightLabel && item.rightLabel !== "File"
          ? item.rightLabel
          : "File";
      const location = item.location || item.path || "—";
      return [
        ["Kind", kind],
        ["Location", location],
        ["Size", item.size || "—"],
      ];
    }
    if (item.category === "calc" && item.calc) {
      return [
        ["Kind", "Calculation"],
        ["Expression", item.calc.expression],
        ["Result", item.calc.value],
      ];
    }
    return [];
  }

  onMount(() => {
    applyThemeAndCaret(settings);
    focusSearchInput();
    void performSearch("");
    void notifyFrontendReady().catch(() => {});

    void getIndexStats()
      .then((stats) => {
        indexStats = stats;
      })
      .catch(() => {});

    let disposed = false;
    let unlistenShown: (() => void) | undefined;
    let unlistenHidden: (() => void) | undefined;
    let unlistenIndexing: (() => void) | undefined;
    let unlistenOpenLauncher: (() => void) | undefined;
    let unlistenOpenClipboard: (() => void) | undefined;
    let unlistenSettings: (() => void) | undefined;
    let unlistenFeatures: (() => void) | undefined;

    void getFeaturesConfig()
      .then((config) => {
        if (!disposed) {
          featuresConfig = config;
        }
      })
      .catch(() => {});

    void onFeaturesChanged((newFeatures) => {
      featuresConfig = newFeatures;
      if (!newFeatures.clipboardHistory && appMode === "clipboard") {
        void switchMode("launcher");
        say("Clipboard History is disabled in Settings");
      }
    }).then((unlisten) => {
      if (disposed) {
        unlisten();
      } else {
        unlistenFeatures = unlisten;
      }
    });

    void onSettingsChanged((newSettings) => {
      settings = newSettings;
      applyThemeAndCaret(settings);
    }).then((unlisten) => {
      if (disposed) {
        unlisten();
      } else {
        unlistenSettings = unlisten;
      }
    });

    void onOpenLauncher(() => {
      actionsOpen = false;
      void switchMode("launcher");
    }).then((unlisten) => {
      if (disposed) {
        unlisten();
      } else {
        unlistenOpenLauncher = unlisten;
      }
    });

    void onOpenClipboard(() => {
      if (!featuresConfig.clipboardHistory) {
        say("Clipboard History is disabled in Settings");
        return;
      }
      void switchMode("clipboard");
    }).then((unlisten) => {
      if (disposed) {
        unlisten();
      } else {
        unlistenOpenClipboard = unlisten;
      }
    });

    void getLauncherStatus()
      .then((status) => {
        if (disposed) {
          return;
        }
        if (!status.shortcut.registered) {
          shortcutWarning = `Shortcut ${status.shortcut.shortcut} unavailable`;
        }
      })
      .catch(() => {});

    void onIndexingStatus((payload) => {
      isIndexing = payload.isIndexing;
      if (!payload.isIndexing) {
        indexStats = {
          totalApps: payload.totalApps,
          totalFiles: payload.totalFiles,
          isIndexed: true,
        };
        void performSearch(query);
      }
    }).then((unlisten) => {
      if (disposed) {
        unlisten();
      } else {
        unlistenIndexing = unlisten;
      }
    });

    void onLauncherShown(() => {
      if (appMode === "launcher") {
        resetLauncherState(true);
      }
    }).then((unlisten) => {
      if (disposed) {
        unlisten();
      } else {
        unlistenShown = unlisten;
      }
    });

    void onLauncherHidden(() => {
      resetLauncherState(false);
      appMode = "launcher";
    }).then((unlisten) => {
      if (disposed) {
        unlisten();
      } else {
        unlistenHidden = unlisten;
      }
    });

    const handleWindowFocus = (): void => {
      if (appMode === "launcher") {
        focusSearchInput();
      }
    };

    window.addEventListener("focus", handleWindowFocus);
    window.addEventListener("keydown", handleKeydown);

    return () => {
      disposed = true;
      clearTimeout(feedbackTimer);
      clearTimeout(searchDebounceTimer);
      clearTimeout(searchProgressTimer);
      unlistenShown?.();
      unlistenHidden?.();
      unlistenIndexing?.();
      unlistenOpenLauncher?.();
      unlistenOpenClipboard?.();
      unlistenSettings?.();
      unlistenFeatures?.();
      window.removeEventListener("focus", handleWindowFocus);
      window.removeEventListener("keydown", handleKeydown);
    };
  });
</script>

{#if windowLabel === "settings"}
  <SettingsView />
{:else if appMode === "launcher"}
<div
  class="win"
  id="win"
  class:idle={isIdle}
  class:collapsing={isCollapsing}
  class:nopv={!showPreview}
  role="dialog"
  aria-label="Lux command palette"
>
  <div class="bar" id="bar">
    <svg
      width="22"
      height="22"
      viewBox="0 0 20 20"
      fill="none"
      stroke="currentColor"
      stroke-width="1.6"
      stroke-linecap="round"
      aria-hidden="true"
    >
      <circle cx="9" cy="9" r="5.5" />
      <path d="m13.2 13.2 3.3 3.3" />
    </svg>
    {#if prefixResolution.hasPrefix && prefixResolution.prefix}
      <span
        class="prefix-pill"
        title="Prefix '{prefixResolution.prefix}' ignored for Flow Launcher compatibility"
      >
        {prefixResolution.prefix}
      </span>
    {/if}
    <input
      id="q"
      bind:this={searchInput}
      bind:value={query}
      oninput={handleQueryInput}
      autocomplete="off"
      spellcheck="false"
      placeholder="Search apps, files, commands…"
      aria-label="Search"
    />
    <div class="spin" aria-hidden="true"></div>
    <button
      class="gear"
      id="gear"
      type="button"
      aria-label="Open Lux settings"
      onmousedown={preventFocusLossOnMouseDown}
      onclick={handleOpenSettings}
    >
      <svg
        width="18"
        height="18"
        viewBox="0 0 20 20"
        fill="none"
        stroke="currentColor"
        stroke-width="1.6"
        stroke-linecap="round"
      >
        <path d="M3 6h7M14 6h3M3 14h3M10 14h7" />
        <circle cx="12" cy="6" r="2" />
        <circle cx="8" cy="14" r="2" />
      </svg>
    </button>
    {#if isIndexing && settings.progressBar}
      <div class="progress-bar indexing-bar" aria-hidden="true" title="Indexing applications and files…">
        <div class="progress-bar-fill indexing-fill"></div>
      </div>
    {/if}
    {#if searchBarVisible}
      <div class="progress-bar search-bar" aria-hidden="true" title="Searching…">
        <div class="progress-bar-fill search-fill"></div>
      </div>
    {/if}
  </div>

  <div class="hints" id="hints">
    <div class="hr">
      Try
      <button
        type="button"
        onmousedown={preventFocusLossOnMouseDown}
        onclick={() => setQuery("Firefox")}
      >
        Firefox
      </button>
      <button
        type="button"
        onmousedown={preventFocusLossOnMouseDown}
        onclick={() => setQuery("25 EUR to USD")}
      >
        25 EUR to USD
      </button>
    </div>
    <div class="hk">
      <span><kbd>↑</kbd><kbd>↓</kbd> navigate</span>
      <span><kbd>↵</kbd> open</span>
      <span><kbd>Ctrl</kbd><kbd>K</kbd> actions</span>
      <span><kbd>Esc</kbd> close</span>
    </div>
  </div>

  <div class="body">
    <div id="list" role="listbox" aria-label="Results" bind:this={listContainer}>
      {#if showEmptyState}
        <div class="empty">
          <svg
            class="big"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
            aria-hidden="true"
          >
            <circle cx="11" cy="11" r="6.5" />
            <path d="m16 16 4 4M9 11h4" />
          </svg>
          <div class="t">No results for “{effectiveQuery.trim()}”</div>
          <p>Check the spelling, or try one of these.</p>
        </div>
      {/if}

      {#each sections as section (section.title)}
        {@const isGrid =
          settings.layout === "grid" &&
          (section.category === "app" || section.category === "file")}
        <div class="sec">{section.title}</div>
        {#if isGrid}
          <div class="grid">
            {#each section.items as item (item.category + ":" + (item.path || item.name))}
              {@const itemIndex = flatItems.indexOf(item)}
              {@const isSelected = itemIndex === selectedIndex}
              {@const highlighted = splitHighlight(item.name, query)}
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <div
                class="tile row-item"
                class:on={isSelected}
                data-i={itemIndex}
                role="option"
                tabindex="-1"
                aria-selected={isSelected}
                onmousedown={preventFocusLossOnMouseDown}
                onmousemove={() => handleRowHover(itemIndex)}
                onclick={() => handleRowClick(itemIndex)}
              >
                {#if item.category === "app"}
                  {#if item.icon}
                    <img class="ico-img" src={item.icon} alt="" />
                  {:else}
                    <div class="ico g" style="background: var(--chip);">
                      <svg
                        width="20"
                        height="20"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="1.7"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        aria-hidden="true"
                      >
                        <rect x="3" y="3" width="18" height="18" rx="4" />
                        <path d="M3 9h18M9 21V9" />
                      </svg>
                    </div>
                  {/if}
                {:else if item.isImage && item.thumbnail}
                  <img class="thumb" src={item.thumbnail} alt="" />
                {:else if item.icon}
                  <img class="ico-img" src={item.icon} alt="" />
                {:else}
                  <div class="ico g">
                    <svg
                      width="20"
                      height="20"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="1.7"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      aria-hidden="true"
                    >
                      <path d={getCategoryIconPath(item)} />
                    </svg>
                  </div>
                {/if}
                <span class="t">
                  {highlighted.before}{#if highlighted.match}<mark
                      >{highlighted.match}</mark
                    >{/if}{highlighted.after}
                </span>
              </div>
            {/each}
          </div>
        {:else}
          {#each section.items as item (item.category + ":" + (item.path || item.name))}
            {@const itemIndex = flatItems.indexOf(item)}
            {@const isSelected = itemIndex === selectedIndex}
            {#if item.category === "calc" && item.calc}
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <div
                class="calc row-item"
                class:on={isSelected}
                data-i={itemIndex}
                role="option"
                tabindex="-1"
                aria-selected={isSelected}
                onmousedown={preventFocusLossOnMouseDown}
                onmousemove={() => handleRowHover(itemIndex)}
                onclick={() => handleRowClick(itemIndex)}
              >
                <div class="a">
                  <small>{item.calc.expression}</small>
                  <div class="e">{item.calc.expression}</div>
                </div>
                <div class="ar" aria-hidden="true">
                  <svg
                    width="15"
                    height="15"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.7"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  >
                    <path d="M5 12h14M13 6l6 6-6 6" />
                  </svg>
                </div>
                <div class="b">
                  <small>{item.calc.label}</small>
                  <div class="v">{item.calc.value}</div>
                </div>
              </div>
            {:else}
              {@const highlighted = splitHighlight(item.name, query)}
              <!-- svelte-ignore a11y_click_events_have_key_events -->
              <div
                class="row row-item"
                class:on={isSelected}
                data-i={itemIndex}
                role="option"
                tabindex="-1"
                aria-selected={isSelected}
                onmousedown={preventFocusLossOnMouseDown}
                onmousemove={() => handleRowHover(itemIndex)}
                onclick={() => handleRowClick(itemIndex)}
              >
                {#if item.category === "app"}
                  {#if item.icon}
                    <img class="ico-img" src={item.icon} alt="" />
                  {:else}
                    <div class="ico g" style="background: var(--chip);">
                      <svg
                        width="15"
                        height="15"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="1.7"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        aria-hidden="true"
                      >
                        <rect x="3" y="3" width="18" height="18" rx="4" />
                        <path d="M3 9h18M9 21V9" />
                      </svg>
                    </div>
                  {/if}
                {:else if item.isImage && item.thumbnail}
                  <img class="thumb" src={item.thumbnail} alt="" />
                {:else if item.icon}
                  <img class="ico-img" src={item.icon} alt="" />
                {:else if isGradientThumbnail(item) || isClipThumbnail(item)}
                  <div class="thumb" style:background={item.thumbnail}></div>
                {:else}
                  <div class="ico g">
                    <svg
                      width="15"
                      height="15"
                      viewBox="0 0 24 24"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="1.7"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      aria-hidden="true"
                    >
                      <path d={getCategoryIconPath(item)} />
                    </svg>
                  </div>
                {/if}

                <div class="txt">
                  <span class="t">
                    {highlighted.before}{#if highlighted.match}<mark
                        >{highlighted.match}</mark
                      >{/if}{highlighted.after}
                  </span>
                  <span class="s">{item.subtitle}</span>
                </div>
                <span class="r">{item.rightLabel}</span>
              </div>
            {/if}
          {/each}
        {/if}
      {/each}
    </div>

    {#if showPreview && selectedItem}
      <aside id="pv" class="pv" aria-label="Preview">
        {#if selectedItem.isImage && selectedItem.thumbnail}
          <img class="pbig" src={selectedItem.thumbnail} alt="" style="object-fit: contain;" />
        {:else}
          <div class="pico">
            {#if selectedItem.category === "app"}
              {#if selectedItem.icon}
                <img
                  class="ico-img"
                  src={selectedItem.icon}
                  alt=""
                  style="width: 68px; height: 68px; border-radius: 18px; object-fit: contain;"
                />
              {:else}
                <div
                  class="ico g"
                  style="width: 68px; height: 68px; border-radius: 18px; background: var(--chip);"
                >
                  <svg
                    width="30"
                    height="30"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.7"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  >
                    <rect x="3" y="3" width="18" height="18" rx="4" />
                    <path d="M3 9h18M9 21V9" />
                  </svg>
                </div>
              {/if}
            {:else if selectedItem.icon}
              <img
                class="ico-img"
                src={selectedItem.icon}
                alt=""
                style="width: 68px; height: 68px; border-radius: 18px; object-fit: contain;"
              />
            {:else}
              <div
                class="ico g"
                style="width: 68px; height: 68px; border-radius: 18px;"
              >
                <svg
                  width="30"
                  height="30"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="1.7"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                >
                  <path d={getCategoryIconPath(selectedItem)} />
                </svg>
              </div>
            {/if}
          </div>
        {/if}
        <div class="pt">{selectedItem.name}</div>
        {#each getMetadata(selectedItem) as [k, v] (k)}
          <div class="pm"><span>{k}</span><span>{v}</span></div>
        {/each}
      </aside>
    {/if}
  </div>

  <div class="foot">
    <div class="logo" aria-label="Lux">
      <svg viewBox="0 0 64 64" width="12" height="12" fill="currentColor">
        <path d="M 10 8 H 22 V 40 L 46 16 H 58 L 34 40 H 58 V 52 H 10 Z" />
      </svg>
    </div>
    <span id="msg">{displayedMessage}</span>
    <span class="sp"></span>
    <div
      class="g1"
      id="gp"
      style:visibility={selectedItem ? "visible" : "hidden"}
    >
      <button
        class="btn"
        id="go"
        type="button"
        onmousedown={preventFocusLossOnMouseDown}
        onclick={runSelected}
      >
        <span id="pl"
          >{selectedItem ? selectedItem.primaryLabel : "Open Application"}</span
        >
        <span class="k"><kbd>↵</kbd></span>
      </button>
    </div>
    <span
      class="vd"
      id="vd"
      aria-hidden="true"
      style:visibility={selectedItem ? "visible" : "hidden"}
    ></span>
    <div
      class="g2"
      id="gs"
      style:visibility={selectedItem ? "visible" : "hidden"}
    >
      <button
        class="btn"
        id="ab"
        type="button"
        onmousedown={preventFocusLossOnMouseDown}
        onclick={toggleActionsPopover}
      >
        <span>Actions</span>
        <span class="k"><kbd>Ctrl</kbd><kbd>K</kbd></span>
      </button>
    </div>
  </div>

  <div class="pop" id="pop" class:open={actionsOpen && selectedItem !== undefined}>
    {#if selectedItem}
      <h6>{selectedItem.name.slice(0, 34)}</h6>
      {#each selectedItem.actions as action, actionIndex (action.label)}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div
          class="row"
          class:dan={action.danger === true}
          class:on={actionIndex === selectedActionIndex}
          role="option"
          tabindex="-1"
          aria-selected={actionIndex === selectedActionIndex}
          onmousedown={preventFocusLossOnMouseDown}
          onmousemove={() => handleActionHover(actionIndex)}
          onclick={() => handleActionClick(actionIndex)}
        >
          <span class="t">{action.label}</span>
          <span class="r k">
            {#each action.shortcut.split(" ").filter(Boolean) as key (key)}
              <kbd>{key}</kbd>
            {/each}
          </span>
        </div>
        {#if actionIndex === 0 && selectedItem.actions.length > 1}
          <div class="hs" aria-hidden="true"></div>
        {/if}
      {/each}
    {/if}
  </div>
</div>
{:else}
<div
  class="win clipboard-win"
  id="win"
  role="dialog"
  aria-label="Lux Clipboard History"
>
  <ClipboardView
    onClose={dismissLauncher}
    onSwitchToLauncher={() => void switchMode("launcher")}
  />
</div>
{/if}


<style>
  :global(:root) {
    --bg: rgba(251, 251, 253, 0.92);
    --bgt: rgba(251, 251, 253, 0.92);
    --fg: #1d1d1f;
    --mut: #6e6e73;
    --dim: #9a9aa0;
    --line: rgba(0, 0, 0, 0.08);
    --sel: rgba(0, 0, 0, 0.07);
    --chip: rgba(0, 0, 0, 0.06);
    --kbd: rgba(0, 0, 0, 0.07);
    --acc: #e5484d;
    --shadow:
      0 0 0 1px rgba(0, 0, 0, 0.12), 0 24px 60px -12px rgba(0, 0, 0, 0.35);
    --wall: linear-gradient(135deg, #cfd6de, #b4bfcc 50%, #98a6b8);
    --tile: #fff;
    --glass: rgba(251, 251, 253, 0.92);
    --gfb: rgba(251, 251, 253, 0.92);
    --hi: rgba(255, 255, 255, 0.65);
    color-scheme: light;
  }

  @media (prefers-color-scheme: dark) {
    :global(:root:not([data-theme="light"])) {
      --bg: rgba(10, 10, 12, 0.94);
      --bgt: rgba(10, 10, 12, 0.94);
      --fg: #f4f4f5;
      --mut: #a1a1aa;
      --dim: #71717a;
      --line: rgba(255, 255, 255, 0.07);
      --sel: rgba(255, 255, 255, 0.08);
      --chip: rgba(255, 255, 255, 0.07);
      --kbd: rgba(255, 255, 255, 0.09);
      --shadow:
        0 0 0 1px rgba(255, 255, 255, 0.08), 0 24px 60px -12px rgba(0, 0, 0, 0.8);
      --wall: linear-gradient(135deg, #0a0a0c, #141418 50%, #1e1e24);
      --glass: rgba(14, 14, 18, 0.9);
      --gfb: rgba(10, 10, 12, 0.96);
      --hi: rgba(255, 255, 255, 0.07);
      color-scheme: dark;
    }
  }

  :global(:root[data-theme="dark"]) {
    --bg: rgba(10, 10, 12, 0.94);
    --bgt: rgba(10, 10, 12, 0.94);
    --fg: #f4f4f5;
    --mut: #a1a1aa;
    --dim: #71717a;
    --line: rgba(255, 255, 255, 0.07);
    --sel: rgba(255, 255, 255, 0.08);
    --chip: rgba(255, 255, 255, 0.07);
    --kbd: rgba(255, 255, 255, 0.09);
    --shadow:
      0 0 0 1px rgba(255, 255, 255, 0.08), 0 24px 60px -12px rgba(0, 0, 0, 0.8);
    --wall: linear-gradient(135deg, #0a0a0c, #141418 50%, #1e1e24);
    --glass: rgba(14, 14, 18, 0.9);
    --gfb: rgba(10, 10, 12, 0.96);
    --hi: rgba(255, 255, 255, 0.07);
    color-scheme: dark;
  }

  :global(:root[data-theme="light"]) {
    --bg: rgba(251, 251, 253, 0.92);
    --bgt: rgba(251, 251, 253, 0.92);
    --fg: #1d1d1f;
    --mut: #6e6e73;
    --dim: #9a9aa0;
    --line: rgba(0, 0, 0, 0.08);
    --sel: rgba(0, 0, 0, 0.07);
    --chip: rgba(0, 0, 0, 0.06);
    --kbd: rgba(0, 0, 0, 0.07);
    --acc: #e5484d;
    --shadow:
      0 0 0 1px rgba(0, 0, 0, 0.12), 0 24px 60px -12px rgba(0, 0, 0, 0.35);
    --wall: linear-gradient(135deg, #cfd6de, #b4bfcc 50%, #98a6b8);
    --tile: #fff;
    --glass: rgba(251, 251, 253, 0.92);
    --gfb: rgba(251, 251, 253, 0.92);
    --hi: rgba(255, 255, 255, 0.65);
    color-scheme: light;
  }

  :global(html) {
    background: transparent !important;
    background-color: transparent !important;
  }

  :global(body) {
    margin: 0;
    padding: 0;
    background: transparent !important;
    background-color: transparent !important;
    font:
      400 13.5px/1.35 Inter,
      "Segoe UI",
      Cantarell,
      system-ui,
      sans-serif;
    color: var(--fg);
    overflow: hidden;
    -webkit-font-smoothing: antialiased;
    user-select: none;
  }

  .win {
    width: 100vw;
    height: 100vh;
    box-sizing: border-box;
    background: var(--bg);
    -webkit-backdrop-filter: blur(20px);
    backdrop-filter: blur(20px);
    border-radius: 14px;
    box-shadow: var(--shadow), inset 0 1px 0 var(--hi);
    overflow: hidden;
    position: relative;
    display: flex;
    flex-direction: column;
  }

  .win.idle {
    height: 144px;
    max-height: 144px;
    flex: none;
  }

  .bar {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 68px;
    padding: 0 18px;
    border-bottom: 1px solid var(--line);
    flex: none;
    position: relative;
  }

  .progress-bar {
    position: absolute;
    left: 0;
    right: 0;
    height: 2px;
    background: transparent;
    overflow: hidden;
    pointer-events: none;
    z-index: 2;
  }

  .indexing-bar {
    top: 0;
  }

  .indexing-fill {
    height: 100%;
    width: 25%;
    background: var(--mut);
    opacity: 0.55;
    animation: progress-pulse 1.3s infinite ease-in-out;
  }

  .search-bar {
    bottom: 0;
  }

  .search-fill {
    height: 100%;
    width: 32%;
    background: var(--fg);
    opacity: 0.85;
    border-radius: 1px;
    animation: search-sweep 0.85s infinite ease-in-out;
  }

  @keyframes search-sweep {
    0% {
      transform: translateX(-100%);
    }
    100% {
      transform: translateX(350%);
    }
  }

  @keyframes progress-pulse {
    0% {
      transform: translateX(-100%);
    }
    100% {
      transform: translateX(350%);
    }
  }

  .bar > svg {
    flex: none;
    color: var(--mut);
    width: 22px;
    height: 22px;
  }

  #q {
    flex: 1;
    min-width: 0;
    background: none;
    border: 0;
    outline: 0;
    color: var(--fg);
    caret-color: var(--caret-color, var(--fg));
    font:
      400 22px Inter,
      "Segoe UI",
      system-ui,
      sans-serif;
    letter-spacing: -0.01em;
  }

  #q::placeholder {
    color: var(--dim);
  }

  .prefix-pill {
    flex: none;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    padding: 2px 7px;
    border-radius: 5px;
    background: var(--chip);
    color: var(--mut);
    border: 1px solid var(--line);
    user-select: none;
    line-height: 1.4;
  }

  .spin {
    width: 15px;
    height: 15px;
    border: 2px solid var(--chip);
    border-top-color: var(--mut);
    border-radius: 50%;
    animation: s 0.7s linear infinite;
    display: none;
  }

  @keyframes s {
    to {
      transform: rotate(360deg);
    }
  }

  .gear {
    background: none;
    border: 0;
    color: var(--mut);
    padding: 7px;
    border-radius: 8px;
    cursor: pointer;
    display: grid;
    place-items: center;
    flex: none;
  }

  .gear:hover {
    background: var(--sel);
    color: var(--fg);
  }

  .hints {
    display: none;
    padding: 8px 20px 16px;
  }

  .idle:not(.collapsing) .hints {
    display: block;
  }

  .idle:not(.collapsing) .body,
  .idle:not(.collapsing) .foot {
    display: none;
  }

  .idle:not(.collapsing) .bar {
    border-bottom-color: transparent;
  }

  .hr {
    display: flex;
    gap: 6px;
    align-items: center;
    color: var(--dim);
    font-size: 12.5px;
    flex-wrap: wrap;
  }

  .hr button {
    font: inherit;
    font-size: 12.5px;
    color: var(--mut);
    background: var(--chip);
    border: 0;
    border-radius: 6px;
    padding: 3px 9px;
    cursor: pointer;
  }

  .hr button:hover {
    background: var(--sel);
    color: var(--fg);
  }

  .hk {
    display: flex;
    gap: 16px;
    margin-top: 12px;
    color: var(--dim);
    font-size: 12px;
    flex-wrap: wrap;
  }

  .hk span {
    display: inline-flex;
    gap: 4px;
    align-items: center;
  }

  .body {
    display: flex;
    flex: 1;
    min-height: 0;
  }

  #list {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    padding: 4px 10px 10px;
    scrollbar-width: thin;
  }

  .pv {
    width: 250px;
    flex: none;
    border-left: 1px solid var(--line);
    padding: 16px;
    overflow: hidden;
    box-sizing: border-box;
  }

  .nopv .pv {
    display: none;
  }

  .pbig {
    height: 140px;
    width: 100%;
    object-fit: cover;
    border-radius: 10px;
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.12);
    margin-bottom: 12px;
    display: block;
  }

  .pico {
    display: grid;
    place-items: center;
    height: 110px;
    margin-bottom: 8px;
  }

  .pico .ico {
    width: 68px;
    height: 68px;
    border-radius: 18px;
    font-size: 28px;
  }

  .pico .ico-img {
    width: 68px;
    height: 68px;
    border-radius: 18px;
  }

  .pico .ico svg {
    width: 30px;
    height: 30px;
  }

  .pt {
    font-weight: 600;
    font-size: 14px;
    margin-bottom: 8px;
    word-break: break-word;
  }

  .pm {
    display: flex;
    justify-content: space-between;
    gap: 10px;
    padding: 4px 0;
    font-size: 12px;
    color: var(--mut);
  }

  .pm span:last-child {
    color: var(--fg);
    text-align: right;
    word-break: break-all;
  }

  .sec {
    font-size: 12px;
    font-weight: 500;
    color: var(--dim);
    padding: 12px 10px 6px;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 4px;
    padding: 2px 0 4px;
  }

  .tile {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 12px 4px 10px;
    border-radius: 12px;
    text-align: center;
    min-width: 0;
    cursor: default;
  }

  .tile.on {
    background: var(--sel);
  }

  .tile .ico {
    width: 44px;
    height: 44px;
    border-radius: 11px;
    font-size: 18px;
  }

  .tile .ico-img {
    width: 44px;
    height: 44px;
    border-radius: 11px;
  }

  .tile .thumb {
    width: 60px;
    height: 44px;
    border-radius: 7px;
    object-fit: cover;
  }

  .tile .t {
    font-size: 12px;
    max-width: 100%;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 48px;
    padding: 0 10px;
    border-radius: 8px;
    cursor: default;
  }

  .row.on {
    background: var(--sel);
  }

  .ico,
  .thumb {
    width: 30px;
    height: 30px;
  }

  .ico {
    flex: none;
    border-radius: 8px;
    display: grid;
    place-items: center;
    color: #fff;
    font-weight: 600;
    font-size: 13px;
  }

  .ico-img {
    width: 30px;
    height: 30px;
    border-radius: 8px;
    flex: none;
    object-fit: contain;
  }

  .ico.g {
    background: var(--chip);
    color: var(--mut);
  }

  .thumb {
    border-radius: 6px;
    flex: none;
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.12);
    object-fit: cover;
  }

  .t {
    font-size: 14px;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .s {
    color: var(--mut);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-size: 12.5px;
  }

  .txt {
    display: flex;
    gap: 9px;
    align-items: baseline;
    min-width: 0;
    flex: 1;
  }

  .r {
    margin-left: auto;
    color: var(--dim);
    font-size: 12px;
    flex: none;
    padding-left: 8px;
  }

  mark {
    background: none;
    color: inherit;
    font-weight: 600;
  }

  .calc {
    display: flex;
    align-items: center;
    margin: 2px 0 4px;
    padding: 14px 16px;
    border-radius: 10px;
    background: var(--sel);
  }

  .calc .a,
  .calc .b {
    flex: 1;
    min-width: 0;
  }

  .calc small {
    display: block;
    color: var(--dim);
    font-size: 11.5px;
    margin-bottom: 3px;
  }

  .calc .v {
    font-size: 26px;
    font-weight: 500;
    letter-spacing: -0.02em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .calc .e {
    font-size: 17px;
    color: var(--mut);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .calc .ar {
    color: var(--dim);
    padding: 0 14px;
  }

  .calc .b {
    text-align: right;
  }

  .empty {
    padding: 34px 16px 30px;
    text-align: center;
  }

  .empty .big {
    width: 34px;
    height: 34px;
    margin: 0 auto 10px;
    color: var(--dim);
  }

  .empty p {
    color: var(--mut);
    margin-top: 3px;
    font-size: 12.5px;
  }

  .foot {
    display: flex;
    align-items: center;
    height: 42px;
    padding: 0 14px;
    border-top: 1px solid var(--line);
    gap: 10px;
    color: var(--mut);
    font-size: 12px;
    flex: none;
  }

  .logo {
    width: 18px;
    height: 18px;
    border-radius: 5px;
    background: var(--fg);
    color: var(--bg);
    display: grid;
    place-items: center;
    font-weight: 600;
    font-size: 11px;
    flex: none;
  }

  .logo svg {
    display: block;
  }

  .foot .sp {
    flex: 1;
  }

  .k {
    display: inline-flex;
    gap: 3px;
    align-items: center;
  }

  kbd {
    font:
      500 11px Inter,
      "Segoe UI",
      system-ui,
      sans-serif;
    background: var(--kbd);
    color: var(--mut);
    padding: 2px 6px;
    border-radius: 5px;
    min-width: 20px;
    text-align: center;
  }

  .btn {
    display: inline-flex;
    gap: 7px;
    align-items: center;
    cursor: pointer;
    color: var(--fg);
    background: none;
    border: 0;
    font: inherit;
    padding: 4px 6px;
    border-radius: 6px;
  }

  .btn:hover {
    background: var(--sel);
  }

  .g1 .btn {
    font-weight: 500;
    color: var(--fg);
  }

  .g1 kbd {
    color: var(--fg);
  }

  .g2 .btn {
    color: var(--mut);
  }

  .g2 kbd {
    background: transparent;
    box-shadow: inset 0 0 0 1px var(--line);
  }

  .vd {
    width: 1px;
    height: 16px;
    background: var(--line);
    margin: 0 4px;
    flex: none;
  }

  .pop {
    position: absolute;
    right: 10px;
    bottom: 48px;
    width: 268px;
    background: var(--bg);
    border-radius: 10px;
    box-shadow: var(--shadow);
    padding: 6px;
    display: none;
    z-index: 4;
  }

  .pop.open {
    display: block;
  }

  .pop h6 {
    font:
      500 11.5px Inter,
      "Segoe UI",
      system-ui,
      sans-serif;
    color: var(--dim);
    padding: 6px 8px 4px;
    margin: 0;
  }

  .pop .row {
    height: 34px;
  }

  .pop .row.dan .t {
    color: var(--acc);
  }

  .pop .hs {
    height: 1px;
    background: var(--line);
    margin: 4px 6px;
  }

  .clipboard-win {
    height: 560px;
    max-height: 560px;
  }

  /* Minimal Style: Compact single-line results, no footer noise, maximum information density */
  :global(:root[data-style="minimal"]) .bar {
    height: 54px;
    padding: 0 14px;
    gap: 8px;
  }

  :global(:root[data-style="minimal"]) #q {
    font-size: 18px;
  }

  :global(:root[data-style="minimal"]) .row {
    height: 36px;
    padding: 0 8px;
    gap: 8px;
  }

  :global(:root[data-style="minimal"]) .ico,
  :global(:root[data-style="minimal"]) .ico-img,
  :global(:root[data-style="minimal"]) .thumb {
    width: 22px;
    height: 22px;
    border-radius: 5px;
  }

  :global(:root[data-style="minimal"]) .t {
    font-size: 13px;
  }

  :global(:root[data-style="minimal"]) .s {
    font-size: 11.5px;
  }

  :global(:root[data-style="minimal"]) .hr {
    display: none !important;
  }

  /* Detailed Style: Richer metadata, larger touch/click targets, full status information */
  :global(:root[data-style="detailed"]) .bar {
    height: 72px;
    padding: 0 20px;
  }

  :global(:root[data-style="detailed"]) .row {
    height: 56px;
    padding: 0 14px;
    gap: 14px;
  }

  :global(:root[data-style="detailed"]) .ico,
  :global(:root[data-style="detailed"]) .ico-img,
  :global(:root[data-style="detailed"]) .thumb {
    width: 36px;
    height: 36px;
    border-radius: 9px;
  }

  :global(:root[data-style="detailed"]) .t {
    font-size: 15px;
  }

  :global(:root[data-style="detailed"]) .s {
    font-size: 13px;
    opacity: 0.85;
  }

  :global(:root[data-style="detailed"]) .hr {
    padding: 12px 18px;
  }
</style>
