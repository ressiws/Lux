<script lang="ts">
  import { onMount, tick } from "svelte";
  import {
    CATEGORY_ICONS,
    buildSections,
    hasOnlyFallbackSections,
    splitHighlight,
  } from "$lib/mock-data";
  import {
    getLauncherStatus,
    hideLauncher,
    onLauncherHidden,
    onLauncherShown,
    openTarget,
    openTargetAsAdmin,
    revealItemInFolder,
    searchLauncher,
  } from "$lib/tauri";
  import type { ItemAction, LauncherItem, SearchResponse } from "$lib/types";

  let query = $state("");
  let selectedIndex = $state(0);
  let selectedActionIndex = $state(0);
  let actionsOpen = $state(false);
  let feedbackMessage = $state("");
  let shortcutWarning = $state("");
  let useNoBlurFallback = $state(false);
  let searchResponse = $state<SearchResponse | undefined>();

  let searchInput: HTMLInputElement | undefined = $state();
  let listContainer: HTMLDivElement | undefined = $state();
  let feedbackTimer: ReturnType<typeof setTimeout> | undefined;
  let searchSeq = 0;

  const sections = $derived(buildSections(query, searchResponse));
  const flatItems = $derived(sections.flatMap((section) => section.items));
  const selectedItem = $derived<LauncherItem | undefined>(
    flatItems[selectedIndex],
  );
  const showEmptyState = $derived(
    query.trim().length > 0 && hasOnlyFallbackSections(sections),
  );
  const displayedMessage = $derived(feedbackMessage || shortcutWarning);

  async function performSearch(q: string): Promise<void> {
    const seq = ++searchSeq;
    try {
      const resp = await searchLauncher(q);
      if (seq === searchSeq) {
        searchResponse = resp;
      }
    } catch {
      // Ignore if Tauri invocation is unavailable
    }
  }

  function focusSearchInput(): void {
    searchInput?.focus();
    requestAnimationFrame(() => {
      searchInput?.focus();
    });
  }

  function resetLauncherState(shouldFocus: boolean): void {
    actionsOpen = false;
    selectedActionIndex = 0;
    query = "";
    selectedIndex = 0;
    clearTimeout(feedbackTimer);
    feedbackMessage = "";
    void performSearch("");
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

  function toggleTheme(): void {
    const root = document.documentElement;
    const isDark = root.dataset.theme
      ? root.dataset.theme === "dark"
      : window.matchMedia("(prefers-color-scheme: dark)").matches;
    root.dataset.theme = isDark ? "light" : "dark";
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
    if (item.name === "Toggle Theme") {
      toggleTheme();
      say("Theme changed");
      return;
    }
    if (item.category === "calc" && item.calc) {
      await copyAndAnnounce(item.calc.raw, item.calc.raw);
      return;
    }
    if (item.category === "clip" || item.category === "snip") {
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

  async function executeItemAction(item: LauncherItem, action?: ItemAction): Promise<void> {
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
    void performSearch(query);
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
      selectedIndex = (selectedIndex + 1) % Math.max(count, 1);
      void scrollSelectedIntoView();
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      selectedIndex = (selectedIndex - 1 + count) % Math.max(count, 1);
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
      void dismissLauncher();
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
    if (item.category === "app" || item.category === "calc") {
      return CATEGORY_ICONS.cmd;
    }
    return CATEGORY_ICONS[item.category] ?? CATEGORY_ICONS.cmd;
  }

  $effect(() => {
    document.body.classList.toggle("nofx", useNoBlurFallback);
  });

  onMount(() => {
    focusSearchInput();

    let disposed = false;
    let unlistenShown: (() => void) | undefined;
    let unlistenHidden: (() => void) | undefined;

    void getLauncherStatus()
      .then((status) => {
        if (disposed) {
          return;
        }
        useNoBlurFallback = !status.nativeGlassSupported;
        if (!status.shortcut.registered) {
          shortcutWarning = `Shortcut ${status.shortcut.shortcut} unavailable`;
        }
      })
      .catch(() => {
        if (!disposed) {
          useNoBlurFallback = false;
        }
      });

    void onLauncherShown(() => {
      resetLauncherState(true);
    }).then((unlisten) => {
      if (disposed) {
        unlisten();
      } else {
        unlistenShown = unlisten;
      }
    });

    void onLauncherHidden(() => {
      resetLauncherState(false);
    }).then((unlisten) => {
      if (disposed) {
        unlisten();
      } else {
        unlistenHidden = unlisten;
      }
    });

    const handleWindowFocus = (): void => {
      focusSearchInput();
    };

    window.addEventListener("focus", handleWindowFocus);
    window.addEventListener("keydown", handleKeydown);

    return () => {
      disposed = true;
      clearTimeout(feedbackTimer);
      unlistenShown?.();
      unlistenHidden?.();
      window.removeEventListener("focus", handleWindowFocus);
      window.removeEventListener("keydown", handleKeydown);
    };
  });
</script>

<div class="win" id="win" role="dialog" aria-label="Lux command palette">
  <div class="bar" id="bar">
    <svg
      width="20"
      height="20"
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
  </div>

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
        <div class="t">No results for “{query.trim()}”</div>
        <p>Check the spelling, or try one of these.</p>
      </div>
    {/if}

    {#each sections as section (section.title)}
      <div class="sec">{section.title}</div>
      {#each section.items as item (item.category + ":" + item.name)}
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
                <div class="ico" style:background={item.color ?? "#4a8fd8"}>
                  {item.name[0]}
                </div>
              {/if}
            {:else if item.isImage && item.thumbnail}
              <img class="thumb" src={item.thumbnail} alt="" />
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
                {highlighted.before}{#if highlighted.match}<mark>{highlighted.match}</mark>{/if}{highlighted.after}
              </span>
              <span class="s">{item.subtitle}</span>
            </div>
            <span class="r">{item.rightLabel}</span>
          </div>
        {/if}
      {/each}
    {/each}
  </div>

  <div class="foot">
    <div class="logo" aria-hidden="true">L</div>
    <span id="msg" aria-live="polite">{displayedMessage}</span>
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
        {#if actionIndex === 0}
          <div class="hs"></div>
        {/if}
      {/each}
    {/if}
  </div>
</div>

<style>
  :global(:root) {
    --bg: rgba(250, 250, 250, 0.97);
    --bgt: rgba(250, 250, 250, 0.72);
    --fg: #1d1d1f;
    --mut: #6e6e73;
    --dim: #9a9aa0;
    --line: rgba(0, 0, 0, 0.08);
    --sel: rgba(0, 0, 0, 0.07);
    --chip: rgba(0, 0, 0, 0.06);
    --kbd: rgba(0, 0, 0, 0.07);
    --acc: #e5484d;
    --win-border: rgba(0, 0, 0, 0.12);
    --shadow:
      inset 0 0 0 1px var(--win-border),
      inset 0 1px 0 var(--hi),
      0 20px 48px -12px rgba(0, 0, 0, 0.25);
    --tile: #fff;
    --glass: rgba(252, 252, 254, 0.55);
    --pop-glass: rgba(252, 252, 254, 0.82);
    --gfb: rgba(250, 250, 252, 0.94);
    --hi: rgba(255, 255, 255, 0.65);
    box-sizing: border-box;
    padding-top: env(safe-area-inset-top, 0px);
    padding-bottom: env(safe-area-inset-bottom, 0px);
    color-scheme: light;
  }

  @media (prefers-color-scheme: dark) {
    :global(:root:not([data-theme="light"])) {
      --bg: rgba(18, 18, 22, 0.98);
      --bgt: rgba(18, 18, 22, 0.72);
      --fg: #ededef;
      --mut: #9b9ba1;
      --dim: #6b6b72;
      --line: rgba(255, 255, 255, 0.08);
      --sel: rgba(255, 255, 255, 0.08);
      --chip: rgba(255, 255, 255, 0.08);
      --kbd: rgba(255, 255, 255, 0.1);
      --acc: #e5484d;
      --win-border: rgba(255, 255, 255, 0.12);
      --shadow:
        inset 0 0 0 1px var(--win-border),
        inset 0 1px 0 var(--hi),
        0 20px 48px -12px rgba(0, 0, 0, 0.55);
      --glass: rgba(16, 16, 20, 0.35);
      --pop-glass: rgba(20, 20, 26, 0.78);
      --gfb: rgba(20, 20, 24, 0.92);
      --hi: rgba(255, 255, 255, 0.09);
      color-scheme: dark;
    }
  }

  :global(:root[data-theme="dark"]) {
    --bg: rgba(18, 18, 22, 0.98);
    --bgt: rgba(18, 18, 22, 0.72);
    --fg: #ededef;
    --mut: #9b9ba1;
    --dim: #6b6b72;
    --line: rgba(255, 255, 255, 0.08);
    --sel: rgba(255, 255, 255, 0.08);
    --chip: rgba(255, 255, 255, 0.08);
    --kbd: rgba(255, 255, 255, 0.1);
    --acc: #e5484d;
    --win-border: rgba(255, 255, 255, 0.12);
    --shadow:
      inset 0 0 0 1px var(--win-border),
      inset 0 1px 0 var(--hi),
      0 20px 48px -12px rgba(0, 0, 0, 0.55);
    --glass: rgba(16, 16, 20, 0.35);
    --pop-glass: rgba(20, 20, 26, 0.78);
    --gfb: rgba(20, 20, 24, 0.92);
    --hi: rgba(255, 255, 255, 0.09);
    color-scheme: dark;
  }

  :global(*) {
    box-sizing: border-box;
    margin: 0;
  }

  :global(html),
  :global(body) {
    height: 100%;
    overflow: hidden;
    background: transparent;
  }

  :global(body) {
    font:
      400 13.5px/1.35 Inter,
      "Segoe UI",
      Cantarell,
      system-ui,
      sans-serif;
    color: var(--fg);
    -webkit-font-smoothing: antialiased;
    user-select: none;
  }

  :global(:focus-visible) {
    outline: 2px solid var(--mut);
    outline-offset: 1px;
  }

  .win {
    width: min(680px, 100%);
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--glass);
    -webkit-backdrop-filter: blur(28px) saturate(1.6);
    backdrop-filter: blur(28px) saturate(1.6);
    border-radius: 14px;
    box-shadow:
      inset 0 0 0 1px var(--win-border),
      inset 0 1px 0 var(--hi),
      0 20px 48px -12px rgba(0, 0, 0, 0.5);
    overflow: hidden;
    position: relative;
  }

  .bar {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 56px;
    padding: 0 16px;
    border-bottom: 1px solid var(--line);
    flex: none;
  }

  .bar svg {
    flex: none;
    color: var(--mut);
  }

  #q {
    flex: 1;
    min-width: 0;
    background: none;
    border: 0;
    outline: 0;
    color: var(--fg);
    font:
      400 17px Inter,
      "Segoe UI",
      system-ui,
      sans-serif;
    letter-spacing: -0.01em;
  }

  #q::placeholder {
    color: var(--dim);
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

  #list {
    flex: 1;
    max-height: 392px;
    overflow-y: auto;
    padding: 6px 8px 8px;
    scrollbar-width: thin;
  }

  .sec {
    font-size: 11.5px;
    font-weight: 500;
    color: var(--dim);
    padding: 10px 8px 5px;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 11px;
    height: 42px;
    padding: 0 8px;
    border-radius: 8px;
    cursor: default;
  }

  .row.on {
    background: var(--sel);
  }

  .ico {
    flex: none;
    width: 26px;
    height: 26px;
    border-radius: 7px;
    display: grid;
    place-items: center;
    color: #fff;
    font-weight: 600;
    font-size: 12px;
  }

  .ico-img {
    width: 26px;
    height: 26px;
    border-radius: 7px;
    flex: none;
    object-fit: contain;
  }

  .ico.g {
    background: var(--chip);
    color: var(--mut);
  }

  .thumb {
    width: 26px;
    height: 26px;
    border-radius: 5px;
    flex: none;
    box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.12);
    object-fit: cover;
  }

  .t {
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
    gap: 0;
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
    gap: 8px;
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

  .g1,
  .g2 {
    display: flex;
    align-items: center;
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
    background: var(--pop-glass);
    -webkit-backdrop-filter: blur(28px) saturate(1.6);
    backdrop-filter: blur(28px) saturate(1.6);
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

  @supports not (
    (backdrop-filter: blur(1px)) or (-webkit-backdrop-filter: blur(1px))
  ) {
    .win,
    .pop {
      background: var(--gfb);
    }
  }

  :global(.nofx) .win {
    background: var(--gfb);
    border-radius: 14px;
    box-shadow:
      inset 0 0 0 1px var(--win-border),
      inset 0 1px 0 var(--hi);
  }

  :global(.nofx) .pop {
    background: var(--gfb);
    -webkit-backdrop-filter: none;
    backdrop-filter: none;
  }

  @media (prefers-reduced-motion: reduce) {
    .spin {
      animation: none;
    }
  }

  @media (max-width: 520px) {
    .r {
      display: none;
    }
  }
</style>
