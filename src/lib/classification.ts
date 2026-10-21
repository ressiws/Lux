import type { LauncherItem, SearchResultItem } from "./types";
import type { SearchFilter } from "./mock-data";

export type AppSubcategory =
  | "all"
  | "utilities"
  | "productivity"
  | "finance"
  | "social"
  | "entertainment"
  | "games"
  | "devtools"
  | "other";

export const APP_SUBCATEGORY_LABELS: Record<AppSubcategory, string> = {
  all: "All",
  utilities: "Utilities",
  productivity: "Productivity",
  finance: "Finance",
  social: "Social",
  entertainment: "Entertainment",
  games: "Games",
  devtools: "Developer Tools",
  other: "Other",
};

export const APP_SUBCATEGORY_ORDER: readonly AppSubcategory[] = [
  "all",
  "utilities",
  "productivity",
  "finance",
  "social",
  "entertainment",
  "games",
  "devtools",
  "other",
];

export type FileSubcategory =
  | "all"
  | "folders"
  | "documents"
  | "images"
  | "videos"
  | "audio"
  | "archives"
  | "other";

export const FILE_SUBCATEGORY_LABELS: Record<FileSubcategory, string> = {
  all: "All",
  folders: "Folders",
  documents: "Documents",
  images: "Images",
  videos: "Videos",
  audio: "Audio",
  archives: "Archives",
  other: "Other",
};

export const FILE_SUBCATEGORY_ORDER: readonly FileSubcategory[] = [
  "all",
  "folders",
  "documents",
  "images",
  "videos",
  "audio",
  "archives",
  "other",
];

export type ActionSubcategory = "all" | "preferences" | "tools";

export const ACTION_SUBCATEGORY_LABELS: Record<ActionSubcategory, string> = {
  all: "All",
  preferences: "Preferences",
  tools: "Tools",
};

export const ACTION_SUBCATEGORY_ORDER: readonly ActionSubcategory[] = [
  "all",
  "preferences",
  "tools",
];

export interface SubcategoryItem {
  readonly id: string;
  readonly label: string;
}

// In-memory classification caches to avoid recomputing on every render/keystroke
const appCache = new Map<string, AppSubcategory>();
const fileCache = new Map<string, FileSubcategory>();

const IMAGE_EXTS = new Set([
  "png", "jpg", "jpeg", "gif", "bmp", "webp", "svg", "ico", "tiff", "tif", "avif", "heic",
]);

const DOCUMENT_EXTS = new Set([
  "pdf", "doc", "docx", "txt", "rtf", "odt", "xls", "xlsx", "csv", "ppt", "pptx",
  "md", "markdown", "json", "xml", "yaml", "yml", "epub", "log",
]);

const VIDEO_EXTS = new Set([
  "mp4", "mkv", "avi", "mov", "wmv", "flv", "webm", "m4v", "3gp",
]);

const AUDIO_EXTS = new Set([
  "mp3", "wav", "flac", "ogg", "m4a", "wma", "aac", "opus", "alac",
]);

const ARCHIVE_EXTS = new Set([
  "zip", "rar", "7z", "tar", "gz", "bz2", "xz", "iso",
]);

/**
 * Deterministically classifies an application using local metadata
 * (path folders, executable name, keywords, comments).
 */
