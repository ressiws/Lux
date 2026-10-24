import { mapBackendApp, mapBackendFile } from "./mock-data";
import type { LauncherItem } from "./types";

export const FAVORITES_STORAGE_KEY = "lux-favorites";

export interface FavoriteEntry {
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
}

function optionalString(value: unknown): string | null {
  return typeof value === "string" && value.length > 0 ? value : null;
}

export function getFavoriteKey(
  item: Pick<LauncherItem, "category" | "path">,
): string | null {
  if (
    (item.category !== "app" && item.category !== "file") ||
    !item.path
  ) {
    return null;
  }
  return `${item.category}:${item.path}`;
}

function normalizeFavorite(value: unknown): FavoriteEntry | undefined {
  if (!value || typeof value !== "object") {
    return undefined;
  }

  const entry = value as Record<string, unknown>;
  if (
    (entry.category !== "app" && entry.category !== "file") ||
    typeof entry.name !== "string" ||
    entry.name.length === 0 ||
    typeof entry.path !== "string" ||
    entry.path.length === 0
  ) {
    return undefined;
  }

  return {
    key: `${entry.category}:${entry.path}`,
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
  };
}

export function loadFavoriteEntries(): FavoriteEntry[] {
  if (typeof localStorage === "undefined") {
    return [];
  }

  try {
    const raw = localStorage.getItem(FAVORITES_STORAGE_KEY);
    if (!raw) {
      return [];
    }
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) {
      return [];
    }

    const seen = new Set<string>();
    const entries: FavoriteEntry[] = [];
    for (const value of parsed) {
      const entry = normalizeFavorite(value);
      if (!entry || seen.has(entry.key)) {
        continue;
      }
      seen.add(entry.key);
      entries.push(entry);
    }
    return entries;
  } catch {
    return [];
  }
}

function saveFavoriteEntries(entries: readonly FavoriteEntry[]): void {
  if (typeof localStorage === "undefined") {
    return;
  }

  try {
    localStorage.setItem(FAVORITES_STORAGE_KEY, JSON.stringify(entries));
  } catch {
    // Ignore storage quota or disabled storage errors.
  }
}

export function toggleFavoriteItem(item: LauncherItem): FavoriteEntry[] {
  if (
    (item.category !== "app" && item.category !== "file") ||
    !item.path
  ) {
    return loadFavoriteEntries();
  }

  const key = `${item.category}:${item.path}`;
  const entries = loadFavoriteEntries();
  const exists = entries.some((entry) => entry.key === key);
  if (exists) {
    const next = entries.filter((entry) => entry.key !== key);
    saveFavoriteEntries(next);
    return next;
  }

  const nextEntry: FavoriteEntry = {
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
  };
  const next = [nextEntry, ...entries];
  saveFavoriteEntries(next);
  return next;
}

function toLauncherItem(entry: FavoriteEntry): LauncherItem {
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

export function getFavoriteItems(
  entries: readonly FavoriteEntry[],
): LauncherItem[] {
  return entries.map(toLauncherItem);
}
