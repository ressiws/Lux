import { classifyFile } from "./classification";
import { mapBackendApp, mapBackendFile } from "./mock-data";
import type { LauncherItem } from "./types";

export const ITEM_USAGE_STORAGE_KEY = "lux-item-usage";
const MAX_USAGE_ENTRIES = 96;

export type FrequentMode = "apps" | "folders" | "all";

export interface ItemUsage {
  readonly key: string;
  readonly category: "app" | "file";
  readonly name: string;
  readonly subtitle: string;
  readonly path: string;
  readonly location: string;
  readonly rightLabel: string;
  readonly isImage: boolean;
  readonly icon: string | null;
  readonly size: string | null;
  readonly version: string | null;
  readonly count: number;
  readonly lastUsed: number;
}

function optionalString(value: unknown): string | null {
  return typeof value === "string" && value.length > 0 ? value : null;
}

function normalizeUsage(value: unknown): ItemUsage | undefined {
  if (!value || typeof value !== "object") {
    return undefined;
  }

  const entry = value as Record<string, unknown>;
  if (
    typeof entry.key !== "string" ||
    (entry.category !== "app" && entry.category !== "file") ||
    typeof entry.name !== "string" ||
    typeof entry.path !== "string" ||
    typeof entry.count !== "number" ||
    !Number.isFinite(entry.count) ||
    entry.count <= 0 ||
    typeof entry.lastUsed !== "number" ||
    !Number.isFinite(entry.lastUsed)
  ) {
    return undefined;
  }

  return {
    key: entry.key,
    category: entry.category,
    name: entry.name,
    subtitle: typeof entry.subtitle === "string" ? entry.subtitle : "",
    path: entry.path,
    location: typeof entry.location === "string" ? entry.location : "",
    rightLabel:
      typeof entry.rightLabel === "string" && entry.rightLabel.length > 0
        ? entry.rightLabel
        : entry.category === "app"
          ? "Application"
          : "File",
    isImage: entry.isImage === true,
    icon: optionalString(entry.icon),
    size: optionalString(entry.size),
    version: optionalString(entry.version),
    count: entry.count,
    lastUsed: entry.lastUsed,
  };
}

export function loadUsageEntries(): ItemUsage[] {
  if (typeof localStorage === "undefined") {
    return [];
  }

  try {
    const raw = localStorage.getItem(ITEM_USAGE_STORAGE_KEY);
    if (!raw) {
      return [];
    }
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) {
      return [];
    }
    return parsed
      .map(normalizeUsage)
      .filter((entry): entry is ItemUsage => entry !== undefined);
  } catch {
    return [];
  }
}

function saveUsageEntries(entries: readonly ItemUsage[]): void {
  if (typeof localStorage === "undefined") {
    return;
  }

  try {
    localStorage.setItem(ITEM_USAGE_STORAGE_KEY, JSON.stringify(entries));
  } catch {
    // Ignore storage quota or disabled storage errors.
  }
}

function compareUsage(a: ItemUsage, b: ItemUsage): number {
  return b.count - a.count || b.lastUsed - a.lastUsed;
}

export function recordItemUsage(
  item: LauncherItem,
  now = Date.now(),
): ItemUsage[] {
  if (
    (item.category !== "app" && item.category !== "file") ||
    !item.path
  ) {
    return loadUsageEntries();
  }

  const key = `${item.category}:${item.path}`;
  const entries = loadUsageEntries();
  const existing = entries.find((entry) => entry.key === key);
  const count = (existing?.count ?? 0) + 1;

  const nextEntry: ItemUsage = {
    key,
    category: item.category,
    name: item.name,
    subtitle: item.subtitle,
    path: item.path,
    location: item.location ?? "",
    rightLabel: item.rightLabel,
    isImage: item.isImage === true,
    icon: item.icon ?? null,
    size: item.size ?? null,
    version: item.version ?? null,
    count,
    lastUsed: now,
  };

  const next = [
    ...entries.filter((entry) => entry.key !== key),
    nextEntry,
  ]
    .sort(compareUsage)
    .slice(0, MAX_USAGE_ENTRIES);

  saveUsageEntries(next);
  return next;
}

function toLauncherItem(entry: ItemUsage): LauncherItem {
  const base = {
    id: entry.key,
    category: entry.category,
    name: entry.name,
    subtitle: entry.subtitle,
    path: entry.path,
    location: entry.location,
    icon: entry.icon,
    thumbnail: null,
    color: null,
    isImage: entry.isImage,
    rightLabel: entry.rightLabel,
    size: entry.size,
    version: entry.version,
  };

  return entry.category === "app"
    ? mapBackendApp(base)
    : mapBackendFile(base);
}

export function getFrequentItems(
  entries: readonly ItemUsage[],
  mode: FrequentMode,
  limit = 4,
): LauncherItem[] {
  const sorted = [...entries].sort(compareUsage);
  const selected: ItemUsage[] = [];

  for (const entry of sorted) {
    if (selected.length >= limit) {
      break;
    }
    if (mode === "apps" && entry.category !== "app") {
      continue;
    }
    if (
      mode === "folders" &&
      (entry.category !== "file" ||
        classifyFile(toLauncherItem(entry)) !== "folders")
    ) {
      continue;
    }
    selected.push(entry);
  }

  return selected.map(toLauncherItem);
}
