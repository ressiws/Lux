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

export async function openTarget(path: string): Promise<void> {
  return invoke<void>("open_target", { path });
}

export async function openTargetAsAdmin(path: string): Promise<void> {
  return invoke<void>("open_target_as_admin", { path });
}

export async function revealItemInFolder(path: string): Promise<void> {
  return invoke<void>("reveal_item_in_folder", { path });
}

export async function setLauncherHeight(expanded: boolean): Promise<void> {
  return invoke<void>("set_launcher_height", { expanded });
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
