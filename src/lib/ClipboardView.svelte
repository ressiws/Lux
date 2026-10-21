<script lang="ts">
  import { onMount, tick } from "svelte";
  import type { ClipboardEntry, ClipboardKind } from "./types";
  import {
    clearClipboardHistory,
    copyClipboardEntry,
    deleteClipboardEntry,
    getClipboardHistory,
    hideLauncher,
    onClipboardUpdated,
    setWindowHeight,
    togglePinClipboardEntry,
  } from "./tauri";

  interface Props {
    onClose: () => void;
    onSwitchToLauncher: () => void;
  }

  const { onClose, onSwitchToLauncher }: Props = $props();

  const CLIPBOARD_WINDOW_HEIGHT = 560;

  const PATHS: Record<string, string> = {
    all: "M4 4h6v6H4z M14 4h6v6h-6z M4 14h6v6H4z M14 14h6v6h-6z",
    text: "M5 7h14M5 12h14M5 17h9",
    image: "M4 5h16v14H4z M4 16l5-5 4 4 3-3 4 4",
    file: "M7 3h7l4 4v14H7z M14 3v4h4",
    link: "M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1 M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1",
    color: "M12 3s6 6.5 6 11a6 6 0 0 1-12 0c0-4.5 6-11 6-11z",
    ck: "M5 12l4 4 10-10",
    pin: "M16 3l1 1-4 4 3 7-2 2-4-5-5 5-1-1 5-5-5-4 2-2 7 3 4-4 1 1z",
  };

  const FILTER_OPTIONS: Array<[ClipboardKind | "all", string, string]> = [
    ["all", "All Types", "search clipboard history"],
    ["text", "Text Only", "search text"],
    ["image", "Images Only", "search images"],
    ["file", "Files Only", "search files"],
    ["link", "Links Only", "search links"],
    ["color", "Colors Only", "search colors"],
  ];

  const PASTE_LABELS: Record<ClipboardKind, string> = {
    text: "Paste Text",
    image: "Paste Image",
    file: "Paste File",
    link: "Paste Link",
    color: "Paste Color",
  };

  let query = $state("");
  let activeFilter = $state<ClipboardKind | "all">("all");
  let filterMenuOpen = $state(false);
  let filterMenuIndex = $state(0);
  let selectedIndex = $state(0);
  let actionsOpen = $state(false);
  let selectedActionIndex = $state(0);
  let feedbackMessage = $state("");
  let feedbackTimer: ReturnType<typeof setTimeout> | undefined;

  let entries = $state<ClipboardEntry[]>([]);
  let searchInput: HTMLInputElement | undefined = $state();
  let listElement: HTMLDivElement | undefined = $state();

  function say(text: string): void {
    feedbackMessage = text;
    clearTimeout(feedbackTimer);
    feedbackTimer = setTimeout(() => {
      feedbackMessage = "";
    }, 1600);
  }

  function formatTimeAgo(timestampMs: number | null | undefined): string {
    if (typeof timestampMs !== "number" || isNaN(timestampMs) || timestampMs <= 0) {
      return "just now";
    }
    const diff = Math.max(0, Date.now() - timestampMs);
    const sec = Math.floor(diff / 1000);
    if (sec < 60) return "just now";
    const min = Math.floor(sec / 60);
    if (min < 60) return `${min} min ago`;
    const hours = Math.floor(min / 60);
    if (hours < 24) return `${hours} h ago`;
    const days = Math.floor(hours / 24);
    return `${days} d ago`;
  }

  function getDateGroup(timestampMs: number | null | undefined): "Today" | "Yesterday" | "Earlier" {
    if (typeof timestampMs !== "number" || isNaN(timestampMs) || timestampMs <= 0) {
      return "Today";
    }
    const now = new Date();
    const todayMidnight = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
    const yesterdayMidnight = todayMidnight - 86400000;

    if (timestampMs >= todayMidnight) return "Today";
    if (timestampMs >= yesterdayMidnight) return "Yesterday";
    return "Earlier";
  }

  function safeHostname(urlStr: string | null | undefined): string {
    if (!urlStr) return "";
    try {
      return new URL(urlStr).hostname;
    } catch {
      return urlStr;
    }
  }

  const filteredEntries = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return entries.filter((item) => {
      if (activeFilter !== "all" && item.kind !== activeFilter) {
        return false;
      }
      if (!q) return true;
      const haystack = [
        item.name,
        item.sourceApp || "",
        item.linkTitle || "",
        item.linkDesc || "",
        item.filePath || "",
      ]
        .join(" ")
        .toLowerCase();
      return haystack.includes(q);
    });
  });

  interface GroupedSection {
    readonly title: string;
    readonly items: readonly ClipboardEntry[];
  }

  const groupedSections = $derived.by<GroupedSection[]>(() => {
    const pinned = filteredEntries.filter((e) => e.isPinned);
    const unpinned = filteredEntries.filter((e) => !e.isPinned);

    const sections: GroupedSection[] = [];
    if (pinned.length > 0) {
      sections.push({ title: "Pinned", items: pinned });
    }

    const today = unpinned.filter((e) => getDateGroup(e.createdAt) === "Today");
    const yesterday = unpinned.filter((e) => getDateGroup(e.createdAt) === "Yesterday");
    const earlier = unpinned.filter((e) => getDateGroup(e.createdAt) === "Earlier");

    if (today.length > 0) sections.push({ title: "Today", items: today });
    if (yesterday.length > 0) sections.push({ title: "Yesterday", items: yesterday });
    if (earlier.length > 0) sections.push({ title: "Earlier", items: earlier });

    return sections;
  });

  const flatItems = $derived.by<ClipboardEntry[]>(() => {
    return groupedSections.flatMap((s) => s.items);
  });

  const selectedItem = $derived<ClipboardEntry | undefined>(
    flatItems[selectedIndex],
  );

  interface ActionOption {
    readonly label: string;
    readonly shortcut: string;
    readonly danger?: boolean;
    readonly action: () => void | Promise<void>;
  }

  const currentActions = $derived.by<ActionOption[]>(() => {
    if (!selectedItem) return [];
    const it = selectedItem;
    const actions: ActionOption[] = [];

    // Primary action
    actions.push({
      label: PASTE_LABELS[it.kind] || "Paste",
      shortcut: "↵",
      action: () => pasteItem(it),
    });

    // Content-specific secondary actions
    if (it.kind === "text") {
      actions.push({
        label: "Copy to Clipboard",
        shortcut: "Ctrl C",
        action: () => copyItem(it),
      });
      actions.push({
        label: "Paste as Plain Text",
        shortcut: "Ctrl ⇧ V",
        action: () => pasteItem(it),
      });
    } else if (it.kind === "image") {
      actions.push({
        label: "Copy Image",
        shortcut: "Ctrl C",
        action: () => copyItem(it),
      });
    } else if (it.kind === "file") {
      actions.push({
        label: "Copy File Path",
        shortcut: "Ctrl ⇧ C",
        action: () => {
          if (it.filePath) {
            navigator.clipboard.writeText(it.filePath);
            say("File path copied");
          }
        },
      });
    } else if (it.kind === "link") {
      actions.push({
        label: "Open in Browser",
        shortcut: "Ctrl O",
        action: async () => {
          if (it.linkUrl) {
            window.open(it.linkUrl, "_blank");
            await hideLauncher();
          }
        },
      });
      actions.push({
        label: "Copy as Markdown",
        shortcut: "Ctrl ⇧ C",
        action: () => {
          const title = it.linkTitle || it.name;
          const url = it.linkUrl || it.name;
          navigator.clipboard.writeText(`[${title}](${url})`);
          say("Markdown link copied");
        },
      });
    } else if (it.kind === "color") {
      actions.push({
        label: "Copy HEX",
        shortcut: "Ctrl C",
        action: () => {
          navigator.clipboard.writeText(it.name);
          say("HEX copied");
        },
      });
      if (it.colorRgb) {
        actions.push({
          label: "Copy RGB",
          shortcut: "Ctrl ⇧ C",
          action: () => {
            navigator.clipboard.writeText(it.colorRgb!);
            say("RGB copied");
          },
        });
      }
      if (it.colorHsl) {
        actions.push({
          label: "Copy HSL",
          shortcut: "Ctrl ⇧ H",
          action: () => {
            navigator.clipboard.writeText(it.colorHsl!);
            say("HSL copied");
          },
        });
      }
    }

    // Pinning
    actions.push({
      label: it.isPinned ? "Unpin Entry" : "Pin Entry",
      shortcut: "Ctrl P",
      action: () => togglePin(it),
    });

    // Delete
    actions.push({
      label: "Delete Entry",
      shortcut: "Del",
      danger: true,
      action: () => deleteItem(it),
    });

    // Clear all
    actions.push({
      label: "Clear History",
      shortcut: "",
      danger: true,
      action: () => clearAllHistory(),
    });

    return actions;
  });

  async function loadHistory(): Promise<void> {
    try {
      entries = await getClipboardHistory();
      if (selectedIndex >= flatItems.length) {
        selectedIndex = Math.max(0, flatItems.length - 1);
      }
    } catch (e) {
      console.error("Failed to load clipboard history:", e);
    }
  }

  async function pasteItem(item: ClipboardEntry): Promise<void> {
    try {
      await copyClipboardEntry(item.id);
      say(item.kind === "color" ? `Pasted ${item.name}` : "Pasted");
      await hideLauncher();
      onClose();
    } catch (err) {
      say(`Failed to paste: ${err}`);
    }
  }

  async function copyItem(item: ClipboardEntry): Promise<void> {
    try {
      await copyClipboardEntry(item.id);
      say(
        item.kind === "color"
          ? "HEX copied"
          : item.kind === "image"
            ? "Image copied"
            : "Copied to Clipboard",
      );
      actionsOpen = false;
    } catch (err) {
      say(`Failed to copy: ${err}`);
    }
  }

  async function togglePin(item: ClipboardEntry): Promise<void> {
    try {
      const isPinned = await togglePinClipboardEntry(item.id);
      say(isPinned ? "Pinned" : "Unpinned");
      await loadHistory();
      actionsOpen = false;
    } catch (err) {
      say(`Failed to pin: ${err}`);
    }
  }

  async function deleteItem(item: ClipboardEntry): Promise<void> {
    try {
      await deleteClipboardEntry(item.id);
      say("Deleted");
      actionsOpen = false;
      await loadHistory();
    } catch (err) {
      say(`Failed to delete: ${err}`);
    }
  }

  async function clearAllHistory(): Promise<void> {
    try {
      await clearClipboardHistory();
      say("History cleared");
      actionsOpen = false;
      await loadHistory();
    } catch (err) {
      say(`Failed to clear: ${err}`);
    }
  }

  function pickFilter(kind: ClipboardKind | "all"): void {
    activeFilter = kind;
    selectedIndex = 0;
    filterMenuOpen = false;
    searchInput?.focus();
  }

  function openFilterMenu(): void {
    actionsOpen = false;
    filterMenuIndex = FILTER_OPTIONS.findIndex(([k]) => k === activeFilter);
    filterMenuOpen = true;
  }

  function closeFilterMenu(): void {
    filterMenuOpen = false;
    searchInput?.focus();
  }

  function toggleActions(): void {
    if (actionsOpen) {
      actionsOpen = false;
    } else if (selectedItem) {
      filterMenuOpen = false;
      selectedActionIndex = 0;
      actionsOpen = true;
    }
  }

  function handleKeydown(e: KeyboardEvent): void {
    const k = e.key;

    // Filter menu shortcut: Ctrl + P (when actions popover is closed)
    if (e.ctrlKey && k.toLowerCase() === "p" && !actionsOpen) {
      e.preventDefault();
      if (filterMenuOpen) closeFilterMenu();
      else openFilterMenu();
      return;
    }

    // When filter menu is open
    if (filterMenuOpen) {
      if (k === "ArrowDown" || k === "ArrowUp") {
        e.preventDefault();
        const n = FILTER_OPTIONS.length;
        filterMenuIndex = (filterMenuIndex + (k === "ArrowDown" ? 1 : -1) + n) % n;
      } else if (k === "Enter") {
        e.preventDefault();
        pickFilter(FILTER_OPTIONS[filterMenuIndex][0]);
      } else if (k === "Escape") {
        e.preventDefault();
        closeFilterMenu();
      }
      return;
    }

    // Actions popover navigation
    if (actionsOpen) {
      const actionCount = currentActions.length;
      if (k === "ArrowDown" || k === "ArrowUp") {
        e.preventDefault();
        selectedActionIndex =
          (selectedActionIndex + (k === "ArrowDown" ? 1 : -1) + actionCount) % actionCount;
        return;
      }
      if (k === "Enter") {
        e.preventDefault();
        const action = currentActions[selectedActionIndex];
        action?.action();
        return;
      }
      if (k === "Escape" || (e.ctrlKey && k.toLowerCase() === "k") || k === "Tab") {
        e.preventDefault();
        actionsOpen = false;
        return;
      }
    }

    // List navigation
    const n = flatItems.length;
    if (k === "ArrowDown" || k === "ArrowUp") {
      e.preventDefault();
      if (n > 0) {
        selectedIndex = (selectedIndex + (k === "ArrowDown" ? 1 : -1) + n) % n;
        scrollSelectedIntoView();
      }
      return;
    }

    // Paste primary item
    if (k === "Enter") {
      e.preventDefault();
      if (selectedItem) {
        void pasteItem(selectedItem);
      }
      return;
    }

    // Copy item: Ctrl + C / Cmd + C (when not selecting text in search input)
    if ((e.ctrlKey || e.metaKey) && !e.altKey && !e.shiftKey && k.toLowerCase() === "c") {
      const isInputActive = document.activeElement === searchInput;
      const hasInputSelection =
        isInputActive &&
        searchInput &&
        searchInput.selectionStart !== searchInput.selectionEnd;

      if (!hasInputSelection && selectedItem) {
        e.preventDefault();
        void copyItem(selectedItem);
        return;
      }
    }

    // Secondary copy action: Ctrl + Shift + C
    if ((e.ctrlKey || e.metaKey) && !e.altKey && e.shiftKey && k.toLowerCase() === "c") {
      if (selectedItem) {
        if (selectedItem.kind === "file" && selectedItem.filePath) {
          e.preventDefault();
          void navigator.clipboard.writeText(selectedItem.filePath);
          say("File path copied");
          return;
        } else if (selectedItem.kind === "link") {
          e.preventDefault();
          const title = selectedItem.linkTitle || selectedItem.name;
          const url = selectedItem.linkUrl || selectedItem.name;
          void navigator.clipboard.writeText(`[${title}](${url})`);
          say("Markdown link copied");
          return;
        } else if (selectedItem.kind === "color" && selectedItem.colorRgb) {
          e.preventDefault();
          void navigator.clipboard.writeText(selectedItem.colorRgb);
          say("RGB copied");
          return;
        }
      }
    }

    // Toggle actions popover: Ctrl + K or Tab
    if (k === "Tab" || (e.ctrlKey && k.toLowerCase() === "k")) {
      e.preventDefault();
      toggleActions();
      return;
    }

    // Delete item directly with Delete key when search is empty
    if (k === "Delete" && query === "" && selectedItem) {
      e.preventDefault();
      void deleteItem(selectedItem);
      return;
    }

    // Escape closes popover, clears search, or hides launcher
    if (k === "Escape") {
      e.preventDefault();
      if (actionsOpen) {
        actionsOpen = false;
      } else if (query) {
        query = "";
        selectedIndex = 0;
      } else {
        onClose();
      }
      return;
    }
  }

  function scrollSelectedIntoView(): void {
    tick().then(() => {
      const activeEl = listElement?.querySelector(".row-item.on");
      activeEl?.scrollIntoView({ block: "nearest" });
    });
  }

  function getActiveFilterLabel(): string {
    const found = FILTER_OPTIONS.find(([k]) => k === activeFilter);
    return found ? found[1] : "All Types";
  }

  function getSearchPlaceholder(): string {
    if (activeFilter === "all") return "Search clipboard history…";
    const found = FILTER_OPTIONS.find(([k]) => k === activeFilter);
    return `Search ${found ? found[2].replace("search ", "") : "items"}…`;
  }

  function getColorLuminance(hex: string): number {
    const clean = hex.replace("#", "");
    if (clean.length < 6) return 255;
    const r = parseInt(clean.slice(0, 2), 16);
    const g = parseInt(clean.slice(2, 4), 16);
    const b = parseInt(clean.slice(4, 6), 16);
    return (r * 299 + g * 587 + b * 114) / 1000;
  }

  function handleCopy(e: ClipboardEvent): void {
    const isInputActive = document.activeElement === searchInput;
    const hasInputSelection =
      isInputActive &&
      searchInput &&
      searchInput.selectionStart !== searchInput.selectionEnd;

    if (hasInputSelection) {
      return;
    }

    const selection = window.getSelection();
    if (selection && !selection.isCollapsed) {
      const anchor = selection.anchorNode;
      const focus = selection.focusNode;
      const pv = document.getElementById("pv");
      if (pv && ((anchor && pv.contains(anchor)) || (focus && pv.contains(focus)))) {
        return;
      }
    }

    if (selectedItem) {
      e.preventDefault();
      void copyItem(selectedItem);
    }
  }

  onMount(() => {
    void setWindowHeight(CLIPBOARD_WINDOW_HEIGHT);
    void loadHistory();
    searchInput?.focus();

    let unlisten: (() => void) | undefined;
    void onClipboardUpdated(() => {
      void loadHistory();
    }).then((fn) => {
      unlisten = fn;
    });

    window.addEventListener("keydown", handleKeydown);
    window.addEventListener("copy", handleCopy);

    return () => {
      clearTimeout(feedbackTimer);
      unlisten?.();
      window.removeEventListener("keydown", handleKeydown);
      window.removeEventListener("copy", handleCopy);
    };
  });
