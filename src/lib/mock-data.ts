import { convertFileSrc } from "@tauri-apps/api/core";
import { evaluateMathExpression } from "./calculator";
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
    color: app.color ?? "#4a8fd8",
    icon: app.icon ?? undefined,
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
  const thumbnail = getFileThumbnail(file.path, isImage);
  const label = isImage ? "Image" : file.rightLabel;
  return {
    category: "file",
    name: file.name,
    subtitle: file.subtitle,
    path: file.path,
    isImage,
    thumbnail,
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
  image:
    "M4 5h16a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1z M8.5 10a1.5 1.5 0 1 0 0-3 1.5 1.5 0 0 0 0 3z M21 15l-5-5L5 21",
  cmd: "M9 6a3 3 0 1 0-3 3h12a3 3 0 1 0-3-3v12a3 3 0 1 0 3-3H6a3 3 0 1 0 3 3z",
  clip: "M9 4h6v3H9z M7 5H6v16h12V5h-1",
  snip: "M5 7h14M5 12h14M5 17h8",
  fb: "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18z M3 12h18 M12 3c3 3 3 15 0 18 M12 3c-3 3-3 15 0 18",
};

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

export function buildSections(
  rawQuery: string,
  searchResponse?: SearchResponse,
): LauncherSection[] {
  const trimmed = rawQuery.trim();
  const lowered = trimmed.toLowerCase();

  if (!trimmed) {
    const suggestedApps =
      searchResponse && searchResponse.apps.length > 0
        ? searchResponse.apps.slice(0, 3).map(mapBackendApp)
        : MOCK_ITEMS.filter((item) => SUGGESTED_APP_NAMES.has(item.name));

    return [
      {
        category: "app",
        title: "Suggestions",
        items: suggestedApps,
      },
      {
        category: "cmd",
        title: "Commands",
        items: MOCK_COMMANDS.slice(0, 3),
      },
    ];
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
          rightLabel: "Calculator",
          primaryLabel: "Copy Result",
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
        items: searchResponse.apps.map(mapBackendApp),
      });
    }

    if (searchResponse.files.length > 0) {
      sections.push({
        category: "file",
        title: SECTION_TITLES.file,
        items: searchResponse.files.map(mapBackendFile),
      });
    }
  } else {
    const grouped = new Map<
      ItemCategory,
      Array<{ score: number; item: LauncherItem }>
    >();

    for (const item of MOCK_ITEMS) {
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

  const matchedCommands = MOCK_COMMANDS.filter(
    (cmd) => scoreItem(cmd, lowered) > 0,
  );
  if (matchedCommands.length > 0 && !sections.some((s) => s.category === "cmd")) {
    sections.push({
      category: "cmd",
      title: SECTION_TITLES.cmd,
      items: matchedCommands.slice(0, 3),
    });
  }

  if (sections.length === 0) {
    sections.push({
      category: "fb",
      title: SECTION_TITLES.fb,
      items: [
        {
          category: "fb",
          name: `Search the web for “${trimmed}”`,
          subtitle: "Default browser",
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
