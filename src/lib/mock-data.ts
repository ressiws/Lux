import { convertFileSrc } from "@tauri-apps/api/core";
import { evaluateMathExpression } from "./calculator";
import { classifyApp, classifyFile, classifyAction, detectFileFormat, getFileFormatBadge } from "./classification";
import type { FileFormatKind } from "./types";
import type {
  HighlightParts,
  ItemAction,
  ItemCategory,
  LauncherItem,
  LauncherSection,
  SearchResponse,
  SearchResultItem,
} from "./types";

export function getFileThumbnail(
  path: string,
  isImage: boolean,
): string | undefined {
  if (!isImage) {
    return undefined;
  }
  try {
    if (
      typeof window !== "undefined" &&
      Boolean(
        (window as unknown as { __TAURI_INTERNALS__?: unknown })
          .__TAURI_INTERNALS__,
      )
    ) {
      return convertFileSrc(path);
    }
  } catch {
    // Return undefined when running outside a Tauri webview
  }
  return undefined;
}

export function mapBackendApp(app: SearchResultItem): LauncherItem {
  return {
    category: "app",
    name: app.name,
    subtitle: app.subtitle,
    path: app.path,
    location: app.location,
    color: app.color ?? "#4a8fd8",
    icon: app.icon ?? undefined,
    size: app.size ?? undefined,
    version: app.version ?? undefined,
    keywords: "",
    rightLabel: "Application",
    primaryLabel: "Open Application",
    actions: [
      { label: "Open Application", shortcut: "↵" },
      { label: "Reveal in Folder", shortcut: "Ctrl ↵" },
      { label: "Copy Path", shortcut: "Ctrl ⇧ C" },
      { label: "Run as Administrator", shortcut: "Ctrl ⇧ ↵" },
    ],
  };
}

export function mapBackendFile(file: SearchResultItem): LauncherItem {
  const isImage = file.isImage;
  const thumbnail = file.thumbnail || getFileThumbnail(file.path, isImage);
  const label = isImage ? "Image" : file.rightLabel;
  return {
    category: "file",
    name: file.name,
    subtitle: file.subtitle,
    path: file.path,
    location: file.location,
    icon: file.icon ?? undefined,
    isImage,
    thumbnail,
    size: file.size ?? undefined,
    version: file.version ?? undefined,
    keywords: "",
    rightLabel: label,
    primaryLabel: isImage ? "Open Image" : `Open ${label}`,
    actions: [
      { label: isImage ? "Open Image" : `Open ${label}`, shortcut: "↵" },
      { label: "Reveal in Folder", shortcut: "Ctrl ↵" },
      { label: "Copy File Path", shortcut: "Ctrl ⇧ C" },
    ],
  };
}

function createApp(
  name: string,
  subtitle: string,
  color: string,
  keywords = "",
): LauncherItem {
  return {
    category: "app",
    name,
    subtitle,
    color,
    keywords,
    rightLabel: "Application",
    primaryLabel: "Open Application",
    actions: [
      { label: "Open Application", shortcut: "↵" },
      { label: "Show in Files", shortcut: "Ctrl ↵" },
      { label: "Copy Path", shortcut: "Ctrl ⇧ C" },
      { label: "Run as Administrator", shortcut: "Ctrl ⇧ ↵" },
      { label: "Uninstall", shortcut: "", danger: true },
    ],
  };
}

function createFile(
  name: string,
  subtitle: string,
  thumbnail: string,
  keywords = "",
  rightLabel = "Image",
): LauncherItem {
  return {
    category: "file",
    name,
    subtitle,
    thumbnail,
    keywords,
    rightLabel,
    primaryLabel: `Open ${rightLabel}`,
    actions: [
      { label: `Open ${rightLabel}`, shortcut: "↵" },
      { label: "Quick Look", shortcut: "Ctrl Y" },
      { label: "Reveal in Folder", shortcut: "Ctrl ↵" },
      { label: "Copy File Path", shortcut: "Ctrl ⇧ C" },
      { label: "Move to Trash", shortcut: "Del", danger: true },
    ],
  };
}