</script>

<div class="clipboard-root" role="dialog" aria-label="Clipboard History">
  <!-- Top search bar -->
  <div class="bar" data-tauri-drag-region>
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="1.6"
      stroke-linecap="round"
      stroke-linejoin="round"
      aria-hidden="true"
    >
      <path d="M9 4h6v3H9z M7 5H6v16h12V5h-1" />
    </svg>
    <input
      id="q"
      bind:this={searchInput}
      bind:value={query}
      oninput={() => {
        selectedIndex = 0;
      }}
      autocomplete="off"
      spellcheck="false"
      placeholder={getSearchPlaceholder()}
      aria-label="Search clipboard history"
    />
    <button
      class="flt"
      id="flt"
      type="button"
      class:on={activeFilter !== "all"}
      aria-haspopup="listbox"
      aria-expanded={filterMenuOpen}
      title="Filter by type (Ctrl P)"
      onclick={() => (filterMenuOpen ? closeFilterMenu() : openFilterMenu())}
    >
      <span class="flt-icon">
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
          <path d={PATHS[activeFilter] || PATHS.all} />
        </svg>
      </span>
      <span>{getActiveFilterLabel()}</span>
      <svg
        width="14"
        height="14"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        stroke-linejoin="round"
      >
        <path d="M6 9l6 6 6-6" />
      </svg>
    </button>
  </div>

  <!-- Filter Dropdown Menu -->
  {#if filterMenuOpen}
    <div class="menu" id="menu" role="listbox" aria-label="Filter by type">
      <h6>Filter by type</h6>
      {#each FILTER_OPTIONS as [kind, label], i (kind)}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div
          class="mi"
          class:on={i === filterMenuIndex}
          role="option"
          tabindex="-1"
          aria-selected={activeFilter === kind}
          onmouseenter={() => (filterMenuIndex = i)}
          onclick={() => pickFilter(kind)}
        >
          <span class="ck">
            {#if activeFilter === kind}
              <svg
                width="15"
                height="15"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
              >
                <path d={PATHS.ck} />
              </svg>
            {/if}
          </span>
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
            <path d={PATHS[kind] || PATHS.all} />
          </svg>
          <span>{label}</span>
          <span class="n">
            {kind === "all"
              ? entries.length
              : entries.filter((e) => e.kind === kind).length}
          </span>
        </div>
        {#if i === 0}
          <div class="hs" aria-hidden="true"></div>
        {/if}
      {/each}
    </div>
  {/if}

  <!-- Split Body: 40% List / 60% Preview -->
  <div class="body">
    <div id="list" role="listbox" bind:this={listElement}>
      {#if flatItems.length === 0}
        <div class="empty">
          <div class="t">
            No {activeFilter === "all" ? "results" : activeFilter} found
          </div>
          <p>
            {query
              ? "Try a different search, or switch the type filter."
              : "Copy something and it will show up here."}
          </p>
        </div>
      {:else}
        {#each groupedSections as section (section.title)}
          <div class="sec">{section.title}</div>
          {#each section.items as item (item.id)}
            {@const globalIndex = flatItems.indexOf(item)}
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <div
              class="row row-item"
              class:on={globalIndex === selectedIndex}
              data-i={globalIndex}
              role="option"
              tabindex="-1"
              aria-selected={globalIndex === selectedIndex}
              onmousemove={() => {
                if (selectedIndex !== globalIndex) {
                  selectedIndex = globalIndex;
                }
              }}
              onclick={() => void pasteItem(item)}
            >
              {#if item.kind === "color"}
                <div class="sw" style="background: {item.name}"></div>
              {:else if item.kind === "image"}
                <div
                  class="thumb"
                  style="background: url('{item.preview || ''}') center/cover no-repeat"
                ></div>
              {:else}
                <div class="ico">
                  <svg
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.7"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  >
                    <path d={PATHS[item.kind] || PATHS.text} />
                  </svg>
                </div>
              {/if}

              <div class="txt">
                <span class="t">{item.name}</span>
                {#if item.sourceApp}
                  <span class="s">{item.sourceApp}</span>
                {/if}
              </div>

              {#if item.isPinned}
                <span class="pin-badge" title="Pinned">📌</span>
              {/if}

              <span class="r">{formatTimeAgo(item.createdAt)}</span>
            </div>
          {/each}
        {/each}
      {/if}
    </div>

    <!-- Preview Panel -->
    <aside class="pv" id="pv" aria-label="Preview">
      {#if selectedItem}
        <div class="pvbox">
          {#if selectedItem.kind === "color"}
            <div class="fill" style="background: {selectedItem.name}">
              <span
                class="hex"
                style="color: {getColorLuminance(selectedItem.name) > 150
                  ? '#111'
                  : '#fff'}"
              >
                {selectedItem.name}
              </span>
            </div>
          {:else if selectedItem.kind === "image"}
            {#if selectedItem.preview}
              <div
                class="fill"
                style="background: url('{selectedItem.preview}') center/contain no-repeat"
              ></div>
            {:else}
              <div class="stack">
                <div class="ico" style="width: 76px; height: 76px;">
                  <svg
                    width="32"
                    height="32"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.7"
                  >
                    <path d={PATHS.image} />
                  </svg>
                </div>
                <div class="cap">
                  <b>{selectedItem.name}</b>
                  <span>Image · {selectedItem.imageSize || "—"}</span>
                </div>
              </div>
            {/if}
          {:else if selectedItem.kind === "text"}
            <div class="ptxt">{selectedItem.name}</div>
          {:else if selectedItem.kind === "link"}
            <div class="lc">
              <div
                class="ban"
                style="background: linear-gradient(135deg,#2b2f3a,#566074)"
              ></div>
              <div class="lb">
                <div class="lh">
                  <span class="fv">
                    {(selectedItem.linkUrl
                      ? safeHostname(selectedItem.linkUrl)[0] || "L"
                      : "L"
                    ).toUpperCase()}
                  </span>
                  <span>
                    {selectedItem.linkUrl
                      ? safeHostname(selectedItem.linkUrl) || "Link"
                      : "Link"}
                  </span>
                </div>
                <div class="lt">
                  {selectedItem.linkTitle || selectedItem.name}
                </div>
                <div class="ld">
                  {selectedItem.linkDesc || selectedItem.name}
                </div>
              </div>
            </div>
          {:else if /\.pdf$/i.test(selectedItem.name)}
            <div class="page">
              <b></b>
              <i></i>
              <i></i>
              <i style="width: 80%"></i>
              <div class="bx"></div>
              <i></i>
              <i style="width: 60%"></i>
              <i></i>
              <em>PDF</em>
            </div>
          {:else}
            <div class="stack">
              <div class="ico" style="width: 76px; height: 76px;">
                <svg
                  width="32"
                  height="32"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="1.7"
                >
                  <path d={PATHS.file} />
                </svg>
              </div>
              <div class="cap">
                <b>{selectedItem.name}</b>
                <span>
                  {(selectedItem.fileType || "File").toUpperCase()} ·
                  {selectedItem.fileSize || "—"}
                </span>
              </div>
            </div>
          {/if}
        </div>

        <div class="pvi">
          <div class="ph">Information</div>
          {#if selectedItem.sourceApp}
            <div class="pm">
              <span>Application</span>
              <span>{selectedItem.sourceApp}</span>
            </div>
          {/if}
          <div class="pm">
            <span>Content Type</span>
            <span>
              {selectedItem.kind.charAt(0).toUpperCase() +
                selectedItem.kind.slice(1)}
            </span>
          </div>
          <div class="pm">
            <span>Copied</span>
            <span>{formatTimeAgo(selectedItem.createdAt)}</span>
          </div>

          {#if selectedItem.kind === "text"}
            {#if selectedItem.charCount != null}
              <div class="pm">
                <span>Characters</span>
                <span>{selectedItem.charCount}</span>
              </div>
            {/if}
            {#if selectedItem.wordCount != null}
              <div class="pm">
                <span>Words</span>
                <span>{selectedItem.wordCount}</span>
              </div>
            {/if}
          {:else if selectedItem.kind === "link"}
            {#if selectedItem.linkUrl}
              <div class="pm">
                <span>Domain</span>
                <span>{safeHostname(selectedItem.linkUrl)}</span>
              </div>
              <div class="pm">
                <span>Address</span>
                <span>{selectedItem.linkUrl}</span>
              </div>
            {/if}
          {:else if selectedItem.kind === "image"}
            {#if selectedItem.imageDims}
              <div class="pm">
                <span>Dimensions</span>
                <span>{selectedItem.imageDims}</span>
              </div>
            {/if}
            {#if selectedItem.imageSize}
              <div class="pm">
                <span>Size</span>
                <span>{selectedItem.imageSize}</span>
              </div>
            {/if}
          {:else if selectedItem.kind === "file"}
            {#if selectedItem.filePath}
              <div class="pm">
                <span>Path</span>
                <span>{selectedItem.filePath}</span>
              </div>
            {/if}
            {#if selectedItem.fileType}
              <div class="pm">
                <span>Kind</span>
                <span>{selectedItem.fileType.toUpperCase()}</span>
              </div>
            {/if}
            {#if selectedItem.fileSize}
              <div class="pm">
                <span>Size</span>
                <span>{selectedItem.fileSize}</span>
              </div>
            {/if}
          {:else if selectedItem.kind === "color"}
            <div class="pm">
              <span>HEX</span>
              <span>{selectedItem.name}</span>
            </div>
            {#if selectedItem.colorRgb}
              <div class="pm">
                <span>RGB</span>
                <span>{selectedItem.colorRgb}</span>
              </div>
            {/if}
            {#if selectedItem.colorHsl}
              <div class="pm">
                <span>HSL</span>
                <span>{selectedItem.colorHsl}</span>
              </div>
            {/if}
          {/if}
        </div>
      {/if}
    </aside>
  </div>

  <!-- Footer -->
  <div class="foot">
    <button
      class="logo-btn"
      type="button"
      title="Switch to Launcher (Alt Space)"
      onclick={onSwitchToLauncher}
    >
      <div class="logo">L</div>
    </button>
    <span id="cnt">
      {flatItems.length} {flatItems.length === 1 ? "item" : "items"}
    </span>
    <span class="sp"></span>
    <span id="msg">{feedbackMessage}</span>

    {#if selectedItem}
      <div class="g1" id="gp">
        <button
          class="btn"
          id="go"
          type="button"
          onclick={() => void pasteItem(selectedItem!)}
        >
          <span id="pl">{PASTE_LABELS[selectedItem.kind] || "Paste"}</span>
          <span class="k"><kbd>↵</kbd></span>
        </button>
      </div>
      <span class="vd" id="vd" aria-hidden="true"></span>
      <div class="g2" id="gs">
        <button
          class="btn"
          id="ab"
          type="button"
          onclick={toggleActions}
        >
          <span>Actions</span>
          <span class="k"><kbd>Ctrl</kbd><kbd>K</kbd></span>
        </button>
      </div>
    {/if}
  </div>

  <!-- Actions Popover (Ctrl K) -->
  {#if actionsOpen && selectedItem}
    <div class="pop open" id="pop">
      <h6>{selectedItem.name.slice(0, 32)}</h6>
      {#each currentActions as act, i (act.label)}
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <div
          class="row"
          class:dan={act.danger}
          class:on={i === selectedActionIndex}
          role="option"
          tabindex="-1"
          aria-selected={i === selectedActionIndex}
          onmousemove={() => (selectedActionIndex = i)}
          onclick={() => act.action()}
        >
          <span class="t">{act.label}</span>
          {#if act.shortcut}
            <span class="r k">
              {#each act.shortcut.split(" ").filter(Boolean) as key (key)}
                <kbd>{key}</kbd>
              {/each}
            </span>
          {/if}
        </div>
        {#if i === 0 && currentActions.length > 1}
          <div class="hs" aria-hidden="true"></div>
        {/if}
      {/each}
    </div>
  {/if}
</div>

<style>
  .clipboard-root {
    width: 100%;
    height: 100%;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    position: relative;
    user-select: none;
  }

  .bar {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 68px;
    padding: 0 14px 0 18px;
    border-bottom: 1px solid var(--line);
    flex: none;
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
    font: 400 22px Inter, "Segoe UI", system-ui, sans-serif;
    letter-spacing: -0.01em;
  }

  #q::placeholder {
    color: var(--dim);
  }

  .flt {
    display: flex;
    align-items: center;
    gap: 7px;
    height: 32px;
    padding: 0 8px 0 10px;
    border: 0;
    border-radius: 8px;
    background: var(--chip);
    color: var(--mut);
    font: 500 12.5px Inter, "Segoe UI", system-ui, sans-serif;
    cursor: pointer;
    flex: none;
  }

  .flt:hover,
  .flt[aria-expanded="true"] {
    background: var(--sel);
    color: var(--fg);
  }

  .flt.on {
    color: var(--fg);
  }

  .flt-icon {
    display: flex;
    align-items: center;
  }

  .menu {
    position: absolute;
    right: 12px;
    top: 62px;
    width: 226px;
    background: var(--glass);
    -webkit-backdrop-filter: blur(28px) saturate(1.6);
    backdrop-filter: blur(28px) saturate(1.6);
    border-radius: 10px;
    box-shadow: var(--shadow);
    padding: 6px;
    z-index: 6;
  }

  .menu h6 {
    margin: 0;
    font: 500 11.5px Inter, "Segoe UI", system-ui, sans-serif;
    color: var(--dim);
    padding: 6px 8px 4px;
  }

  .mi {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 34px;
    padding: 0 8px;
    border-radius: 7px;
    cursor: default;
  }

  .mi.on {
    background: var(--sel);
  }

  .mi svg {
    color: var(--mut);
    flex: none;
  }

  .mi .n {
    margin-left: auto;
    color: var(--dim);
    font-size: 12px;
  }

  .mi .ck {
    width: 15px;
    color: var(--fg);
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .hs {
    height: 1px;
    background: var(--line);
    margin: 4px 6px;
  }

  .body {
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  #list {
    flex: 0 0 40%;
    min-width: 0;
    overflow-y: auto;
    padding: 4px 10px 10px;
    scrollbar-width: thin;
    user-select: none;
    -webkit-user-select: none;
  }

  .sec {
    font-size: 12px;
    font-weight: 500;
    color: var(--dim);
    padding: 12px 10px 6px;
    user-select: none;
    -webkit-user-select: none;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    height: 48px;
    padding: 0 10px;
    border-radius: 10px;
    cursor: default;
    user-select: none;
    -webkit-user-select: none;
  }

  .row.on {
    background: var(--sel);
  }

  .ico,
  .thumb,
  .sw {
    flex: none;
    width: 30px;
    height: 30px;
    border-radius: 50%;
    user-select: none;
    -webkit-user-select: none;
    pointer-events: none;
  }

  .ico {
    display: grid;
    place-items: center;
    background: var(--chip);
    color: var(--mut);
  }

  .thumb,
  .sw {
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.14);
  }

  .txt {
    display: flex;
    gap: 9px;
    align-items: baseline;
    min-width: 0;
    flex: 1;
  }

  .t {
    font-weight: 500;
    font-size: 14px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .s {
    color: var(--mut);
    white-space: nowrap;
    font-size: 12.5px;
    flex: none;
    user-select: none;
    -webkit-user-select: none;
    pointer-events: none;
  }

  .pin-badge {
    font-size: 12px;
    flex: none;
    user-select: none;
    -webkit-user-select: none;
    pointer-events: none;
  }

  .r {
    margin-left: auto;
    color: var(--dim);
    font-size: 12px;
    flex: none;
    padding-left: 8px;
    user-select: none;
    -webkit-user-select: none;
    pointer-events: none;
  }

  .pv {
    width: 60%;
    flex: none;
    border-left: 1px solid var(--line);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .pvbox {
    flex: 0 0 54%;
    width: 100%;
    box-sizing: border-box;
    display: grid;
    place-items: center;
    background: var(--sel);
    overflow: hidden;
  }

  .fill {
    width: 100%;
    height: 100%;
    display: grid;
    place-items: center;
  }

  .hex {
    font: 600 22px Inter, "Segoe UI", system-ui, sans-serif;
    letter-spacing: 0.02em;
  }

  .ph {
    font-size: 12px;
    font-weight: 500;
    color: var(--dim);
    padding: 0 10px 5px;
  }

  .stack {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 16px;
    width: 100%;
  }

  .cap {
    margin-top: 12px;
    text-align: center;
    max-width: 92%;
  }

  .cap b {
    display: block;
    font-weight: 600;
    font-size: 14px;
    word-break: break-all;
  }

  .cap span {
    color: var(--mut);
    font-size: 12.5px;
    word-break: break-all;
  }

  .page {
    width: 172px;
    height: 236px;
    background: #fff;
    border-radius: 6px;
    box-shadow:
      0 0 0 1px rgba(0, 0, 0, 0.12),
      0 8px 24px -8px rgba(0, 0, 0, 0.35);
    padding: 22px 18px;
    position: relative;
  }

  .page b {
    display: block;
    height: 9px;
    width: 60%;
    background: #2b2f3a;
    border-radius: 2px;
    margin-bottom: 14px;
  }

  .page i {
    display: block;
    height: 5px;
    border-radius: 2px;
    background: #c9ccd3;
    margin-bottom: 8px;
  }

  .page .bx {
    height: 56px;
    border-radius: 4px;
    background: #e4e6eb;
    margin: 12px 0;
  }

  .page em {
    position: absolute;
    right: 10px;
    bottom: 10px;
    font: 600 10px Inter, sans-serif;
    color: #e5484d;
    font-style: normal;
  }

  .lc {
    width: 82%;
    background: var(--bg);
    border-radius: 12px;
    box-shadow: 0 0 0 1px var(--line);
    overflow: hidden;
    text-align: left;
  }

  .lc .ban {
    height: 64px;
  }

  .lc .lb {
    padding: 12px 14px;
  }

  .lc .lh {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--mut);
    font-size: 12px;
    margin-bottom: 6px;
  }

  .lc .fv {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--fg);
    color: var(--bg);
    display: grid;
    place-items: center;
    font-size: 10px;
    font-weight: 600;
  }

  .lc .lt {
    font-weight: 600;
    font-size: 14px;
    margin-bottom: 4px;
  }

  .lc .ld {
    color: var(--mut);
    font-size: 12.5px;
    line-height: 1.4;
  }

  .ptxt {
    box-sizing: border-box;
    width: 100%;
    height: 100%;
    align-self: stretch;
    justify-self: stretch;
    font-size: 13px;
    line-height: 1.55;
    padding: 20px 22px;
    max-height: 100%;
    overflow-y: auto;
    word-break: break-word;
    white-space: pre-wrap;
    text-align: left;
  }

  .pvi {
    box-sizing: border-box;
    padding: 10px 22px 14px 16px;
    overflow-y: auto;
    scrollbar-width: thin;
    flex: 1;
    user-select: none;
    -webkit-user-select: none;
  }

  .pm {
    box-sizing: border-box;
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 12px;
    padding: 5px 10px;
    position: relative;
    font-size: 12px;
    line-height: 1.3;
    color: var(--mut);
  }

  .pm span:first-child {
    flex-shrink: 0;
  }

  .pm + .pm::before {
    content: "";
    position: absolute;
    top: 0;
    left: 10px;
    right: 10px;
    height: 1px;
    background: var(--line);
  }

  .pm span:last-child {
    color: var(--fg);
    text-align: right;
    word-break: break-all;
    min-width: 0;
  }

  .empty {
    padding: 44px 16px;
    text-align: center;
    flex: 1;
  }

  .empty p {
    color: var(--mut);
    margin-top: 4px;
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

  .logo-btn {
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
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
  }

  .sp {
    flex: 1;
  }

  .g1,
  .g2 {
    display: flex;
    align-items: center;
  }

  .btn {
    display: inline-flex;
    gap: 8px;
    align-items: center;
    cursor: pointer;
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

  .k {
    display: inline-flex;
    gap: 3px;
  }

  kbd {
    font: 500 11px Inter, "Segoe UI", system-ui, sans-serif;
    background: var(--kbd);
    color: var(--mut);
    padding: 2px 6px;
    border-radius: 5px;
    min-width: 20px;
    text-align: center;
  }

  .pop {
    position: absolute;
    right: 10px;
    bottom: 48px;
    width: 268px;
    background: var(--glass);
    -webkit-backdrop-filter: blur(28px);
    backdrop-filter: blur(28px);
    border-radius: 10px;
    box-shadow: var(--shadow);
    padding: 6px;
    z-index: 4;
  }

  .pop .row {
    height: 34px;
  }

  .pop .row.dan .t {
    color: var(--acc);
  }

  .pop h6 {
    margin: 0;
    font: 500 11.5px Inter, "Segoe UI", system-ui, sans-serif;
    color: var(--dim);
    padding: 6px 8px 4px;
  }

  /* Information Density: Minimal hides footer navigation hints and metadata noise */
  :global(:root[data-info-density="minimal"]) .foot {
    display: none !important;
  }

  :global(:root[data-info-density="minimal"]) .s {
    opacity: 0.6;
  }

  /* Information Density: Detailed provides clearer subtitles */
  :global(:root[data-info-density="detailed"]) .s {
    opacity: 0.9;
  }

  /* Layout Density: Compact reduces dimensions, row heights and padding */
  :global(:root[data-layout-density="compact"]) .bar {
    height: 52px;
    padding: 0 14px;
    gap: 8px;
  }

  :global(:root[data-layout-density="compact"]) #q {
    font-size: 18px;
  }

  :global(:root[data-layout-density="compact"]) .row {
    height: 36px;
    padding: 0 8px;
    gap: 8px;
  }

  :global(:root[data-layout-density="compact"]) .ico,
  :global(:root[data-layout-density="compact"]) .thumb,
  :global(:root[data-layout-density="compact"]) .sw {
    width: 22px;
    height: 22px;
  }

  :global(:root[data-layout-density="compact"]) .t {
    font-size: 13px;
  }

  :global(:root[data-layout-density="compact"]) .s {
    font-size: 11.5px;
  }

  /* Layout Density: Comfortable provides spacious rows and larger touch/click targets */
  :global(:root[data-layout-density="comfortable"]) .bar {
    height: 70px;
    padding: 0 20px;
  }

  :global(:root[data-layout-density="comfortable"]) .row {
    height: 56px;
    padding: 0 14px;
    gap: 14px;
  }

  :global(:root[data-layout-density="comfortable"]) .ico,
  :global(:root[data-layout-density="comfortable"]) .thumb,
  :global(:root[data-layout-density="comfortable"]) .sw {
    width: 36px;
    height: 36px;
  }

  :global(:root[data-layout-density="comfortable"]) .t {
    font-size: 15px;
  }

  :global(:root[data-layout-density="comfortable"]) .s {
    font-size: 13px;
  }
</style>
