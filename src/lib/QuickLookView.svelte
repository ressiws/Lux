<script lang="ts">
  import { onMount } from "svelte";
  import { convertFileSrc } from "@tauri-apps/api/core";
  import {
    hidePreview,
    getFilePreview,
    openTarget,
    revealItemInFolder,
    onOpenPreview,
  } from "./tauri";
  import type { FilePreviewInfo, FileFormatKind } from "./types";
  import { FORMAT_ICONS, FORMAT_COLORS } from "./mock-data";

  let filePath = $state("");
  let previewInfo = $state<FilePreviewInfo | null>(null);
  let isLoading = $state(true);
  let activeMarkdownTab = $state<"rendered" | "raw">("rendered");
  let copyFeedback = $state(false);

  export function setFilePath(path: string): void {
    filePath = path;
    void loadPreview(path);
  }

  async function loadPreview(path: string): Promise<void> {
    if (!path) {
      return;
    }
    isLoading = true;
    try {
      const info = await getFilePreview(path);
      previewInfo = info;
    } catch (err) {
      previewInfo = {
        path,
        filename: path.split(/[/\\]/).pop() || path,
        fileSizeBytes: 0,
        fileSizeFormatted: "0 B",
        formatKind: "generic",
        mimeOrTypeLabel: "File",
        modifiedTimestamp: null,
        modifiedFormatted: null,
        textContent: null,
        isTruncated: false,
        lineCount: null,
        dimensions: null,
        pageCount: null,
        archiveEntries: null,
        error: String(err),
      };
    } finally {
      isLoading = false;
    }
  }

  async function handleClose(): Promise<void> {
    try {
      await hidePreview();
    } catch {
      // Ignore
    }
  }

  async function handleOpenTarget(): Promise<void> {
    if (!previewInfo?.path) return;
    try {
      await openTarget(previewInfo.path);
      await handleClose();
    } catch {
      // Ignore
    }
  }

  async function handleReveal(): Promise<void> {
    if (!previewInfo?.path) return;
    try {
      await revealItemInFolder(previewInfo.path);
      await handleClose();
    } catch {
      // Ignore
    }
  }

  async function handleCopyPath(): Promise<void> {
    if (!previewInfo?.path) return;
    try {
      await navigator.clipboard.writeText(previewInfo.path);
      copyFeedback = true;
      setTimeout(() => {
        copyFeedback = false;
      }, 1500);
    } catch {
      // Ignore
    }
  }

  function handleKeydown(event: KeyboardEvent): void {
    if (event.key === "Escape" || event.key === " ") {
      event.preventDefault();
      event.stopPropagation();
      void handleClose();
      return;
    }

    if (event.key === "Enter") {
      event.preventDefault();
      if ((event.ctrlKey || event.metaKey) && !event.shiftKey) {
        void handleReveal();
      } else {
        void handleOpenTarget();
      }
      return;
    }

    if (
      (event.ctrlKey || event.metaKey) &&
      event.shiftKey &&
      event.key.toLowerCase() === "c"
    ) {
      event.preventDefault();
      void handleCopyPath();
      return;
    }
  }

  function renderSafeMarkdown(raw: string): string {
    // Escape raw HTML entities to strictly prevent script injection
    const escaped = raw
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;");

    const lines = escaped.split("\n");
    const output: string[] = [];
    let inCodeBlock = false;
    let codeBlockContent: string[] = [];
    let inList = false;

    for (const line of lines) {
      if (line.trim().startsWith("```")) {
        if (inCodeBlock) {
          inCodeBlock = false;
          output.push(
            `<pre class="ql-code-block"><code>${codeBlockContent.join("\n")}</code></pre>`,
          );
          codeBlockContent = [];
        } else {
          inCodeBlock = true;
        }
        continue;
      }

      if (inCodeBlock) {
        codeBlockContent.push(line);
        continue;
      }

      if (line.startsWith("# ")) {
        output.push(`<h1>${formatInlineMarkdown(line.slice(2))}</h1>`);
      } else if (line.startsWith("## ")) {
        output.push(`<h2>${formatInlineMarkdown(line.slice(3))}</h2>`);
      } else if (line.startsWith("### ")) {
        output.push(`<h3>${formatInlineMarkdown(line.slice(4))}</h3>`);
      } else if (line.startsWith("#### ")) {
        output.push(`<h4>${formatInlineMarkdown(line.slice(5))}</h4>`);
      } else if (line.startsWith("> ")) {
        output.push(`<blockquote>${formatInlineMarkdown(line.slice(2))}</blockquote>`);
      } else if (line.startsWith("- ") || line.startsWith("* ")) {
        if (!inList) {
          inList = true;
          output.push("<ul>");
        }
        output.push(`<li>${formatInlineMarkdown(line.slice(2))}</li>`);
      } else {
        if (inList) {
          inList = false;
          output.push("</ul>");
        }
        if (line.trim().length === 0) {
          output.push("<br/>");
        } else {
          output.push(`<p>${formatInlineMarkdown(line)}</p>`);
        }
      }
    }

    if (inList) {
      output.push("</ul>");
    }
    if (inCodeBlock) {
      output.push(
        `<pre class="ql-code-block"><code>${codeBlockContent.join("\n")}</code></pre>`,
      );
    }

    return output.join("\n");
  }

  function formatInlineMarkdown(text: string): string {
    return text
      .replace(/\*\*(.*?)\*\*/g, "<strong>$1</strong>")
      .replace(/\*(.*?)\*/g, "<em>$1</em>")
      .replace(/`([^`]+)`/g, "<code>$1</code>");
  }

  onMount(() => {
    window.addEventListener("keydown", handleKeydown);

    let unlisten: (() => void) | undefined;
    void onOpenPreview((newPath) => {
      setFilePath(newPath);
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      window.removeEventListener("keydown", handleKeydown);
      unlisten?.();
    };
  });
</script>

<div class="ql-window" role="dialog" aria-label="Lux File Preview">
  <!-- Window Header / Drag Bar -->
  <header class="ql-header" data-tauri-drag-region>
    <div class="ql-meta">
      {#if previewInfo}
        {@const color = FORMAT_COLORS[previewInfo.formatKind as FileFormatKind] ?? FORMAT_COLORS.generic}
        <div class="ql-badge" style="background: {color}22; color: {color}; border-color: {color}44;">
          <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
          >
            <path d={FORMAT_ICONS[previewInfo.formatKind as FileFormatKind] ?? FORMAT_ICONS.generic} />
          </svg>
          <span>{previewInfo.formatKind.toUpperCase()}</span>
        </div>
        <div class="ql-titles">
          <h1 class="ql-filename" title={previewInfo.filename}>{previewInfo.filename}</h1>
          <p class="ql-sub">
            <span>{previewInfo.mimeOrTypeLabel}</span>
            <span class="dot">•</span>
            <span>{previewInfo.fileSizeFormatted}</span>
            {#if previewInfo.modifiedFormatted}
              <span class="dot">•</span>
              <span>{previewInfo.modifiedFormatted}</span>
            {/if}
            {#if previewInfo.pageCount}
              <span class="dot">•</span>
              <span>{previewInfo.pageCount} {previewInfo.pageCount === 1 ? 'page' : 'pages'}</span>
            {/if}
            {#if previewInfo.dimensions}
              <span class="dot">•</span>
              <span>{previewInfo.dimensions[0]} × {previewInfo.dimensions[1]} px</span>
            {/if}
          </p>
        </div>
      {:else}
        <div class="ql-titles">
          <h1 class="ql-filename">Quick Look</h1>
        </div>
      {/if}
    </div>

    <!-- Actions toolbar -->
    <div class="ql-actions">
      {#if previewInfo?.formatKind === "markdown"}
        <div class="ql-tabs">
          <button
            class="ql-tab-btn"
            class:active={activeMarkdownTab === "rendered"}
            onclick={() => (activeMarkdownTab = "rendered")}
          >
            Preview
          </button>
          <button
            class="ql-tab-btn"
            class:active={activeMarkdownTab === "raw"}
            onclick={() => (activeMarkdownTab = "raw")}
          >
            Raw
          </button>
        </div>
      {/if}

      <button
        class="ql-btn"
        onclick={handleCopyPath}
        title="Copy path (Ctrl+Shift+C)"
        aria-label="Copy file path"
      >
        <span>{copyFeedback ? "Copied" : "Copy Path"}</span>
        <kbd>Ctrl ⇧ C</kbd>
      </button>

      <button
        class="ql-btn"
        onclick={handleReveal}
        title="Reveal in folder (Ctrl+Enter)"
        aria-label="Reveal in folder"
      >
        <span>Reveal</span>
        <kbd>Ctrl ↵</kbd>
      </button>

      <button
        class="ql-btn ql-btn-primary"
        onclick={handleOpenTarget}
        title="Open file (Enter)"
        aria-label="Open file"
      >
        <span>Open</span>
        <kbd>↵</kbd>
      </button>

      <button
        class="ql-btn ql-btn-close"
        onclick={handleClose}
        title="Close preview (Escape)"
        aria-label="Close preview"
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <line x1="18" y1="6" x2="6" y2="18"></line>
          <line x1="6" y1="6" x2="18" y2="18"></line>
        </svg>
      </button>
    </div>
  </header>

  <!-- Body Content -->
  <main class="ql-body">
    {#if isLoading}
      <div class="ql-center-state">
        <div class="ql-spinner" aria-label="Loading preview"></div>
        <p>Loading preview…</p>
      </div>
    {:else if previewInfo?.error}
      <div class="ql-center-state ql-error-state">
        <div class="ql-large-icon" style="color: var(--danger, #e05555);">
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6">
            <circle cx="12" cy="12" r="10"></circle>
            <line x1="12" y1="8" x2="12" y2="12"></line>
            <line x1="12" y1="16" x2="12.01" y2="16"></line>
          </svg>
        </div>
        <h2>Unable to Preview File</h2>
        <p class="ql-err-msg">{previewInfo.error}</p>
        <p class="ql-err-path">{previewInfo.path}</p>
        <button class="ql-btn ql-btn-primary" onclick={handleOpenTarget}>
          Open with Default Application
        </button>
      </div>
    {:else if previewInfo?.formatKind === "image"}
      <div class="ql-image-view">
        <img
          class="ql-image-element"
          src={convertFileSrc(previewInfo.path)}
          alt={previewInfo.filename}
        />
      </div>
    {:else if previewInfo?.formatKind === "pdf"}
      <div class="ql-pdf-view">
        <iframe
          class="ql-pdf-frame"
          src={convertFileSrc(previewInfo.path)}
          title={previewInfo.filename}
        ></iframe>
      </div>
    {:else if previewInfo?.formatKind === "markdown"}
      <div class="ql-text-scroll">
        {#if activeMarkdownTab === "rendered" && previewInfo.textContent}
          <div class="ql-markdown-rendered">
            <!-- eslint-disable-next-line svelte/no-at-html-tags -->
            {@html renderSafeMarkdown(previewInfo.textContent)}
          </div>
        {:else if previewInfo.textContent}
          <div class="ql-code-container">
            <div class="ql-line-numbers" aria-hidden="true">
              {#each Array(previewInfo.lineCount ?? 1) as _, i}
                <span>{i + 1}</span>
              {/each}
            </div>
            <pre class="ql-code-text"><code>{previewInfo.textContent}</code></pre>
          </div>
        {/if}
      </div>
    {:else if previewInfo?.formatKind === "code" || previewInfo?.formatKind === "text"}
      <div class="ql-text-scroll">
        {#if previewInfo.textContent}
          <div class="ql-code-container">
            <div class="ql-line-numbers" aria-hidden="true">
              {#each Array(previewInfo.lineCount ?? 1) as _, i}
                <span>{i + 1}</span>
              {/each}
            </div>
            <pre class="ql-code-text"><code>{previewInfo.textContent}</code></pre>
          </div>
          {#if previewInfo.isTruncated}
            <div class="ql-truncated-banner">
              File truncated for preview (exceeds 512 KB). Open file to view full contents.
            </div>
          {/if}
        {:else}
          <div class="ql-center-state">
            <p>Empty text file</p>
          </div>
        {/if}
      </div>
    {:else if previewInfo?.formatKind === "archive"}
      <div class="ql-archive-view">
        {#if previewInfo.archiveEntries && previewInfo.archiveEntries.length > 0}
          <div class="ql-archive-header">
            <span>Archive contents ({previewInfo.archiveEntries.length} items)</span>
          </div>
          <div class="ql-archive-list" role="table">
            {#each previewInfo.archiveEntries as entry}
              <div class="ql-archive-row" role="row">
                <span class="ql-arch-icon">
                  {#if entry.isDir}
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
                      <path d={FORMAT_ICONS.folder} />
                    </svg>
                  {:else}
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
                      <path d={FORMAT_ICONS.generic} />
                    </svg>
                  {/if}
                </span>
                <span class="ql-arch-name">{entry.name}</span>
                <span class="ql-arch-size">{entry.isDir ? "Folder" : entry.sizeFormatted}</span>
              </div>
            {/each}
          </div>
        {:else}
          <div class="ql-center-state">
            <div class="ql-large-icon" style="color: var(--accent, #d19a66);">
              <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6">
                <path d={FORMAT_ICONS.archive} />
              </svg>
            </div>
            <h2>Archive Preview</h2>
            <p>{previewInfo.mimeOrTypeLabel} • {previewInfo.fileSizeFormatted}</p>
            <button class="ql-btn ql-btn-primary" onclick={handleOpenTarget}>
              Open Archive
            </button>
          </div>
        {/if}
      </div>
    {:else if previewInfo?.formatKind === "video"}
      <div class="ql-media-view">
        <!-- svelte-ignore a11y_media_has_caption -->
        <video
          class="ql-video-player"
          controls
          preload="metadata"
          src={convertFileSrc(previewInfo.path)}
        ></video>
      </div>
    {:else if previewInfo?.formatKind === "audio"}
      <div class="ql-center-state ql-audio-view">
        <div class="ql-large-icon" style="color: #c678dd;">
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6">
            <path d={FORMAT_ICONS.audio} />
          </svg>
        </div>
        <h2>{previewInfo.filename}</h2>
        <p>{previewInfo.mimeOrTypeLabel} • {previewInfo.fileSizeFormatted}</p>
        <audio
          class="ql-audio-player"
          controls
          preload="metadata"
          src={convertFileSrc(previewInfo.path)}
        ></audio>
      </div>
    {:else}
      <!-- Unsupported / generic format card -->
      <div class="ql-center-state ql-generic-view">
        <div class="ql-large-icon" style="color: #828997;">
          <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6">
            <path d={FORMAT_ICONS.generic} />
          </svg>
        </div>
        <h2>{previewInfo?.filename}</h2>
        <p class="ql-sub">{previewInfo?.mimeOrTypeLabel} • {previewInfo?.fileSizeFormatted}</p>
        <p class="ql-path-display">{previewInfo?.path}</p>
        <div class="ql-generic-actions">
          <button class="ql-btn ql-btn-primary" onclick={handleOpenTarget}>
            Open with Default Application
          </button>
          <button class="ql-btn" onclick={handleReveal}>
            Reveal in Folder
          </button>
        </div>
      </div>
    {/if}
  </main>
</div>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    overflow: hidden;
    user-select: none;
    -webkit-user-select: none;
    background: transparent !important;
  }

  .ql-window {
    display: flex;
    flex-direction: column;
    width: 100vw;
    height: 100vh;
    box-sizing: border-box;
    background: rgba(26, 27, 30, 0.94);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 14px;
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.5), 0 0 0 1px rgba(255, 255, 255, 0.05);
    color: #e1e4e8;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    overflow: hidden;
  }

  .ql-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 52px;
    min-height: 52px;
    padding: 0 14px;
    background: rgba(255, 255, 255, 0.03);
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    cursor: default;
  }

  .ql-meta {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    flex: 1;
  }

  .ql-badge {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 3px 7px;
    border-radius: 6px;
    border: 1px solid transparent;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.5px;
    flex-shrink: 0;
  }

  .ql-titles {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .ql-filename {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    color: #ffffff;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ql-sub {
    margin: 0;
    font-size: 11px;
    color: rgba(255, 255, 255, 0.5);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    display: flex;
    align-items: center;
    gap: 5px;
  }

  .ql-actions {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .ql-tabs {
    display: flex;
    background: rgba(0, 0, 0, 0.25);
    padding: 2px;
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.05);
    margin-right: 6px;
  }

  .ql-tab-btn {
    background: transparent;
    border: none;
    color: rgba(255, 255, 255, 0.5);
    font-size: 11px;
    font-weight: 500;
    padding: 3px 8px;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .ql-tab-btn.active {
    background: rgba(255, 255, 255, 0.12);
    color: #ffffff;
  }

  .ql-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 5px 10px;
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    color: #d1d5db;
    font-size: 12px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .ql-btn:hover {
    background: rgba(255, 255, 255, 0.1);
    border-color: rgba(255, 255, 255, 0.15);
    color: #ffffff;
  }

  .ql-btn-primary {
    background: rgba(74, 143, 216, 0.2);
    border-color: rgba(74, 143, 216, 0.4);
    color: #61afef;
  }

  .ql-btn-primary:hover {
    background: rgba(74, 143, 216, 0.3);
    border-color: rgba(74, 143, 216, 0.6);
    color: #ffffff;
  }

  .ql-btn kbd {
    background: rgba(0, 0, 0, 0.35);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 3px;
    padding: 1px 4px;
    font-size: 9px;
    font-family: inherit;
    color: rgba(255, 255, 255, 0.6);
  }

  .ql-btn-close {
    padding: 5px 7px;
  }

  .ql-body {
    flex: 1;
    overflow: hidden;
    position: relative;
    display: flex;
    flex-direction: column;
  }

  .ql-center-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    flex: 1;
    padding: 24px;
    text-align: center;
    color: rgba(255, 255, 255, 0.7);
  }

  .ql-large-icon {
    margin-bottom: 12px;
  }

  .ql-error-state h2 {
    margin: 0 0 6px 0;
    font-size: 16px;
    color: #ffffff;
  }

  .ql-err-msg {
    color: #e05555;
    font-size: 12px;
    margin: 0 0 4px 0;
  }

  .ql-err-path {
    color: rgba(255, 255, 255, 0.4);
    font-size: 11px;
    font-family: monospace;
    margin: 0 0 16px 0;
    max-width: 80%;
    word-break: break-all;
  }

  .ql-spinner {
    width: 28px;
    height: 28px;
    border: 2px solid rgba(255, 255, 255, 0.1);
    border-top-color: #4a8fd8;
    border-radius: 50%;
    animation: ql-spin 0.8s linear infinite;
    margin-bottom: 12px;
  }

  @keyframes ql-spin {
    to {
      transform: rotate(360deg);
    }
  }

  /* Image View */
  .ql-image-view {
    display: flex;
    align-items: center;
    justify-content: center;
    flex: 1;
    padding: 16px;
    overflow: auto;
  }

  .ql-image-element {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    border-radius: 6px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3);
  }

  /* PDF View */
  .ql-pdf-view {
    flex: 1;
    width: 100%;
    height: 100%;
    overflow: hidden;
  }

  .ql-pdf-frame {
    width: 100%;
    height: 100%;
    border: none;
    background: #2a2a2a;
  }

  /* Text & Code View */
  .ql-text-scroll {
    flex: 1;
    overflow: auto;
    padding: 12px;
  }

  .ql-code-container {
    display: flex;
    font-family: "Cascadia Code", "Fira Code", Consolas, "Courier New", monospace;
    font-size: 12px;
    line-height: 1.6;
    user-select: text;
    -webkit-user-select: text;
  }

  .ql-line-numbers {
    display: flex;
    flex-direction: column;
    padding-right: 14px;
    color: rgba(255, 255, 255, 0.25);
    text-align: right;
    user-select: none;
    -webkit-user-select: none;
    border-right: 1px solid rgba(255, 255, 255, 0.06);
    margin-right: 14px;
    flex-shrink: 0;
  }

  .ql-code-text {
    margin: 0;
    white-space: pre;
    color: #abb2bf;
    overflow-x: auto;
    flex: 1;
  }

  .ql-truncated-banner {
    background: rgba(229, 192, 123, 0.15);
    border-top: 1px solid rgba(229, 192, 123, 0.3);
    color: #e5c07b;
    font-size: 11px;
    padding: 6px 14px;
    text-align: center;
    position: sticky;
    bottom: 0;
  }

  /* Markdown rendered */
  .ql-markdown-rendered {
    padding: 12px 24px;
    color: #d1d5db;
    font-size: 13px;
    line-height: 1.6;
    user-select: text;
    -webkit-user-select: text;
  }

  :global(.ql-markdown-rendered h1) {
    font-size: 20px;
    margin: 0 0 12px 0;
    color: #ffffff;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    padding-bottom: 6px;
  }

  :global(.ql-markdown-rendered h2) {
    font-size: 16px;
    margin: 16px 0 8px 0;
    color: #ffffff;
  }

  :global(.ql-markdown-rendered h3) {
    font-size: 14px;
    margin: 14px 0 6px 0;
    color: #ffffff;
  }

  :global(.ql-markdown-rendered p) {
    margin: 0 0 10px 0;
  }

  :global(.ql-markdown-rendered blockquote) {
    margin: 10px 0;
    padding: 4px 12px;
    border-left: 3px solid #4a8fd8;
    background: rgba(74, 143, 216, 0.08);
    color: rgba(255, 255, 255, 0.7);
  }

  :global(.ql-markdown-rendered ul) {
    margin: 0 0 10px 0;
    padding-left: 20px;
  }

  :global(.ql-markdown-rendered code) {
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 4px;
    padding: 1px 5px;
    font-family: monospace;
    font-size: 11px;
    color: #e5c07b;
  }

  :global(.ql-markdown-rendered .ql-code-block) {
    background: rgba(0, 0, 0, 0.35);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    padding: 10px;
    overflow-x: auto;
  }

  /* Archive View */
  .ql-archive-view {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow: hidden;
  }

  .ql-archive-header {
    padding: 8px 14px;
    font-size: 11px;
    font-weight: 600;
    color: rgba(255, 255, 255, 0.5);
    border-bottom: 1px solid rgba(255, 255, 255, 0.06);
    background: rgba(0, 0, 0, 0.15);
  }

  .ql-archive-list {
    flex: 1;
    overflow-y: auto;
  }

  .ql-archive-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 14px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.03);
    font-size: 12px;
  }

  .ql-archive-row:hover {
    background: rgba(255, 255, 255, 0.03);
  }

  .ql-arch-icon {
    color: rgba(255, 255, 255, 0.4);
    display: flex;
    align-items: center;
  }

  .ql-arch-name {
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    color: #e1e4e8;
  }

  .ql-arch-size {
    font-size: 11px;
    color: rgba(255, 255, 255, 0.4);
    flex-shrink: 0;
  }

  /* Media Players */
  .ql-media-view {
    display: flex;
    align-items: center;
    justify-content: center;
    flex: 1;
    padding: 16px;
    background: #000000;
  }

  .ql-video-player {
    max-width: 100%;
    max-height: 100%;
    border-radius: 6px;
  }

  .ql-audio-player {
    width: 320px;
    margin-top: 14px;
  }

  /* Generic fallback card */
  .ql-path-display {
    font-family: monospace;
    font-size: 11px;
    color: rgba(255, 255, 255, 0.4);
    margin: 8px 0 18px 0;
    max-width: 80%;
    word-break: break-all;
  }

  .ql-generic-actions {
    display: flex;
    gap: 8px;
  }
</style>
