export type ItemCategory =
  | "calc"
  | "app"
  | "file"
  | "cmd"
  | "clip"
  | "snip"
  | "fb";

export interface ItemAction {
  readonly label: string;
  readonly shortcut: string;
  readonly danger?: boolean;
}

export interface CalcPreview {
  readonly value: string;
  readonly expression: string;
  readonly label: string;
  readonly raw: string;
}

export interface LauncherItem {
  readonly category: ItemCategory;
  readonly name: string;
  readonly subtitle: string;
  readonly keywords: string;
  readonly rightLabel: string;
  readonly primaryLabel: string;
  readonly actions: readonly ItemAction[];
  readonly color?: string;
  readonly thumbnail?: string;
  readonly calc?: CalcPreview;
  readonly path?: string;
  readonly isImage?: boolean;
  readonly icon?: string;
}

export interface SearchResultItem {
  readonly id: string;
  readonly category: "app" | "file";
  readonly name: string;
  readonly subtitle: string;
  readonly path: string;
  readonly icon: string | null;
  readonly color: string | null;
  readonly isImage: boolean;
  readonly rightLabel: string;
}

export interface SearchResponse {
  readonly apps: readonly SearchResultItem[];
  readonly files: readonly SearchResultItem[];
}

export interface LauncherSection {
  readonly category: ItemCategory;
  readonly title: string;
  readonly items: readonly LauncherItem[];
}

export interface HighlightParts {
  readonly before: string;
  readonly match: string;
  readonly after: string;
}

export interface ShortcutStatus {
  readonly shortcut: string;
  readonly registered: boolean;
  readonly error: string | null;
}

export interface LauncherStatus {
  readonly shortcut: ShortcutStatus;
  readonly nativeGlassSupported: boolean;
}