function createCommand(
  name: string,
  subtitle: string,
  keywords = "",
  primaryLabel = "Run Command",
): LauncherItem {
  return {
    category: "cmd",
    name,
    subtitle,
    keywords,
    rightLabel: "Command",
    primaryLabel,
    actions: [
      { label: "Run Command", shortcut: "↵" },
      { label: "Assign Hotkey", shortcut: "Ctrl H" },
      { label: "Copy Deeplink", shortcut: "Ctrl ⇧ L" },
    ],
  };
}

function createClipboardItem(
  name: string,
  subtitle: string,
  keywords: string,
  actions: readonly ItemAction[],
  thumbnail?: string,
): LauncherItem {
  return {
    category: "clip",
    name,
    subtitle,
    keywords,
    thumbnail,
    rightLabel: "Clipboard",
    primaryLabel: "Paste to Active App",
    actions,
  };
}

function createSnippetItem(
  name: string,
  subtitle: string,
  keywords: string,
  rightLabel: string,
  actions: readonly ItemAction[],
): LauncherItem {
  return {
    category: "snip",
    name,
    subtitle,
    keywords,
    rightLabel,
    primaryLabel: "Paste Snippet",
    actions,
  };
}

export const MOCK_ITEMS: readonly LauncherItem[] = [
  createApp("Firefox", "Web browser", "#e8663d", "browser"),
  createApp("Visual Studio Code", "Code editor", "#2f80d1", "vscode editor"),
  createApp("darktable", "Photo workflow", "#6b6f7a", "photo raw edit"),
  createApp("LightCraft", "Photo editor", "#c9892b", "photo editor lightroom"),
  createApp("GIMP", "Image editor", "#7a6a55", "photo image"),
  createApp("Files", "File manager", "#4a8fd8", "explorer"),
  createApp("Terminal", "Command line", "#3a3a40", "shell"),
  createApp("Spotify", "Music", "#2f9e5b", "music"),
  createFile(
    "wedding-selects-0412.jpg",
    "~/Pictures/Wedding/Selects",
    "linear-gradient(160deg,#f4d9c6,#c98f7a 55%,#6b4f5c)",
    "photo",
  ),
  createFile(
    "IMG_4821.jpg",
    "~/Pictures/Camera",
    "linear-gradient(170deg,#9fc4e8,#e9d3a8 60%,#5d7a52)",
    "photo",
  ),
  createFile(
    "photo-session-contract.pdf",
    "~/Documents/Studio",
    "#d9d9de",
    "photo contract",
    "PDF",
  ),
  createFile("Wedding", "~/Pictures", "#d9d9de", "wedding photo", "Folder"),
  createFile(
    "portfolio-hero.png",
    "~/Projects/site/assets",
    "linear-gradient(135deg,#2b2f3a,#6d7a8f)",
    "photo portfolio",
  ),
  createCommand(
    "Toggle Theme",
    "Switch between light and dark",
    "theme dark light",
    "Toggle Theme",
  ),
  createCommand("Open Lux Settings", "Preferences", "settings preferences"),
  createCommand(
    "Clipboard History",
    "Browse recent copies",
    "clip clipboard",
  ),
  createCommand(
    "Manage Snippets",
    "Create and edit text shortcuts",
    "snip snippets",
  ),
  createCommand(
    "Color Picker",
    "Pick a color from the screen",
    "color tools",
  ),
  createClipboardItem(
    "Invoice #2026-014 — due 30 Oct",
    "Text · 4 min ago",
    "clip invoice",
    [
      { label: "Paste to Active App", shortcut: "↵" },
      { label: "Copy to Clipboard", shortcut: "Ctrl C" },
      { label: "Pin Entry", shortcut: "Ctrl P" },
      { label: "Delete Entry", shortcut: "Del", danger: true },
    ],
  ),
  createSnippetItem(
    "Booking confirmation",
    "Hi {name}, your session is booked for {date}…",
    "snip booking",
    ";book",
    [
      { label: "Paste Snippet", shortcut: "↵" },
      { label: "Edit Snippet", shortcut: "Ctrl E" },
      { label: "Duplicate", shortcut: "Ctrl D" },
      { label: "Delete Snippet", shortcut: "Del", danger: true },
    ],
  ),
  createClipboardItem(
    "https://swisser.pt/portfolio",
    "Link · 21 min ago",
    "clip link url",
    [
      { label: "Paste to Active App", shortcut: "↵" },
      { label: "Open in Browser", shortcut: "Ctrl O" },
      { label: "Pin Entry", shortcut: "Ctrl P" },
      { label: "Delete Entry", shortcut: "Del", danger: true },
    ],
  ),
  createClipboardItem(
    "#3B3550",
    "Color · 1 h ago",
    "clip color",
    [
      { label: "Paste to Active App", shortcut: "↵" },
      { label: "Copy to Clipboard", shortcut: "Ctrl C" },
      { label: "Delete Entry", shortcut: "Del", danger: true },
    ],
    "#3b3550",
  ),
  createSnippetItem(
    "Studio address",
    "Rua da Prata 120, 1100-420 Lisboa",
    "snip address",
    ";addr",
    [
      { label: "Paste Snippet", shortcut: "↵" },
      { label: "Edit Snippet", shortcut: "Ctrl E" },
      { label: "Duplicate", shortcut: "Ctrl D" },
      { label: "Delete Snippet", shortcut: "Del", danger: true },
    ],
  ),
  createSnippetItem(
    "Email signature",
    "Best regards, — studio name",
    "snip signature",
    ";sig",
    [
      { label: "Paste Snippet", shortcut: "↵" },
      { label: "Edit Snippet", shortcut: "Ctrl E" },
      { label: "Delete Snippet", shortcut: "Del", danger: true },
    ],
  ),
];

