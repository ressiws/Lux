import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { LauncherStatus, SearchResponse } from "./types";

const EVENT_LAUNCHER_SHOWN = "lux://launcher-shown";
const EVENT_LAUNCHER_HIDDEN = "lux://launcher-hidden";
const EVENT_INDEXING_STATUS = "lux://indexing-status";

export async function hideLauncher(): Promise<void> {
  await invoke("hide_launcher");
}

export async function getLauncherStatus(): Promise<LauncherStatus> {
  return invoke<LauncherStatus>("get_launcher_status");
}

export async function searchLauncher(query: string): Promise<SearchResponse> {
  return invoke<SearchResponse>("search_launcher", { query });
}

export async function browseLauncher(category: string): Promise<SearchResponse> {
  return invoke<SearchResponse>("browse_launcher", { category });
}

export async function openTarget(path: string): Promise<void> {
  return invoke<void>("open_target", { path });
}

export async function openTargetAsAdmin(path: string): Promise<void> {
  return invoke<void>("open_target_as_admin", { path });
}

export async function revealItemInFolder(path: string): Promise<void> {
  return invoke<void>("reveal_item_in_folder", { path });
}

export async function getImageThumbnail(
  path: string,
): Promise<string | undefined> {
  try {
    const res = await invoke<string | null>("get_image_thumbnail", { path });
    return res ?? undefined;
  } catch {
    return undefined;
  }
}

export async function setLauncherHeight(
  expanded: boolean,
  idleHeight?: number,
): Promise<void> {
  return invoke<void>("set_launcher_height", { expanded, idleHeight });
}

export async function setLauncherIdleHeight(height: number): Promise<void> {
  return invoke<void>("set_launcher_idle_height", { height });
}

export async function setNativeGlass(enable: boolean): Promise<void> {
  return invoke<void>("set_native_glass", { enable });
}

export async function reindex(): Promise<void> {
  return invoke<void>("reindex");
}

export async function getIndexStats(): Promise<import("./types").IndexStats> {
  return invoke<import("./types").IndexStats>("get_index_stats");
}

export async function onIndexingStatus(
  handler: (payload: import("./types").IndexingStatusPayload) => void,
): Promise<UnlistenFn> {
  return listen<import("./types").IndexingStatusPayload>(
    EVENT_INDEXING_STATUS,
    (event) => {
      handler(event.payload);
    },
  );
}

export async function onLauncherShown(
  handler: () => void,
): Promise<UnlistenFn> {
  return listen(EVENT_LAUNCHER_SHOWN, () => {
    handler();
  });
}

export async function onLauncherHidden(
  handler: () => void,
): Promise<UnlistenFn> {
  return listen(EVENT_LAUNCHER_HIDDEN, () => {
    handler();
  });
}

export async function setWindowHeight(height: number): Promise<void> {
  return invoke<void>("set_window_height", { height });
}

export async function getClipboardHistory(): Promise<import("./types").ClipboardEntry[]> {
  return invoke<import("./types").ClipboardEntry[]>("get_clipboard_history");
}

export async function getFilteredClipboardHistory(): Promise<import("./types").ClipboardEntry[]> {
  return invoke<import("./types").ClipboardEntry[]>("get_filtered_clipboard_history");
}

export async function getClipboardFiltersConfig(): Promise<import("./types").ClipboardFiltersConfig> {
  return invoke<import("./types").ClipboardFiltersConfig>("get_clipboard_filters_config");
}

export async function saveClipboardFiltersConfig(
  config: import("./types").ClipboardFiltersConfig,
): Promise<void> {
  return invoke<void>("save_clipboard_filters_config", { config });
}

export async function validateClipboardFilter(
  pattern: string,
  isRegex: boolean,
  testText?: string,
): Promise<import("./types").FilterValidationResult> {
  return invoke<import("./types").FilterValidationResult>("validate_clipboard_filter", {
    pattern,
    isRegex,
    testText,
  });
}

export async function copyClipboardEntry(id: string): Promise<void> {
  return invoke<void>("copy_clipboard_entry", { id });
}

export async function deleteClipboardEntry(id: string): Promise<void> {
  return invoke<void>("delete_clipboard_entry", { id });
}

