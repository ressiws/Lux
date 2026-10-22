<script lang="ts">
  import { onMount, tick } from "svelte";
  import ClipboardView from "$lib/ClipboardView.svelte";
  import {
    CATEGORY_ICONS,
    buildSections,
    getCategoryPlaceholder,
    hasOnlyFallbackSections,
    splitHighlight,
    getItemDisplayIcon,
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
    browseLauncher,
    openTargetAsAdmin,
    reindex,
    revealItemInFolder,
    searchLauncher,
    setLauncherHeight,
    setWindowHeight,
    getDefaultBrowser,
    showPreview,
    hidePreview,
    togglePreview,
    onOpenPreview,
    onClosePreview,
  } from "$lib/tauri";
  import SettingsView from "$lib/SettingsView.svelte";
  import QuickLookView from "$lib/QuickLookView.svelte";
  import {
    applyThemeAndCaret,
    getIdleLauncherHeight,
    loadSettings,
    onSettingsChanged,
    saveSettings,
    type SettingsState,
  } from "$lib/settings";
  import type { SearchFilter, BuildSectionsOptions } from "$lib/mock-data";
  import {
    getAvailableSubcategories,
    type SubcategoryItem,
  } from "$lib/classification";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { resolvePrefix } from "$lib/prefix";
  import type {
    FeaturesConfig,
    IndexStats,
    IndexingStatusPayload,
    ItemAction,
    LauncherItem,
    LauncherSection,
    SearchResponse,
    DefaultBrowserInfo,
  } from "$lib/types";
  import { registerWindowDrag } from "$lib/windowDrag";

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
  let defaultBrowser = $state<DefaultBrowserInfo | undefined>(undefined);
  let isLaunching = false;

  let searchInput: HTMLInputElement | undefined = $state();
  let listContainer: HTMLDivElement | undefined = $state();
  let feedbackTimer: ReturnType<typeof setTimeout> | undefined;
  let opSeq = 0;
  let searchDebounceTimer: ReturnType<typeof setTimeout> | undefined;
  let searchProgressTimer: ReturnType<typeof setTimeout> | undefined;
  let isSearching = $state(false);
  let isPreviewOpen = $state(false);
  let hasNavigatedList = $state(false);
  let searchBarVisible = $state(false);
  let activeFilter = $state<SearchFilter>("all");
  let activeSubcategory = $state<string>("all");
  let browseResponse = $state<SearchResponse | undefined>(undefined);
  const BATCH_SIZE_APPS = 50;
  const BATCH_SIZE_FILES = 20;
  function getBatchSize(filter: SearchFilter): number {
    return filter === "files" ? BATCH_SIZE_FILES : BATCH_SIZE_APPS;
  }
  const browseCache = new Map<SearchFilter, SearchResponse>();
  let visibleCount = $state(BATCH_SIZE_APPS);
  let sentinelElement = $state<HTMLElement | undefined>(undefined);

  const prefixResolution = $derived(
    resolvePrefix(
      query,
      settings.flowPrefixCompatibility,
      settings.flowPrefixes,
    ),
  );
  const effectiveQuery = $derived(prefixResolution.effectiveQuery);
  const isBrowseMode = $derived(activeFilter !== "all");
  const isIdle = $derived(effectiveQuery.trim().length === 0 && !isBrowseMode);

  // All items in the selected category (without subcategory filtering)
  // to compute available contextual subcategories that actually have results.
  const allCategoryItems = $derived.by<LauncherItem[]>(() => {
    if (!isBrowseMode) return [];
    const base = buildSections(effectiveQuery, searchResponse, {
      enableFileSearch: settings.fileSearch,
      filter: activeFilter,
      subcategory: "all",
      browseResponse,
      defaultBrowser,
    });
    return base.flatMap((s) => s.items);
  });

  const availableSubcategories = $derived(
    getAvailableSubcategories(activeFilter, allCategoryItems),
  );

  const sections = $derived(
    buildSections(effectiveQuery, searchResponse, {
      enableFileSearch: settings.fileSearch,
      filter: activeFilter,
      subcategory: activeSubcategory,
      browseResponse,
      defaultBrowser,
    }),
  );
  const flatItems = $derived(sections.flatMap((section) => section.items));

  const visibleSections = $derived.by<LauncherSection[]>(() => {
    if (flatItems.length <= visibleCount) {
      return sections;
    }
    let remaining = visibleCount;
    const sliced: LauncherSection[] = [];
    for (const sec of sections) {
      if (remaining <= 0) break;
      if (sec.items.length <= remaining) {
        sliced.push(sec);
        remaining -= sec.items.length;
      } else {
        sliced.push({
          ...sec,
          items: sec.items.slice(0, remaining),
        });
        remaining = 0;
      }
    }
    return sliced;
  });
  const showViewSelector = $derived(
    (isBrowseMode || !isIdle) && flatItems.length > 0 && !hasOnlyFallbackSections(sections),
  );
  const selectedItem = $derived<LauncherItem | undefined>(
    flatItems[selectedIndex],
  );
  const showEmptyState = $derived(
    (flatItems.length === 0 || hasOnlyFallbackSections(sections)) &&
      (effectiveQuery.trim().length > 0 || isBrowseMode),
  );
  const displayedMessage = $derived(feedbackMessage || shortcutWarning);

  let isCollapsing = $state(false);
  let collapseTimer: ReturnType<typeof setTimeout> | undefined;

  const shouldExpand = $derived(!isIdle);
  let observer: IntersectionObserver | undefined;
  $effect(() => {
    if (windowLabel !== "main") return;
    if (sentinelElement && visibleCount < flatItems.length) {
      observer?.disconnect();
      if (typeof IntersectionObserver !== "undefined") {
        observer = new IntersectionObserver(
          (entries) => {
            if (entries[0]?.isIntersecting) {
              if (visibleCount < flatItems.length) {
                visibleCount = Math.min(
                  flatItems.length,
                  visibleCount + getBatchSize(activeFilter),
                );
              }
            }
          },
          { root: listContainer, rootMargin: "200px" },
        );
        observer.observe(sentinelElement);
      }
      return () => observer?.disconnect();
    }
  });

  function handleListScroll(): void {
    if (!listContainer || visibleCount >= flatItems.length) return;
    const { scrollTop, scrollHeight, clientHeight } = listContainer;
    if (scrollHeight - scrollTop - clientHeight < 200) {
      visibleCount = Math.min(
        flatItems.length,
        visibleCount + getBatchSize(activeFilter),
      );
    }
  }

  $effect(() => {
    if (windowLabel !== "main") return;
    const _q = effectiveQuery;
    const _f = activeFilter;
    const _s = activeSubcategory;
    visibleCount = getBatchSize(_f);
    if (listContainer) {
      listContainer.scrollTop = 0;
    }
  });

  const currentIdleHeight = $derived(
    getIdleLauncherHeight(settings.informationDensity, settings.layoutDensity),
  );

  let lastSyncedHeight: number | undefined;

  async function syncWindowHeight(expanded: boolean): Promise<void> {
    if (windowLabel !== "main") return;
    if (appMode === "clipboard") return;
    const targetHeight = expanded ? 500 : currentIdleHeight;
    if (windowExpanded === expanded && lastSyncedHeight === targetHeight) return;
    windowExpanded = expanded;
    lastSyncedHeight = targetHeight;
    try {
      await setLauncherHeight(expanded, currentIdleHeight);
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
        await setWindowHeight(620);
      } catch {
        // Ignore outside Tauri
      }
    } else {
      await syncWindowHeight(shouldExpand);
      focusSearchInput();
    }
  }


  $effect(() => {
    if (windowLabel !== "main") return;
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
    const seq = ++opSeq;
    const trimmed = q.trim();
    if (trimmed.length === 0) {
      isSearching = false;
      searchBarVisible = false;
      clearTimeout(searchProgressTimer);
      searchResponse = undefined;
      if (activeFilter !== "all" && !browseResponse) {
        void loadBrowseData(activeFilter);
      }
      return;
    }

    isSearching = true;
    clearTimeout(searchProgressTimer);
    searchProgressTimer = setTimeout(() => {
      if (isSearching && seq === opSeq) {
        searchBarVisible = true;
      }
    }, 35);

    try {
      const resp = await searchLauncher(trimmed);
      if (seq === opSeq) {
        searchResponse = resp;
      }
    } catch {
      // Ignore if Tauri invocation is unavailable
    } finally {
      if (seq === opSeq) {
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
    activeSubcategory = "all";
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

  async function loadBrowseData(filter: SearchFilter): Promise<void> {
    const seq = ++opSeq;
    if (filter === "all" || filter === "actions") {
      browseResponse = undefined;
      return;
    }
    const cached = browseCache.get(filter);
    if (cached) {
      browseResponse = cached;
      return;
    }
    try {
      const resp = await browseLauncher(filter);
      if (seq === opSeq && activeFilter === filter) {
        browseCache.set(filter, resp);
        browseResponse = resp;
      }
    } catch {
      // Ignore if outside Tauri environment
    }
  }

  function returnToAllCategories(): void {
    activeFilter = "all";
    activeSubcategory = "all";
    browseResponse = undefined;
    visibleCount = getBatchSize("all");
    selectedIndex = 0;
    selectedActionIndex = 0;
    focusSearchInput();
    if (query.trim().length > 0) {
      void performSearch(effectiveQuery);
    }
  }

  function selectSubcategory(subcatId: string): void {
    activeSubcategory = subcatId;
    visibleCount = getBatchSize(activeFilter);
    selectedIndex = 0;
    selectedActionIndex = 0;
    focusSearchInput();
  }

  function toggleFilter(filter: "apps" | "files" | "actions"): void {
    activeSubcategory = "all";
    visibleCount = getBatchSize(filter);
    if (activeFilter === filter) {
      returnToAllCategories();
    } else {
      activeFilter = filter;
      void loadBrowseData(filter);
    }
    selectedIndex = 0;
    selectedActionIndex = 0;
    actionsOpen = false;
    focusSearchInput();
  }

  function setLayout(layout: "list" | "grid"): void {
    settings = { ...settings, layout };
    saveSettings(settings);
    focusSearchInput();
  }

  function resetLauncherState(shouldFocus: boolean): void {
    activeFilter = "all";
    activeSubcategory = "all";
    actionsOpen = false;
    selectedActionIndex = 0;
    query = "";
    selectedIndex = 0;
    opSeq++;
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
    browseCache.clear();
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
    if (isLaunching) return;
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
      if (item.primaryLabel === "Search the Web") {
        isLaunching = true;
        try {
          const url = `https://www.google.com/search?q=${encodeURIComponent(effectiveQuery.trim())}`;
          await openTarget(url);
          await dismissLauncher();
        } catch (err) {
          say(`Failed to open browser: ${err}`);
        } finally {
          setTimeout(() => {
            isLaunching = false;
          }, 300);
        }
        return;
      }
      if (item.primaryLabel === "Search Files") {
        toggleFilter("files");
        return;
      }
      say("Searching…");
      return;
    }
    if (item.path) {
      isLaunching = true;
      try {
        await openTarget(item.path);
        await dismissLauncher();
      } catch (err) {
        say(`Failed to open: ${err}`);
      } finally {
        setTimeout(() => {
          isLaunching = false;
        }, 300);
      }
      return;
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

    if (item.category === "fb") {
      if (action.label === "Search the Web") {
        if (isLaunching) return;
        isLaunching = true;
        try {
          const url = `https://www.google.com/search?q=${encodeURIComponent(effectiveQuery.trim())}`;
          await openTarget(url);
          await dismissLauncher();
        } catch (err) {
          say(`Failed to open browser: ${err}`);
        } finally {
          setTimeout(() => {
            isLaunching = false;
          }, 300);
        }
        return;
      }
      if (action.label === "Search Files" || action.label === "Search Files for This Term") {
        toggleFilter("files");
        return;
      }
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
        if (isLaunching) return;
        isLaunching = true;
        try {
          await revealItemInFolder(item.path);
          await dismissLauncher();
        } catch (err) {
          say(`Failed to reveal: ${err}`);
        } finally {
          setTimeout(() => {
            isLaunching = false;
          }, 300);
        }
        return;
      }
    }

    if (action.label === "Quick Look" || action.label === "Quick Look Preview") {
      if (item.path) {
        if (isPreviewOpen) {
          await hidePreview();
        } else {
          await showPreview(item.path);
        }
        return;
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
        if (isLaunching) return;
        isLaunching = true;
        try {
          await openTargetAsAdmin(item.path);
          await dismissLauncher();
        } catch (err) {
          say(`Failed to run as admin: ${err}`);
        } finally {
          setTimeout(() => {
            isLaunching = false;
          }, 300);
        }
        return;
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
    hasNavigatedList = false;
    selectedIndex = 0;
    selectedActionIndex = 0;
    actionsOpen = false;
    clearTimeout(searchDebounceTimer);
    clearTimeout(searchProgressTimer);

    if (activeSubcategory !== "all") {
      activeSubcategory = "all";
    }

    const q = effectiveQuery;
    if (q.trim().length === 0) {
      isSearching = false;
      searchBarVisible = false;
      searchResponse = undefined;
      opSeq++;
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

  async function handleKeydown(event: KeyboardEvent): Promise<void> {
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      if (isPreviewOpen) {
        void hidePreview();
        return;
      }
      if (actionsOpen) {
        closeActionsPopover();
        return;
      }
      if (activeFilter !== "all" || activeSubcategory !== "all") {
        returnToAllCategories();
        return;
      }
      void dismissLauncher();
      return;
    }

    const count = flatItems.length;

    if ((event.ctrlKey || event.metaKey) && !event.shiftKey && !event.altKey) {
      if (event.key === "1") {
        event.preventDefault();
        toggleFilter("apps");
        return;
      }
      if (event.key === "2") {
        event.preventDefault();
        toggleFilter("files");
        return;
      }
      if (event.key === "3") {
        event.preventDefault();
        toggleFilter("actions");
        return;
      }
    }

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
      if (isLaunching) return;
      isLaunching = true;
      try {
        await openTargetAsAdmin(selectedItem.path);
        await dismissLauncher();
      } catch (err) {
        say(`Failed to run as admin: ${err}`);
      } finally {
        setTimeout(() => {
          isLaunching = false;
        }, 300);
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
      if (isLaunching) return;
      isLaunching = true;
      try {
        await revealItemInFolder(selectedItem.path);
        await dismissLauncher();
      } catch (err) {
        say(`Failed to reveal: ${err}`);
      } finally {
        setTimeout(() => {
          isLaunching = false;
        }, 300);
      }
      return;
    }

    if (event.key === "ArrowDown") {
      event.preventDefault();
      hasNavigatedList = true;
      selectedIndex = nav(1);
      void scrollSelectedIntoView();
      if (isPreviewOpen && selectedItem?.path) {
        void showPreview(selectedItem.path);
      }
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      hasNavigatedList = true;
      selectedIndex = nav(-1);
      void scrollSelectedIntoView();
      if (isPreviewOpen && selectedItem?.path) {
        void showPreview(selectedItem.path);
      }
      return;
    }

    if (
      (event.key === "ArrowRight" || event.key === "ArrowLeft") &&
      settings.layout === "grid" &&
      selectedItem &&
      (selectedItem.category === "app" || selectedItem.category === "file")
    ) {
      event.preventDefault();
      hasNavigatedList = true;
      selectedIndex =
        event.key === "ArrowRight"
          ? Math.min(count - 1, selectedIndex + 1)
          : Math.max(0, selectedIndex - 1);
      void scrollSelectedIntoView();
      if (isPreviewOpen && selectedItem?.path) {
        void showPreview(selectedItem.path);
      }
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

    if (
      event.shiftKey &&
      !event.ctrlKey &&
      !event.altKey &&
      !event.metaKey &&
      (event.key === " " || event.code === "Space")
    ) {
      if (selectedItem?.path) {
        event.preventDefault();
        event.stopPropagation();
        if (isPreviewOpen) {
          void hidePreview();
        } else {
          void showPreview(selectedItem.path);
        }
        return;
      }
    }

    if (
      !event.shiftKey &&
      !event.ctrlKey &&
      !event.altKey &&
      !event.metaKey &&
      (event.key === " " || event.code === "Space")
    ) {
      const isInputActive = document.activeElement === searchInput;
      if (!isInputActive || hasNavigatedList) {
        if (selectedItem?.path) {
          event.preventDefault();
          event.stopPropagation();
          if (isPreviewOpen) {
            void hidePreview();
          } else {
            void showPreview(selectedItem.path);
          }
          return;
        }
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
      hasNavigatedList = true;
      if (isPreviewOpen && selectedItem?.path) {
        void showPreview(selectedItem.path);
      }
    }
  }

  function handleRowContextMenu(event: MouseEvent, index: number): void {
    event.preventDefault();
    event.stopPropagation();
    selectedIndex = index;
    openActionsPopover();
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

  onMount(() => {
    applyThemeAndCaret(settings);
    if (windowLabel !== "main") {
      return;
    }
    focusSearchInput();
    void performSearch("");
    const idleH = getIdleLauncherHeight(
      settings.informationDensity,
      settings.layoutDensity,
    );
    void notifyFrontendReady(idleH).catch(() => {});

    void getIndexStats()
      .then((stats) => {
        indexStats = stats;
        if (typeof stats.isIndexing === "boolean") {
          isIndexing = stats.isIndexing;
        }
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
      if (appMode === "launcher" && isIdle) {
        void syncWindowHeight(false);
      }
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
      appMode = "clipboard";
      actionsOpen = false;
      void setWindowHeight(620);
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

    void getDefaultBrowser()
      .then((browser) => {
        if (!disposed) {
          defaultBrowser = browser;
        }
      })
      .catch(() => {});

    void onIndexingStatus((payload) => {
      isIndexing = payload.isIndexing;
      if (!payload.isIndexing) {
        browseCache.clear();
        if (activeFilter !== "all") {
          void loadBrowseData(activeFilter);
        }
        indexStats = {
          totalApps: payload.totalApps,
          totalFiles: payload.totalFiles,
          isIndexed: true,
          isIndexing: false,
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
      lastSyncedHeight = undefined;
      windowExpanded = false;
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
    const handleContextMenu = (e: MouseEvent) => {
      const target = e.target as HTMLElement | null;
      if (target?.closest("input, textarea, [contenteditable='true']")) return;
      e.preventDefault();
    };
    window.addEventListener("contextmenu", handleContextMenu);
    let unlistenOpenPreview: (() => void) | undefined;
    let unlistenClosePreview: (() => void) | undefined;
    if (windowLabel !== "preview") {
      void onOpenPreview(() => {
        isPreviewOpen = true;
      }).then((fn) => {
        unlistenOpenPreview = fn;
      });
      void onClosePreview(() => {
        isPreviewOpen = false;
      }).then((fn) => {
        unlistenClosePreview = fn;
      });
    }

    const unregisterDrag = registerWindowDrag();

    return () => {
      disposed = true;
      unregisterDrag();
      clearTimeout(feedbackTimer);
      clearTimeout(searchDebounceTimer);
      clearTimeout(searchProgressTimer);
      unlistenShown?.();
      unlistenHidden?.();
      unlistenIndexing?.();
      unlistenOpenLauncher?.();
      unlistenOpenClipboard?.();
      unlistenOpenPreview?.();
      unlistenClosePreview?.();
      unlistenSettings?.();
      unlistenFeatures?.();
      window.removeEventListener("focus", handleWindowFocus);
      window.removeEventListener("keydown", handleKeydown);
      window.removeEventListener("contextmenu", handleContextMenu);
    };
  });
</script>

{#if windowLabel === "settings"}
  <SettingsView />
{:else if windowLabel === "preview"}
  <QuickLookView />
{:else if appMode === "launcher"}
<div
  class="win"
  id="win"
  class:idle={isIdle}
  class:collapsing={isCollapsing}
  role="dialog"
  aria-label="Lux command palette"
>
  <div class="bar" id="bar" data-tauri-drag-region>
    {#if activeFilter !== "all"}
      <button
        type="button"
        class="cat-back-btn"
        title="Back to Search Anything"
        aria-label="Back to Search Anything"
        onmousedown={preventFocusLossOnMouseDown}
        onclick={returnToAllCategories}
        onkeydown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            returnToAllCategories();
          }
        }}
      >
        <span class="icon-cat" aria-hidden="true">
          {#if activeFilter === "apps"}
            <svg
              class="search-icon"
              width="22"
              height="22"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="1.8"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <rect x="3" y="3" width="18" height="18" rx="4" />
              <path d="M3 9h18M9 21V9" />
            </svg>
          {:else if activeFilter === "files"}
            <svg
              class="search-icon"
              width="22"
              height="22"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="1.8"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
              <polyline points="14 2 14 8 20 8" />
            </svg>
          {:else if activeFilter === "actions"}
            <svg
              class="search-icon"
              width="22"
              height="22"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="1.8"
              stroke-linecap="round"
              stroke-linejoin="round"
            >
              <path d="M9 6a3 3 0 1 0-3 3h12a3 3 0 1 0-3-3v12a3 3 0 1 0 3-3H6a3 3 0 1 0 3 3z" />
            </svg>
          {/if}
        </span>
        <span class="icon-arrow" aria-hidden="true">
          <svg
            class="search-icon"
            width="22"
            height="22"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <path d="M19 12H5M12 19l-7-7 7-7" />
          </svg>
        </span>
      </button>
    {:else if isIdle}
      <div class="logo bar-logo" aria-label="Lux">
        <svg viewBox="0 0 64 64" width="14" height="14" fill="currentColor">
          <path d="M 10 8 H 22 V 40 L 46 16 H 58 L 34 40 H 58 V 52 H 10 Z" />
        </svg>
      </div>
    {:else}
      <svg
        class="search-icon"
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
    {/if}
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
      placeholder={getCategoryPlaceholder(activeFilter)}
      aria-label="Search"
    />
    <div class="spin" aria-hidden="true"></div>

    {#if showViewSelector}
      <div class="view-selector" role="radiogroup" aria-label="Results view">
        <button
          type="button"
          class="view-btn"
          class:active={settings.layout === "list"}
          role="radio"
          aria-checked={settings.layout === "list"}
          title="List view"
          aria-label="List view"
          onmousedown={preventFocusLossOnMouseDown}
          onclick={() => setLayout("list")}
        >
          <svg
            width="13"
            height="13"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            aria-hidden="true"
          >
            <line x1="8" y1="6" x2="21" y2="6" />
            <line x1="8" y1="12" x2="21" y2="12" />
            <line x1="8" y1="18" x2="21" y2="18" />
            <line x1="3" y1="6" x2="3.01" y2="6" />
            <line x1="3" y1="12" x2="3.01" y2="12" />
            <line x1="3" y1="18" x2="3.01" y2="18" />
          </svg>
        </button>
        <button
          type="button"
          class="view-btn"
          class:active={settings.layout === "grid"}
          role="radio"
          aria-checked={settings.layout === "grid"}
          title="Grid view"
          aria-label="Grid view"
          onmousedown={preventFocusLossOnMouseDown}
          onclick={() => setLayout("grid")}
        >
          <svg
            width="13"
            height="13"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            aria-hidden="true"
          >
            <rect x="3" y="3" width="7" height="7" rx="1.5" />
            <rect x="14" y="3" width="7" height="7" rx="1.5" />
            <rect x="14" y="14" width="7" height="7" rx="1.5" />
            <rect x="3" y="14" width="7" height="7" rx="1.5" />
          </svg>
        </button>
      </div>
      <div class="bar-divider" aria-hidden="true"></div>
    {/if}

    {#if activeFilter === "all"}
      <div class="filter-group" role="toolbar" aria-label="Search filters">
        <button
          type="button"
          class="filter-btn"
          title="Search Applications (Ctrl+1)"
          aria-label="Filter: Applications"
          onmousedown={preventFocusLossOnMouseDown}
          onclick={() => toggleFilter("apps")}
        >
          <svg
            width="15"
            height="15"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
          >
            <rect x="3" y="3" width="18" height="18" rx="4" />
            <path d="M3 9h18M9 21V9" />
          </svg>
        </button>

        <button
          type="button"
          class="filter-btn"
          title="Search Files (Ctrl+2)"
          aria-label="Filter: Files and Folders"
          onmousedown={preventFocusLossOnMouseDown}
          onclick={() => toggleFilter("files")}
        >
          <svg
            width="15"
            height="15"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
          >
            <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
            <polyline points="14 2 14 8 20 8" />
          </svg>
        </button>

        <button
          type="button"
          class="filter-btn"
          title="Search Actions (Ctrl+3)"
          aria-label="Filter: Actions and Commands"
          onmousedown={preventFocusLossOnMouseDown}
          onclick={() => toggleFilter("actions")}
        >
          <svg
            width="15"
            height="15"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
          >
            <path d="M9 6a3 3 0 1 0-3 3h12a3 3 0 1 0-3-3v12a3 3 0 1 0 3-3H6a3 3 0 1 0 3 3z" />
          </svg>
        </button>
      </div>
    {/if}
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

  {#if activeFilter !== "all" && availableSubcategories.length > 1}
    <div class="subcategories-bar" role="tablist" aria-label="Subcategories">
      {#each availableSubcategories as subcat (subcat.id)}
        <button
          type="button"
          class="subcat-chip"
          class:active={activeSubcategory === subcat.id}
          role="tab"
          aria-selected={activeSubcategory === subcat.id}
          tabindex="0"
          onmousedown={preventFocusLossOnMouseDown}
          onclick={() => selectSubcategory(subcat.id)}
        >
          {subcat.label}
        </button>
      {/each}
    </div>
  {/if}



  <div class="body">
    <div id="list" role="listbox" aria-label="Results" bind:this={listContainer} onscroll={handleListScroll}>
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
          <div class="t">
            {#if effectiveQuery.trim()}
              {#if activeFilter === "apps"}
                No applications for “{effectiveQuery.trim()}”
              {:else if activeFilter === "files"}
                No files for “{effectiveQuery.trim()}”
              {:else if activeFilter === "actions"}
                No actions for “{effectiveQuery.trim()}”
              {:else}
                No results for “{effectiveQuery.trim()}”
              {/if}
            {:else}
              {#if activeFilter === "apps"}
                No applications available
              {:else if activeFilter === "files"}
                No files available
              {:else if activeFilter === "actions"}
                No actions available
              {:else}
                No results
              {/if}
            {/if}
          </div>
          <p>
            {#if effectiveQuery.trim()}
              Check the spelling, or try another search.
            {:else if activeSubcategory !== "all"}
              No items found in this category.
            {:else}
              No indexed items found.
            {/if}
          </p>
        </div>
      {/if}

      {#each visibleSections as section (section.title)}
        {@const isGrid =
          settings.layout === "grid" &&
          (section.category === "app" || section.category === "file")}
        <div class="sec">{section.title}</div>
        {#if isGrid}
          <div class="grid">
            {#each section.items as item, itemIdx (item.category + ":" + (item.path || item.name) + ":" + itemIdx)}
              {@const itemIndex = flatItems.indexOf(item)}
              {@const isSelected = itemIndex === selectedIndex}
              {@const highlighted = splitHighlight(item.name, query)}
              {@const disp = getItemDisplayIcon(item)}
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
                oncontextmenu={(e) => handleRowContextMenu(e, itemIndex)}
              >
                {#if disp.type === "thumb" && disp.src}
                  <img class="thumb" src={disp.src} alt="" />
                {:else if disp.type === "shell" && disp.src}
                  <img class="ico-img" src={disp.src} alt="" />
                {:else}
                  <div class="ico g" style={disp.color ? `color: ${disp.color};` : ""}>
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
                      <path d={disp.path ?? getCategoryIconPath(item)} />
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
          {#each section.items as item, itemIdx (item.category + ":" + (item.path || item.name) + ":" + itemIdx)}
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
                oncontextmenu={(e) => handleRowContextMenu(e, itemIndex)}
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
                  <div class="v">
                    {#if item.calc.colorPreview}
                      <span class="color-preview-chip" style:background-color={item.calc.colorPreview} aria-hidden="true"></span>
                    {/if}{item.calc.value}
                  </div>
                </div>
              </div>
            {:else}
              {@const highlighted = splitHighlight(item.name, query)}
              {@const disp = getItemDisplayIcon(item)}
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
                oncontextmenu={(e) => handleRowContextMenu(e, itemIndex)}
              >
                {#if disp.type === "thumb" && disp.src}
                  <img class="thumb" src={disp.src} alt="" />
                {:else if disp.type === "shell" && disp.src}
                  <img class="ico-img" src={disp.src} alt="" />
                {:else if isGradientThumbnail(item) || isClipThumbnail(item)}
                  <div class="thumb" style:background={item.thumbnail}></div>
                {:else}
                  <div class="ico g" style={disp.color ? `color: ${disp.color};` : ""}>
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
                      <path d={disp.path ?? getCategoryIconPath(item)} />
                    </svg>
                  </div>
                {/if}

                <div class="txt">
                  <span class="t">
                    {highlighted.before}{#if highlighted.match}<mark
                        >{highlighted.match}</mark
                      >{/if}{highlighted.after}
                  </span>
                  {#if item.subtitle && item.subtitle.trim().toLowerCase() !== "application" && item.subtitle.trim().toLowerCase() !== item.rightLabel?.trim().toLowerCase()}<span class="s">{item.subtitle}</span>{/if}
                </div>
                <span class="r">{item.rightLabel}</span>
              </div>
            {/if}
          {/each}
        {/if}
      {/each}
      {#if visibleCount < flatItems.length}
        <div class="sentinel" bind:this={sentinelElement} aria-hidden="true"></div>
      {/if}
    </div>
  </div>

  <div class="foot" data-tauri-drag-region>
    <div class="logo" aria-label="Lux">
      <svg viewBox="0 0 64 64" width="12" height="12" fill="currentColor">
        <path d="M 10 8 H 22 V 40 L 46 16 H 58 L 34 40 H 58 V 52 H 10 Z" />
      </svg>
    </div>
    <span id="msg">{displayedMessage}</span>
    <span class="foot-detail"><kbd>↑</kbd><kbd>↓</kbd> Select</span>
    {#if selectedItem?.path}
      <span class="foot-detail"><kbd>Space</kbd> Preview</span>
    {/if}
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
    width: 100%;
    height: 100%;
    margin: 0;
    padding: 0;
    overflow: hidden;
    background: transparent !important;
    background-color: transparent !important;
  }

  :global(body) {
    width: 100%;
    height: 100%;
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

  .win,
  .win * {
    box-sizing: border-box;
  }

  .win {
    width: 100%;
    height: 100%;
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
    width: 100%;
    height: 100%;
    flex: none;
    overflow: hidden;
  }

  .bar {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 64px;
    padding: 0 20px;
    border-bottom: 1px solid var(--line);
    flex: none;
    position: relative;
  }

  .bar-logo {
    width: 24px;
    height: 24px;
    border-radius: 6px;
    background: var(--fg);
    color: var(--bg);
    display: grid;
    place-items: center;
    flex: none;
  }

  .bar-logo svg {
    display: block;
  }

  .bar .search-icon {
    flex: none;
    color: var(--mut);
    width: 22px;
    height: 22px;
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

  .view-selector {
    display: inline-flex;
    align-items: center;
    background: var(--chip);
    border-radius: 6px;
    padding: 2px;
    gap: 2px;
    flex: none;
  }

  .view-btn {
    border: none;
    background: transparent;
    color: var(--mut);
    width: 24px;
    height: 24px;
    border-radius: 4px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    padding: 0;
    transition: background 0.12s ease, color 0.12s ease;
  }

  .view-btn:hover {
    color: var(--fg);
  }

  .view-btn.active {
    background: var(--bg);
    color: var(--fg);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.08);
  }

  .bar-divider {
    width: 1px;
    height: 18px;
    background: var(--line);
    margin: 0 4px;
    flex: none;
  }

  .filter-group {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    flex: none;
  }

  .cat-back-btn {
    border: none;
    background: transparent;
    padding: 0;
    margin: 0;
    cursor: pointer;
    flex: none;
    width: 28px;
    height: 28px;
    border-radius: 6px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    position: relative;
    color: var(--mut);
    transition: background 0.12s ease, color 0.12s ease;
  }

  .cat-back-btn:hover {
    background: var(--chip);
    color: var(--fg);
  }

  .cat-back-btn:focus-visible {
    outline: 2px solid var(--accent, #4a8fd8);
    outline-offset: 1px;
    background: var(--chip);
    color: var(--fg);
  }

  .cat-back-btn .icon-cat {
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .cat-back-btn .icon-arrow {
    display: none;
    align-items: center;
    justify-content: center;
  }

  .cat-back-btn:hover .icon-cat,
  .cat-back-btn:focus-visible .icon-cat {
    display: none;
  }

  .cat-back-btn:hover .icon-arrow,
  .cat-back-btn:focus-visible .icon-arrow {
    display: inline-flex;
  }

  .sentinel {
    width: 100%;
    height: 1px;
    pointer-events: none;
    visibility: hidden;
  }

  :global(:root[data-layout-density="compact"]) .cat-back-btn {
    width: 22px;
    height: 22px;
  }

  :global(:root[data-layout-density="comfortable"]) .cat-back-btn {
    width: 32px;
    height: 32px;
  }

  .subcategories-bar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 16px 8px;
    border-bottom: 1px solid var(--line);
    overflow-x: auto;
    scrollbar-width: none;
    flex: none;
    background: transparent;
  }

  .subcategories-bar::-webkit-scrollbar {
    display: none;
  }

  .subcat-chip {
    border: 1px solid transparent;
    background: transparent;
    color: var(--mut);
    font-size: 11.5px;
    font-weight: 500;
    line-height: 1;
    padding: 5px 10px;
    border-radius: 9999px;
    cursor: pointer;
    user-select: none;
    white-space: nowrap;
    transition: background 0.12s ease, color 0.12s ease, border-color 0.12s ease;
  }

  .subcat-chip:hover {
    background: var(--chip);
    color: var(--fg);
  }

  .subcat-chip:focus-visible {
    outline: 2px solid var(--accent, #4a8fd8);
    outline-offset: 1px;
  }

  .subcat-chip.active {
    background: var(--sel);
    color: var(--fg);
    border-color: var(--line);
    font-weight: 600;
  }

  :global(:root[data-layout-density="compact"]) .subcategories-bar {
    padding: 4px 12px 6px;
    gap: 4px;
  }

  :global(:root[data-layout-density="compact"]) .subcat-chip {
    font-size: 10.5px;
    padding: 3px 8px;
  }

  :global(:root[data-layout-density="comfortable"]) .subcategories-bar {
    padding: 8px 20px 10px;
    gap: 8px;
  }

  :global(:root[data-layout-density="comfortable"]) .subcat-chip {
    font-size: 12.5px;
    padding: 6px 12px;
  }

  .filter-btn {
    border: none;
    background: transparent;
    color: var(--mut);
    width: 28px;
    height: 28px;
    border-radius: 6px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    padding: 0;
    transition: background 0.12s ease, color 0.12s ease;
  }

  .filter-btn:hover {
    background: var(--chip);
    color: var(--fg);
  }


  @media (prefers-reduced-motion: reduce) {
    .view-btn,
    .filter-btn,
    .subcat-chip {
      transition: none !important;
    }
  }

  .idle:not(.collapsing) .body,
  .idle:not(.collapsing) .foot {
    display: none;
  }

  .idle:not(.collapsing) .bar {
    border-bottom-color: transparent;
  }

  .foot-detail {
    display: none;
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
    padding: 6px 10px 10px;
    scrollbar-width: thin;
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

  .color-preview-chip {
    display: inline-block;
    width: 18px;
    height: 18px;
    border-radius: 4px;
    margin-right: 8px;
    vertical-align: middle;
    border: 1px solid var(--line);
    flex-shrink: 0;
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
    padding: 0 20px 2px;
    margin-bottom: 2px;
    border-top: 1px solid var(--line);
    gap: 10px;
    color: var(--mut);
    font-size: 12px;
    flex: none;
    width: 100%;
  }

  #msg {
    max-width: 200px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  #pl {
    max-width: 220px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
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
    min-width: 8px;
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
    white-space: nowrap;
    flex-shrink: 0;
  }

  .btn:hover {
    background: var(--sel);
  }

  .g1,
  .g2 {
    flex-shrink: 0;
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
    right: 20px;
    bottom: 50px;
    width: 268px;
    max-height: calc(100vh - 120px);
    overflow-y: auto;
    scrollbar-width: thin;
    background: var(--bg);
    border-radius: 10px;
    box-shadow: var(--shadow);
    padding: 6px;
    display: none;
    z-index: 10;
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
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .pop .row {
    height: 34px !important;
    padding: 0 8px;
    gap: 8px;
  }

  .pop .row .t {
    flex: 1;
    min-width: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
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
    width: 100%;
    height: 100%;
    flex: 1;
    overflow: hidden;
  }

  /* Information Density: Minimal */
  :global(:root[data-info-density="minimal"]) .win.idle {
    justify-content: center;
  }

  :global(:root[data-info-density="minimal"]) .win.idle .bar {
    flex: none;
    border-bottom: 0;
  }



  :global(:root[data-info-density="minimal"]) .foot {
    display: none !important;
  }

  :global(:root[data-info-density="minimal"]) .pop {
    bottom: 16px;
  }

  :global(:root[data-info-density="minimal"]) .s {
    max-width: 200px;
    opacity: 0.6;
  }

  :global(:root[data-info-density="minimal"]) .r {
    opacity: 0.6;
  }

  /* Information Density: Detailed */
  :global(:root[data-info-density="detailed"]) .foot-detail {
    display: inline-flex;
  }

  :global(:root[data-info-density="detailed"]) .s {
    opacity: 0.9;
  }

  /* Layout Density: Compact */
  :global(:root[data-layout-density="compact"]) .bar {
    height: 52px;
    padding: 0 16px;
    gap: 8px;
  }

  :global(:root[data-layout-density="compact"]) .bar-logo {
    width: 20px;
    height: 20px;
    border-radius: 5px;
  }

  :global(:root[data-layout-density="compact"]) .bar-logo svg {
    width: 12px;
    height: 12px;
  }

  :global(:root[data-layout-density="compact"]) .bar .search-icon {
    width: 18px;
    height: 18px;
  }

  :global(:root[data-layout-density="compact"]) #q {
    font-size: 18px;
  }



  :global(:root[data-layout-density="compact"]) #list {
    padding: 4px 8px 8px;
  }

  :global(:root[data-layout-density="compact"]) #list .row {
    height: 36px;
    padding: 0 8px;
    gap: 8px;
  }

  :global(:root[data-layout-density="compact"]) .ico,
  :global(:root[data-layout-density="compact"]) .ico-img,
  :global(:root[data-layout-density="compact"]) .thumb {
    width: 24px;
    height: 24px;
    border-radius: 6px;
  }

  :global(:root[data-layout-density="compact"]) .t {
    font-size: 13px;
  }

  :global(:root[data-layout-density="compact"]) .s {
    font-size: 11.5px;
  }

  :global(:root[data-layout-density="compact"]) .foot {
    height: 38px;
    padding: 0 16px 2px;
    font-size: 11.5px;
    gap: 8px;
    margin-bottom: 2px;
  }

  :global(:root[data-layout-density="compact"]) .foot .logo {
    width: 16px;
    height: 16px;
    font-size: 10px;
    border-radius: 4px;
  }

  :global(:root[data-layout-density="compact"]) .foot .logo svg {
    width: 10px;
    height: 10px;
  }

  :global(:root[data-layout-density="compact"]) .btn {
    padding: 3px 5px;
    gap: 5px;
    font-size: 11.5px;
  }

  :global(:root[data-layout-density="compact"]) kbd {
    font-size: 10px;
    padding: 1px 5px;
    min-width: 18px;
  }

  :global(:root[data-layout-density="compact"]) .pop {
    right: 16px;
    bottom: 44px;
    width: 250px;
  }

  :global(:root[data-layout-density="compact"]) .pop .row {
    height: 30px !important;
    padding: 0 6px;
    gap: 6px;
    font-size: 12px;
  }

  :global(:root[data-info-density="minimal"][data-layout-density="compact"]) .pop {
    bottom: 12px;
  }

  /* Layout Density: Comfortable */
  :global(:root[data-layout-density="comfortable"]) .bar {
    height: 72px;
    padding: 0 24px;
    gap: 14px;
  }

  :global(:root[data-layout-density="comfortable"]) .bar-logo {
    width: 28px;
    height: 28px;
    border-radius: 7px;
  }

  :global(:root[data-layout-density="comfortable"]) .bar-logo svg {
    width: 16px;
    height: 16px;
  }

  :global(:root[data-layout-density="comfortable"]) .bar .search-icon {
    width: 24px;
    height: 24px;
  }

  :global(:root[data-layout-density="comfortable"]) #q {
    font-size: 24px;
  }



  :global(:root[data-layout-density="comfortable"]) #list {
    padding: 8px 12px 12px;
  }

  :global(:root[data-layout-density="comfortable"]) #list .row {
    height: 56px;
    padding: 0 12px;
    gap: 14px;
  }

  :global(:root[data-layout-density="comfortable"]) .ico,
  :global(:root[data-layout-density="comfortable"]) .ico-img,
  :global(:root[data-layout-density="comfortable"]) .thumb {
    width: 36px;
    height: 36px;
    border-radius: 9px;
  }

  :global(:root[data-layout-density="comfortable"]) .t {
    font-size: 15px;
  }

  :global(:root[data-layout-density="comfortable"]) .s {
    font-size: 13px;
  }

  :global(:root[data-layout-density="comfortable"]) .foot {
    height: 46px;
    padding: 0 24px 4px;
    font-size: 13px;
    gap: 12px;
    margin-bottom: 4px;
  }

  :global(:root[data-layout-density="comfortable"]) .foot .logo {
    width: 20px;
    height: 20px;
    font-size: 12px;
    border-radius: 6px;
  }

  :global(:root[data-layout-density="comfortable"]) .foot .logo svg {
    width: 14px;
    height: 14px;
  }

  :global(:root[data-layout-density="comfortable"]) .btn {
    padding: 5px 8px;
    gap: 8px;
    font-size: 13px;
  }

  :global(:root[data-layout-density="comfortable"]) kbd {
    font-size: 11.5px;
    padding: 3px 7px;
    min-width: 22px;
  }

  :global(:root[data-layout-density="comfortable"]) .pop {
    right: 24px;
    bottom: 56px;
    width: 280px;
  }

  :global(:root[data-layout-density="comfortable"]) .pop .row {
    height: 38px !important;
    padding: 0 10px;
    gap: 10px;
  }

  :global(:root[data-info-density="minimal"][data-layout-density="comfortable"]) .pop {
    bottom: 20px;
  }
</style>