export const MOCK_COMMANDS: readonly LauncherItem[] = [
  createCommand(
    "Toggle Theme",
    "Switch between light and dark",
    "theme dark light",
    "Toggle Theme",
  ),
  createCommand("Open Lux Settings", "Preferences", "settings preferences"),
  createCommand(
    "Clipboard History",
    "Browse recent copies",
    "clip clipboard",
  ),
  createCommand(
    "Manage Snippets",
    "Create and edit text shortcuts",
    "snip snippets",
  ),
  createCommand(
    "Color Picker",
    "Pick a color from the screen",
    "color tools",
  ),
];

export const SECTION_TITLES: Readonly<Record<ItemCategory, string>> = {
  calc: "Calculator",
  app: "Applications",
  file: "Files",
  cmd: "Commands",
  clip: "Clipboard history",
  snip: "Snippets",
  fb: "Search elsewhere",
};

export const CATEGORY_ICONS: Readonly<Record<string, string>> = {
  file: "M7 3h7l4 4v14H7z M14 3v4h4",
  folder: "M3 7v10a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-6l-2-2H5a2 2 0 0 0-2 2z",
  image:
    "M4 5h16a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1z M8.5 10a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3z M21 15l-5-5L5 21",
  cmd: "M9 6a3 3 0 1 0-3 3h12a3 3 0 1 0-3-3v12a3 3 0 1 0 3-3H6a3 3 0 1 0 3 3z",
  clip: "M9 4h6v3H9z M7 5H6v16h12V5h-1",
  snip: "M5 7h14M5 12h14M5 17h8",
  fb: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z M3 12h18 M12 3c3 3 3 15 0 18 M12 3c-3 3-3 15 0 18",
};

export const FORMAT_ICONS: Readonly<Record<FileFormatKind, string>> = {
  pdf: "M7 3h7l4 4v14H7z M14 3v4h4 M9 13h6 M9 17h4",
  markdown: "M7 3h7l4 4v14H7z M14 3v4h4 M10 12l2 2 2-2 M12 10v4",
  text: "M7 3h7l4 4v14H7z M14 3v4h4 M9 12h6 M9 15h6 M9 18h4",
  code: "M7 3h7l4 4v14H7z M14 3v4h4 M11 13l-2 2 2 2 M13 13l2 2-2 2",
  archive: "M4 5h16v14H4z M4 9h16 M10 5v4 M14 5v4 M11 11h2v3h-2z",
  image: "M4 5h16a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1z M8.5 10a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3z M21 15l-5-5L5 21",
  video: "M3 5h18v14H3z M10 9l5 3-5 3V9z",
  audio: "M9 18V5l10-2v13 M6 18a3 3 0 1 0 3-3H9 M16 16a3 3 0 1 0 3-3h-3",
  document: "M7 3h7l4 4v14H7z M14 3v4h4 M9 11h6 M9 14h6 M9 17h6",
  folder: "M3 7v10a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-6l-2-2H5a2 2 0 0 0-2 2z",
  generic: "M7 3h7l4 4v14H7z M14 3v4h4",
};

