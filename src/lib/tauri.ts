import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { LauncherStatus, SearchResponse } from "./types";

const EVENT_LAUNCHER_SHOWN = "lux://launcher-shown";
const EVENT_LAUNCHER_HIDDEN = "lux://launcher-hidden";

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