export function classifyApp(item: LauncherItem | SearchResultItem): AppSubcategory {
  const cacheKey = item.path || item.name;
  const cached = appCache.get(cacheKey);
  if (cached) {
    return cached;
  }

  const name = item.name.toLowerCase();
  const path = (item.path || "").toLowerCase();
  const location = (item.location || "").toLowerCase();
  const keywords = ("keywords" in item ? item.keywords : "").toLowerCase();
  const subtitle = (item.subtitle || "").toLowerCase();

  const text = `${name} ${path} ${location} ${keywords} ${subtitle}`;

  let result: AppSubcategory = "other";

  // 1. Developer Tools
  if (
    text.includes("visual studio") ||
    text.includes("vscode") ||
    text.includes("vscodium") ||
    text.includes("git") ||
    text.includes("terminal") ||
    text.includes("powershell") ||
    text.includes("sublime") ||
    text.includes("intellij") ||
    text.includes("pycharm") ||
    text.includes("webstorm") ||
    text.includes("clion") ||
    text.includes("rider") ||
    text.includes("android studio") ||
    text.includes("xcode") ||
    text.includes("eclipse") ||
    text.includes("neovim") ||
    text.includes("vim") ||
    text.includes("emacs") ||
    text.includes("postman") ||
    text.includes("insomnia") ||
    text.includes("docker") ||
    text.includes("dbeaver") ||
    text.includes("wireshark") ||
    text.includes("unity") ||
    text.includes("godot") ||
    text.includes("unreal engine") ||
    text.includes("development") ||
    text.includes("developer tools") ||
    text.includes("code editor")
  ) {
    result = "devtools";
  }
  // 2. Games
  else if (
    text.includes("steam") ||
    text.includes("epic games") ||
    text.includes("battle.net") ||
    text.includes("gog galaxy") ||
    text.includes("riot client") ||
    text.includes("minecraft") ||
    text.includes("xbox") ||
    text.includes("ea app") ||
    text.includes("origin") ||
    text.includes("ubisoft") ||
    text.includes("roblox") ||
    text.includes("solitaire") ||
    path.includes("games") ||
    location.includes("games")
  ) {
    result = "games";
  }
  // 3. Social & Communication
  else if (
    text.includes("discord") ||
    text.includes("slack") ||
    text.includes("telegram") ||
    text.includes("whatsapp") ||
    text.includes("signal") ||
    text.includes("teams") ||
    text.includes("zoom") ||
    text.includes("skype") ||
    text.includes("messenger") ||
    text.includes("wechat") ||
    text.includes("thunderbird") ||
    text.includes("mail") ||
    path.includes("social") ||
    path.includes("communication")
  ) {
    result = "social";
  }
  // 4. Finance & Accounting
  else if (
    text.includes("quickbooks") ||
    text.includes("quicken") ||
    text.includes("turbotax") ||
    text.includes("gnucash") ||
    text.includes("ledger") ||
    text.includes("crypto") ||
    text.includes("wallet") ||
    text.includes("banking") ||
    text.includes("finance")
  ) {
    result = "finance";
  }
  // 5. Entertainment & Media
  else if (
    text.includes("spotify") ||
    text.includes("apple music") ||
    text.includes("itunes") ||
    text.includes("tidal") ||
    text.includes("vlc") ||
    text.includes("media player") ||
    text.includes("netflix") ||
    text.includes("youtube") ||
    text.includes("plex") ||
    text.includes("kodi") ||
    text.includes("audacity") ||
    text.includes("gimp") ||
    text.includes("darktable") ||
    text.includes("lightcraft") ||
    text.includes("lightroom") ||
    text.includes("photoshop") ||
    text.includes("illustrator") ||
    text.includes("premiere") ||
    text.includes("blender") ||
    text.includes("obs studio") ||
    text.includes("handbrake") ||
    text.includes("music") ||
    text.includes("photo workflow") ||
    text.includes("photo editor") ||
    text.includes("image editor")
  ) {
    result = "entertainment";
  }
  // 6. Productivity & Office
  else if (
    text.includes("word") ||
    text.includes("excel") ||
    text.includes("powerpoint") ||
    text.includes("outlook") ||
    text.includes("onenote") ||
    text.includes("libreoffice") ||
    text.includes("openoffice") ||
    text.includes("notion") ||
    text.includes("obsidian") ||
    text.includes("logseq") ||
    text.includes("evernote") ||
    text.includes("acrobat") ||
    text.includes("foxit") ||
    text.includes("notes") ||
    text.includes("sticky notes") ||
    text.includes("calendar") ||
    text.includes("tasks") ||
    text.includes("sheets") ||
    text.includes("docs") ||
    text.includes("slides") ||
    path.includes("office") ||
    path.includes("productivity")
  ) {
    result = "productivity";
  }
  // 7. Utilities & System Tools
  else if (
    text.includes("settings") ||
    text.includes("control panel") ||
    text.includes("task manager") ||
    text.includes("file explorer") ||
    text.includes("files") ||
    text.includes("registry editor") ||
    text.includes("disk cleanup") ||
    text.includes("snipping tool") ||
    text.includes("snip & sketch") ||
    text.includes("7-zip") ||
    text.includes("winrar") ||
    text.includes("everything") ||
    text.includes("calculator") ||
    text.includes("character map") ||
    text.includes("resource monitor") ||
    text.includes("device manager") ||
    text.includes("services") ||
    text.includes("system tools") ||
    text.includes("windows accessories") ||
    text.includes("administrative tools") ||
    text.includes("accessibility") ||
    text.includes("powertoys") ||
    text.includes("browser") ||
    text.includes("firefox") ||
    text.includes("chrome") ||
    text.includes("edge")
  ) {
    result = "utilities";
  }

  appCache.set(cacheKey, result);
  return result;
}