export const FORMAT_COLORS: Readonly<Record<FileFormatKind, string>> = {
  pdf: "#e05555",
  markdown: "#42b983",
  text: "#9da5b4",
  code: "#4a8fd8",
  archive: "#d19a66",
  image: "#56b6c2",
  video: "#e06c75",
  audio: "#c678dd",
  document: "#61afef",
  folder: "#e5c07b",
  generic: "#828997",
};

export function getFileFormatInfo(item: LauncherItem | SearchResultItem): {
  kind: FileFormatKind;
  iconPath: string;
  badge: string;
  color: string;
} {
  const kind = detectFileFormat(item);
  const iconPath = FORMAT_ICONS[kind] ?? FORMAT_ICONS.generic;
  const badge = getFileFormatBadge(kind);
  const color = FORMAT_COLORS[kind] ?? FORMAT_COLORS.generic;
  return { kind, iconPath, badge, color };
}

export interface ItemDisplayIcon {
  type: "thumb" | "shell" | "format";
  src?: string;
  path?: string;
  badge?: string;
  color?: string;
}

export function getItemDisplayIcon(item: LauncherItem): ItemDisplayIcon {
  if (item.category === "app") {
    return item.icon
      ? { type: "shell", src: item.icon }
      : { type: "format", path: "M3 3h18v18H3z M3 9h18M9 21V9", color: "#4a8fd8" };
  }

  if (item.category === "file") {
    const { kind, iconPath, badge, color } = getFileFormatInfo(item);

    // Media and images: prioritize real thumbnail if available
    if ((kind === "image" || kind === "video") && item.thumbnail) {
      return { type: "thumb", src: item.thumbnail, badge, color };
    }

    // Documents, code, archives, text: prioritize format-specific icon
    if (
      kind === "pdf" ||
      kind === "markdown" ||
      kind === "code" ||
      kind === "archive" ||
      kind === "text"
    ) {
      return { type: "format", path: iconPath, badge, color };
    }

    // Other files: use shell icon if available, else format icon
    if (item.icon) {
      return { type: "shell", src: item.icon, badge, color };
    }

    return { type: "format", path: iconPath, badge, color };
  }

  if (item.thumbnail) {
    return { type: "thumb", src: item.thumbnail };
  }
  if (item.icon) {
    return { type: "shell", src: item.icon };
  }
  return {
    type: "format",
    path: CATEGORY_ICONS[item.category] ?? CATEGORY_ICONS.file,
  };
}

const SUGGESTED_APP_NAMES = new Set([
  "Firefox",
  "LightCraft",
  "Visual Studio Code",
]);

export function scoreItem(item: LauncherItem, loweredQuery: string): number {
  const nameLow = item.name.toLowerCase();
  const haystack = `${nameLow} ${item.keywords} ${item.subtitle.toLowerCase()}`;
  if (nameLow.startsWith(loweredQuery)) {
    return 3;
  }
  if (nameLow.split(/[\s_-]/).some((word) => word.startsWith(loweredQuery))) {
    return 2.5;
  }
  if (item.keywords.split(" ").some((word) => word.startsWith(loweredQuery))) {
    return 2;
  }
  return haystack.includes(loweredQuery) ? 1 : 0;
}

export type SearchFilter = "all" | "apps" | "files" | "actions";

export function getCategoryPlaceholder(filter: SearchFilter): string {
  switch (filter) {
    case "apps":
      return "Search Applications…";
    case "files":
      return "Search Files…";
    case "actions":
      return "Search Actions…";
    case "all":
    default:
      return "Search Anything…";
  }
}

export interface BuildSectionsOptions {
  enableFileSearch?: boolean;
  filter?: SearchFilter;
  subcategory?: string;
  browseResponse?: SearchResponse;
  mockItemsForTesting?: readonly LauncherItem[];
  defaultBrowser?: { name: string; icon?: string | null };
}

function deduplicateLauncherItems(
  items: readonly LauncherItem[],
): LauncherItem[] {
  const seen = new Set<string>();
  const out: LauncherItem[] = [];
  for (const item of items) {
    const key = item.path ? item.path.toLowerCase() : item.name.toLowerCase();
    if (!seen.has(key)) {
      seen.add(key);
      out.push(item);
    }
  }
  return out;
}