export async function clearClipboardHistory(): Promise<void> {
  return invoke<void>("clear_clipboard_history");
}

export async function clearClipboardPins(): Promise<number> {
  return invoke<number>("clear_clipboard_pins");
}

export async function togglePinClipboardEntry(id: string): Promise<boolean> {
  return invoke<boolean>("toggle_pin_clipboard_entry", { id });
}

export async function onClipboardUpdated(handler: () => void): Promise<UnlistenFn> {
  return listen("lux://clipboard-updated", () => {
    handler();
  });
}

export async function onOpenLauncher(handler: () => void): Promise<UnlistenFn> {
  return listen("lux://open-launcher", () => {
    handler();
  });
}

export async function onOpenClipboard(handler: () => void): Promise<UnlistenFn> {
  return listen("lux://open-clipboard", () => {
    handler();
  });
}

export async function onOpenSettings(handler: () => void): Promise<UnlistenFn> {
  return listen("lux://open-settings", () => {
    handler();
  });
}

export async function openSettings(): Promise<void> {
  return invoke<void>("open_settings");
}

export async function hideSettings(): Promise<void> {
  return invoke<void>("hide_settings");
}

export async function getShortcutsStatus(): Promise<import("./types").ShortcutConfigStatus> {
  return invoke<import("./types").ShortcutConfigStatus>("get_shortcuts_status");
}

export async function updateShortcuts(
  launcher: string,
  clipboard: string,
): Promise<import("./types").ShortcutConfigStatus> {
  return invoke<import("./types").ShortcutConfigStatus>("update_shortcuts", {
    launcher,
    clipboard,
  });
}

export async function notifyFrontendReady(idleHeight?: number): Promise<void> {
  return invoke<void>("notify_frontend_ready", { idleHeight });
}

export async function getFeaturesConfig(): Promise<import("./types").FeaturesConfig> {
  return invoke<import("./types").FeaturesConfig>("get_features_config");
}

export async function setFeatureEnabled(
  feature: string,
  enabled: boolean,
): Promise<import("./types").FeaturesConfig> {
  return invoke<import("./types").FeaturesConfig>("set_feature_enabled", {
    feature,
    enabled,
  });
}

export async function onFeaturesChanged(
  handler: (features: import("./types").FeaturesConfig) => void,
): Promise<UnlistenFn> {
  return listen<import("./types").FeaturesConfig>("lux://features-changed", (event) => {
    handler(event.payload);
  });
}

export async function getDefaultBrowser(): Promise<import("./types").DefaultBrowserInfo> {
  return invoke<import("./types").DefaultBrowserInfo>("get_default_browser");
}

export const EVENT_OPEN_PREVIEW = "lux://open-preview";
export const EVENT_CLOSE_PREVIEW = "lux://close-preview";

export async function showPreview(path: string): Promise<void> {
  return invoke("show_preview", { path });
}

export async function hidePreview(): Promise<void> {
  return invoke("hide_preview");
}

export async function togglePreview(path: string): Promise<void> {
  return invoke("toggle_preview", { path });
}

export async function getFilePreview(path: string): Promise<import("./types").FilePreviewInfo> {
  return invoke<import("./types").FilePreviewInfo>("get_file_preview", { path });
}

export async function onOpenPreview(handler: (path: string) => void): Promise<UnlistenFn> {
  return listen<string>(EVENT_OPEN_PREVIEW, (event) => {
    handler(event.payload);
  });
}

export async function onClosePreview(handler: () => void): Promise<UnlistenFn> {
  return listen(EVENT_CLOSE_PREVIEW, () => {
    handler();
  });
}

export async function isPreviewVisible(): Promise<boolean> {
  return invoke<boolean>("is_preview_visible");
}

export const EVENT_SHOW_WELCOME = "lux://show-welcome";

export async function isFirstRun(): Promise<boolean> {
  try {
    return await invoke<boolean>("is_first_run");
  } catch {
    return false;
  }
}

export async function completeFirstRun(): Promise<void> {
  try {
    await invoke<void>("complete_first_run");
  } catch {}
}

export async function onShowWelcome(handler: () => void): Promise<UnlistenFn> {
  return listen(EVENT_SHOW_WELCOME, () => {
    handler();
  });
}