/**
 * Classifies an indexed file based on genuine file type and index metadata.
 */
export function classifyFile(item: LauncherItem | SearchResultItem): FileSubcategory {
  const cacheKey = item.path || item.name;
  const cached = fileCache.get(cacheKey);
  if (cached) {
    return cached;
  }

  if (item.rightLabel === "Folder") {
    fileCache.set(cacheKey, "folders");
    return "folders";
  }

  if (item.isImage) {
    fileCache.set(cacheKey, "images");
    return "images";
  }

  const path = item.path || item.name;
  const lastDot = path.lastIndexOf(".");
  const ext = lastDot >= 0 ? path.slice(lastDot + 1).toLowerCase() : "";

  let result: FileSubcategory = "other";

  if (IMAGE_EXTS.has(ext)) {
    result = "images";
  } else if (DOCUMENT_EXTS.has(ext)) {
    result = "documents";
  } else if (VIDEO_EXTS.has(ext)) {
    result = "videos";
  } else if (AUDIO_EXTS.has(ext)) {
    result = "audio";
  } else if (ARCHIVE_EXTS.has(ext)) {
    result = "archives";
  }

  fileCache.set(cacheKey, result);
  return result;
}

/**
 * Classifies a supported Lux command/action into meaningful categories.
 */
export function classifyAction(item: LauncherItem): ActionSubcategory {
  const name = item.name.toLowerCase();
  const keywords = item.keywords.toLowerCase();
  const text = `${name} ${keywords}`;

  if (
    text.includes("settings") ||
    text.includes("preferences") ||
    text.includes("theme") ||
    text.includes("system")
  ) {
    return "preferences";
  }

  return "tools";
}

/**
 * Derives the active contextual subcategory options that have useful results,
 * with "All" always available as the first option.
 */
export function getAvailableSubcategories(
  primaryCategory: SearchFilter,
  items: readonly LauncherItem[],
): SubcategoryItem[] {
  if (primaryCategory === "all") {
    return [];
  }

  if (primaryCategory === "apps") {
    const presentCategories = new Set<AppSubcategory>();
    for (const item of items) {
      if (item.category === "app") {
        presentCategories.add(classifyApp(item));
      }
    }

    const available: SubcategoryItem[] = [
      { id: "all", label: APP_SUBCATEGORY_LABELS.all },
    ];

    for (const subcat of APP_SUBCATEGORY_ORDER) {
      if (subcat !== "all" && presentCategories.has(subcat)) {
        available.push({ id: subcat, label: APP_SUBCATEGORY_LABELS[subcat] });
      }
    }

    return available;
  }

  if (primaryCategory === "files") {
    const presentCategories = new Set<FileSubcategory>();
    for (const item of items) {
      if (item.category === "file") {
        presentCategories.add(classifyFile(item));
      }
    }

    const available: SubcategoryItem[] = [
      { id: "all", label: FILE_SUBCATEGORY_LABELS.all },
    ];

    for (const subcat of FILE_SUBCATEGORY_ORDER) {
      if (subcat !== "all" && presentCategories.has(subcat)) {
        available.push({ id: subcat, label: FILE_SUBCATEGORY_LABELS[subcat] });
      }
    }

    return available;
  }

  if (primaryCategory === "actions") {
    const presentCategories = new Set<ActionSubcategory>();
    for (const item of items) {
      if (item.category === "cmd") {
        presentCategories.add(classifyAction(item));
      }
    }

    const available: SubcategoryItem[] = [
      { id: "all", label: ACTION_SUBCATEGORY_LABELS.all },
    ];

    for (const subcat of ACTION_SUBCATEGORY_ORDER) {
      if (subcat !== "all" && presentCategories.has(subcat)) {
        available.push({ id: subcat, label: ACTION_SUBCATEGORY_LABELS[subcat] });
      }
    }

    return available;
  }

  return [];
}

/** Clears classification caches (e.g. on reindex). */
export function clearClassificationCache(): void {
  appCache.clear();
  fileCache.clear();
}