export function buildSections(
  rawQuery: string,
  searchResponse?: SearchResponse,
  options?: BuildSectionsOptions,
): LauncherSection[] {
  const trimmed = rawQuery.trim();
  const lowered = trimmed.toLowerCase();
  const filter = options?.filter ?? "all";
  const subcategory = options?.subcategory ?? "all";
  const enableFileSearch = options?.enableFileSearch ?? true;
  const testItems = options?.mockItemsForTesting;

  // 1. Filter: Apps
  if (filter === "apps") {
    let apps: LauncherItem[];
    if (trimmed) {
      if (searchResponse) {
        apps = deduplicateLauncherItems(searchResponse.apps.map(mapBackendApp));
      } else if (testItems) {
        apps = testItems.filter(
          (item) => item.category === "app" && scoreItem(item, lowered) > 0,
        );
      } else {
        apps = [];
      }
    } else {
      // Browse mode: show all available applications
      if (searchResponse && searchResponse.apps.length > 0) {
        apps = deduplicateLauncherItems(searchResponse.apps.map(mapBackendApp));
      } else if (options?.browseResponse && options.browseResponse.apps.length > 0) {
        apps = deduplicateLauncherItems(options.browseResponse.apps.map(mapBackendApp));
      } else if (testItems) {
        apps = testItems.filter((item) => item.category === "app");
      } else {
        apps = [];
      }
    }

    if (subcategory !== "all") {
      apps = apps.filter((app) => classifyApp(app) === subcategory);
    }

    return apps.length > 0
      ? [
          {
            category: "app",
            title: SECTION_TITLES.app,
            items: apps,
          },
        ]
      : [];
  }

  // 2. Filter: Files
  if (filter === "files") {
    if (!enableFileSearch) {
      return [];
    }

    let files: LauncherItem[];
    if (trimmed) {
      if (searchResponse) {
        files = deduplicateLauncherItems(searchResponse.files.map(mapBackendFile));
      } else if (testItems) {
        files = testItems.filter(
          (item) => item.category === "file" && scoreItem(item, lowered) > 0,
        );
      } else {
        files = [];
      }
    } else {
      // Browse mode: show all available files, folders, and images
      if (searchResponse && searchResponse.files.length > 0) {
        files = deduplicateLauncherItems(searchResponse.files.map(mapBackendFile));
      } else if (options?.browseResponse && options.browseResponse.files.length > 0) {
        files = deduplicateLauncherItems(options.browseResponse.files.map(mapBackendFile));
      } else if (testItems) {
        files = testItems.filter((item) => item.category === "file");
      } else {
        files = [];
      }
    }

    if (subcategory !== "all") {
      files = files.filter((file) => classifyFile(file) === subcategory);
    }

    return files.length > 0
      ? [
          {
            category: "file",
            title: SECTION_TITLES.file,
            items: files,
          },
        ]
      : [];
  }

  // 3. Filter: Actions
  if (filter === "actions") {
    let cmds = trimmed
      ? MOCK_COMMANDS.filter((cmd) => scoreItem(cmd, lowered) > 0)
      : [...MOCK_COMMANDS];

    if (subcategory !== "all") {
      cmds = cmds.filter((cmd) => classifyAction(cmd) === subcategory);
    }

    return cmds.length > 0
      ? [
          {
            category: "cmd",
            title: trimmed ? "Actions" : "Available Actions",
            items: cmds,
          },
        ]
      : [];
  }

  // 4. Default / All categories
  const suggestedApps =
    searchResponse && searchResponse.apps.length > 0
      ? deduplicateLauncherItems(searchResponse.apps.slice(0, 4).map(mapBackendApp))
      : options?.browseResponse && options.browseResponse.apps.length > 0
        ? deduplicateLauncherItems(options.browseResponse.apps.slice(0, 4).map(mapBackendApp))
        : testItems
          ? testItems.filter((item) => SUGGESTED_APP_NAMES.has(item.name))
          : [];

  if (!trimmed) {
    return suggestedApps.length > 0
      ? [
          {
            category: "app",
            title: "Suggestions",
            items: suggestedApps,
          },
        ]
      : [];
  }

  const sections: LauncherSection[] = [];

  const calcPreview = evaluateMathExpression(trimmed);
  if (calcPreview) {
    sections.push({
      category: "calc",
      title: SECTION_TITLES.calc,
      items: [
        {
          category: "calc",
          name: calcPreview.value,
          subtitle: calcPreview.expression,
          keywords: "",
          rightLabel: calcPreview.categoryLabel ?? "Quick Answer",
          color: calcPreview.colorPreview,
          primaryLabel:
            calcPreview.categoryLabel === "Password"
              ? "Copy Password"
              : "Copy Result",
          calc: calcPreview,
          actions: [
            { label: "Copy Result", shortcut: "↵" },
            { label: "Copy Expression", shortcut: "Ctrl ⇧ C" },
            { label: "Paste Result to App", shortcut: "Ctrl P" },
          ],
        },
      ],
    });
  }

  if (searchResponse) {
    if (searchResponse.apps.length > 0) {
      sections.push({
        category: "app",
        title: SECTION_TITLES.app,
        items: deduplicateLauncherItems(searchResponse.apps.map(mapBackendApp)),
      });
    }

    if (enableFileSearch && searchResponse.files.length > 0) {
      sections.push({
        category: "file",
        title: SECTION_TITLES.file,
        items: deduplicateLauncherItems(searchResponse.files.map(mapBackendFile)),
      });
    }

    const matchingCmds = MOCK_COMMANDS.filter(
      (cmd) => scoreItem(cmd, lowered) > 0,
    );
    if (matchingCmds.length > 0) {
      sections.push({
        category: "cmd",
        title: SECTION_TITLES.cmd,
        items: matchingCmds,
      });
    }
  } else if (testItems) {
    const grouped = new Map<
      ItemCategory,
      Array<{ score: number; item: LauncherItem }>
    >();

    for (const item of testItems) {
      if (!enableFileSearch && item.category === "file") {
        continue;
      }
      const score = scoreItem(item, lowered);
      if (score > 0) {
        const list = grouped.get(item.category) ?? [];
        list.push({ score, item });
        grouped.set(item.category, list);
      }
    }

    const singleCategory = grouped.size === 1;
    const sortedGroups = Array.from(grouped.entries())
      .map(([category, entries]) => ({
        category,
        entries: entries.sort((a, b) => b.score - a.score),
      }))
      .sort((a, b) => b.entries[0].score - a.entries[0].score);

    for (const { category, entries } of sortedGroups) {
      sections.push({
        category,
        title: SECTION_TITLES[category],
        items: entries
          .slice(0, singleCategory ? 8 : 3)
          .map((entry) => entry.item),
      });
    }
  }

  if (sections.length === 0) {
    sections.push({
      category: "fb",
      title: SECTION_TITLES.fb,
      items: [
        {
          category: "fb",
          name: `Search the web for “${trimmed}”`,
          subtitle: options?.defaultBrowser?.name || "Default browser",
          icon: options?.defaultBrowser?.icon || undefined,
          keywords: "",
          rightLabel: "Fallback",
          primaryLabel: "Search the Web",
          actions: [
            { label: "Search the Web", shortcut: "↵" },
            { label: "Search Files for This Term", shortcut: "Ctrl F" },
          ],
        },
        {
          category: "fb",
          name: `Search files named “${trimmed}”`,
          subtitle: "Entire home folder",
          keywords: "",
          rightLabel: "Fallback",
          primaryLabel: "Search Files",
          actions: [{ label: "Search Files", shortcut: "↵" }],
        },
      ],
    });
  }

  return sections;
}
export function splitHighlight(text: string, query: string): HighlightParts {
  const trimmed = query.trim();
  if (!trimmed) {
    return { before: text, match: "", after: "" };
  }
  const index = text.toLowerCase().indexOf(trimmed.toLowerCase());
  if (index < 0) {
    return { before: text, match: "", after: "" };
  }
  return {
    before: text.slice(0, index),
    match: text.slice(index, index + trimmed.length),
    after: text.slice(index + trimmed.length),
  };
}

export function hasOnlyFallbackSections(
  sections: readonly LauncherSection[],
): boolean {
  return (
    sections.length > 0 &&
    sections.every((section) => section.category === "fb")
  );
}
