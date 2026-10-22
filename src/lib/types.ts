export type ItemCategory =
  | "calc"
  | "app"
  | "file"
  | "cmd"
  | "clip"
  | "snip"
  | "fb";

export type FileFormatKind =
  | "pdf"
  | "markdown"
  | "text"
  | "image"
  | "video"
  | "audio"
  | "code"
  | "archive"
  | "document"
  | "folder"
  | "generic";

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
  readonly categoryLabel?: string;
  readonly colorPreview?: string;
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
  readonly location?: string;
  readonly isImage?: boolean;
  readonly icon?: string;
  readonly size?: string;
  readonly version?: string;
}

export interface SearchResultItem {
  readonly id: string;
  readonly category: "app" | "file";
  readonly name: string;
  readonly subtitle: string;
  readonly path: string;
  readonly location: string;
  readonly icon: string | null;
  readonly thumbnail: string | null;
  readonly color: string | null;
  readonly isImage: boolean;
  readonly rightLabel: string;
  readonly size: string | null;
  readonly version: string | null;
}

export interface SearchResponse {
  readonly apps: readonly SearchResultItem[];
  readonly files: readonly SearchResultItem[];
  readonly backend?: "everything" | "native" | string;
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

export interface IndexingStatusPayload {
  readonly isIndexing: boolean;
  readonly totalApps: number;
  readonly totalFiles: number;
}

export interface IndexStats {
  readonly totalApps: number;
  readonly totalFiles: number;
  readonly isIndexed: boolean;
  readonly isIndexing?: boolean;
  readonly backend?: "everything" | "native" | string;
}

export type ClipboardKind = "text" | "link" | "image" | "file" | "color";

export interface ClipboardEntry {
  readonly id: string;
  readonly kind: ClipboardKind;
  readonly name: string;
  readonly preview: string | null;
  readonly sourceApp: string | null;
  readonly createdAt: number;
  readonly isPinned: boolean;
  readonly charCount: number | null;
  readonly wordCount: number | null;
  readonly linkUrl: string | null;
  readonly linkTitle: string | null;
  readonly linkDesc: string | null;
  readonly filePath: string | null;
  readonly fileSize: string | null;
  readonly fileType: string | null;
  readonly imageDims: string | null;
  readonly imageSize: string | null;
  readonly colorHex: string | null;
  readonly colorRgb: string | null;
  readonly colorHsl: string | null;
  readonly isFiltered?: boolean;
  readonly matchedFilter?: string | null;
}

export type ExclusionMode = "ignore" | "hide";

export interface CustomFilter {
  id: string;
  name: string;
  pattern: string;
  isRegex: boolean;
  enabled: boolean;
}

export interface ClipboardFiltersConfig {
  mode: ExclusionMode;
  filterSecrets: boolean;
  filterCamelCase: boolean;
  filterCreditCards: boolean;
  customFilters: CustomFilter[];
}

export interface FilterValidationResult {
  valid: boolean;
  error: string | null;
  matchesTest: boolean | null;
}

export interface ShortcutConfigStatus {
  readonly launcher: ShortcutStatus;
  readonly clipboard: ShortcutStatus;
}

export interface FeaturesConfig {
  readonly clipboardHistory: boolean;
}

export interface DefaultBrowserInfo {
  readonly name: string;
  readonly icon: string | null;
}
