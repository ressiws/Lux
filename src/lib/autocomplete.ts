import type { LauncherItem } from "./types";

export interface InlineCompletion {
  readonly item: LauncherItem;
  readonly completion: string;
}

const COMPLETABLE_CATEGORIES = new Set(["app", "file", "cmd"]);

export function findInlineCompletion(
  query: string,
  items: readonly LauncherItem[],
): InlineCompletion | undefined {
  const trimmed = query.trim();
  if (!trimmed || query !== trimmed) {
    return undefined;
  }

  const lowered = trimmed.toLowerCase();
  for (const item of items) {
    if (!COMPLETABLE_CATEGORIES.has(item.category)) {
      continue;
    }
    if (item.name.length <= lowered.length) {
      continue;
    }
    if (item.name.toLowerCase().startsWith(lowered)) {
      return { item, completion: item.name.slice(trimmed.length) };
    }
  }

  return undefined;
}
