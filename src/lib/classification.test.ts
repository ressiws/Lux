import {
  classifyApp,
  classifyFile,
  classifyAction,
  getAvailableSubcategories,
  clearClassificationCache,
  detectFileFormat,
  getFileFormatBadge,
} from "./classification";
import type { LauncherItem } from "./types";
import { MOCK_ITEMS, MOCK_COMMANDS, getItemDisplayIcon } from "./mock-data";

function assertEqual<T>(actual: T, expected: T, message: string): void {
  if (actual !== expected) {
    throw new Error(
      `${message}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`,
    );
  }
}

export function runClassificationTests(): number {
  let passed = 0;
  const check = (fn: () => void): void => {
    fn();
    passed += 1;
  };

  clearClassificationCache();

  // 1. App classification: developer tools
  check(() => {
    const vscode: LauncherItem = {
      category: "app",
      name: "Visual Studio Code",
      subtitle: "",
      keywords: "editor code",
      rightLabel: "Application",
      primaryLabel: "Open",
      actions: [],
    };
    assertEqual(classifyApp(vscode), "devtools", "VS Code is devtools");

    const git: LauncherItem = {
      category: "app",
      name: "Git Bash",
      subtitle: "",
      keywords: "",
      rightLabel: "Application",
      primaryLabel: "Open",
      actions: [],
    };
    assertEqual(classifyApp(git), "devtools", "Git is devtools");
  });

  // 2. App classification: productivity, social, entertainment, utilities, games, finance, other
  check(() => {
    const word: LauncherItem = {
      category: "app",
      name: "Microsoft Word",
      subtitle: "",
      keywords: "document office",
      rightLabel: "Application",
      primaryLabel: "Open",
      actions: [],
    };
    assertEqual(classifyApp(word), "productivity", "Word is productivity");

    const discord: LauncherItem = {
      category: "app",
      name: "Discord",
      subtitle: "",
      keywords: "chat",
      rightLabel: "Application",
      primaryLabel: "Open",
      actions: [],
    };
    assertEqual(classifyApp(discord), "social", "Discord is social");

    const spotify: LauncherItem = {
      category: "app",
      name: "Spotify",
      subtitle: "",
      keywords: "music",
      rightLabel: "Application",
      primaryLabel: "Open",
      actions: [],
    };
    assertEqual(classifyApp(spotify), "entertainment", "Spotify is entertainment");

    const taskmgr: LauncherItem = {
      category: "app",
      name: "Task Manager",
      subtitle: "",
      keywords: "system",
      rightLabel: "Application",
      primaryLabel: "Open",
      actions: [],
    };
    assertEqual(classifyApp(taskmgr), "utilities", "Task Manager is utilities");

    const steam: LauncherItem = {
      category: "app",
      name: "Steam",
      subtitle: "",
      keywords: "games",
      rightLabel: "Application",
      primaryLabel: "Open",
      actions: [],
    };
    assertEqual(classifyApp(steam), "games", "Steam is games");

    const qb: LauncherItem = {
      category: "app",
      name: "QuickBooks 2024",
      subtitle: "",
      keywords: "accounting",
      rightLabel: "Application",
      primaryLabel: "Open",
      actions: [],
    };
    assertEqual(classifyApp(qb), "finance", "QuickBooks is finance");

    const unknownApp: LauncherItem = {
      category: "app",
      name: "CustomBespokeInternalWidget",
      subtitle: "",
      keywords: "",
      rightLabel: "Application",
      primaryLabel: "Open",
      actions: [],
    };
    assertEqual(classifyApp(unknownApp), "other", "Unknown app falls back to other");
  });

  // 3. File classification: folders, images, documents, videos, audio, archives
  check(() => {
    const folder: LauncherItem = {
      category: "file",
      name: "Projects",
      subtitle: "D:\\",
      keywords: "",
      rightLabel: "Folder",
      primaryLabel: "Open Folder",
      actions: [],
    };
    assertEqual(classifyFile(folder), "folders", "Folder is folders");

    const img: LauncherItem = {
      category: "file",
      name: "screenshot.png",
      subtitle: "",
      keywords: "",
      rightLabel: "Image",
      isImage: true,
      primaryLabel: "Open",
      actions: [],
    };
    assertEqual(classifyFile(img), "images", "PNG is images");

    const doc: LauncherItem = {
      category: "file",
      name: "report.pdf",
      subtitle: "",
      keywords: "",
      rightLabel: "PDF",
      primaryLabel: "Open",
      actions: [],
    };
    assertEqual(classifyFile(doc), "documents", "PDF is documents");

    const video: LauncherItem = {
      category: "file",
      name: "intro.mp4",
      subtitle: "",
      keywords: "",
      rightLabel: "MP4",
      primaryLabel: "Open",
      actions: [],
    };
    assertEqual(classifyFile(video), "videos", "MP4 is videos");

    const audio: LauncherItem = {
      category: "file",
      name: "song.flac",
      subtitle: "",
      keywords: "",
      rightLabel: "FLAC",
      primaryLabel: "Open",
      actions: [],
    };
    assertEqual(classifyFile(audio), "audio", "FLAC is audio");

    const archive: LauncherItem = {
      category: "file",
      name: "backup.zip",
      subtitle: "",
      keywords: "",
      rightLabel: "ZIP",
      primaryLabel: "Open",
      actions: [],
    };
    assertEqual(classifyFile(archive), "archives", "ZIP is archives");
  });

  // 4. Action classification
  check(() => {
    const themeCmd = MOCK_COMMANDS.find((c) => c.name === "Toggle Theme")!;
    assertEqual(classifyAction(themeCmd), "preferences", "Toggle Theme is preferences");

    const settingsCmd = MOCK_COMMANDS.find((c) => c.name === "Open Lux Settings")!;
    assertEqual(classifyAction(settingsCmd), "preferences", "Open Settings is preferences");

    const clipCmd = MOCK_COMMANDS.find((c) => c.name === "Clipboard History")!;
    assertEqual(classifyAction(clipCmd), "tools", "Clipboard History is tools");

    const pickerCmd = MOCK_COMMANDS.find((c) => c.name === "Color Picker")!;
    assertEqual(classifyAction(pickerCmd), "tools", "Color Picker is tools");
  });

  // 5. getAvailableSubcategories returns only populated categories + All
  check(() => {
    const appSubcats = getAvailableSubcategories("apps", MOCK_ITEMS);
    assertEqual(appSubcats[0].id, "all", "First is All");
    assertEqual(appSubcats[0].label, "All", "First label is All");
    assertEqual(appSubcats.some((s) => s.id === "devtools"), true, "devtools is included");
    assertEqual(appSubcats.some((s) => s.id === "entertainment"), true, "entertainment is included");
    // MOCK_ITEMS has no finance apps, so finance shouldn't be included
    assertEqual(appSubcats.some((s) => s.id === "finance"), false, "finance not in mock items");

    const fileSubcats = getAvailableSubcategories("files", MOCK_ITEMS);
    assertEqual(fileSubcats[0].id, "all", "First is All");
    assertEqual(fileSubcats.some((s) => s.id === "folders"), true, "folders present");
    assertEqual(fileSubcats.some((s) => s.id === "documents"), true, "documents present");
    assertEqual(fileSubcats.some((s) => s.id === "images"), true, "images present");
    // MOCK_ITEMS has no videos or audio
    assertEqual(fileSubcats.some((s) => s.id === "videos"), false, "videos not in mock items");

    const actionSubcats = getAvailableSubcategories("actions", MOCK_COMMANDS);
    assertEqual(actionSubcats[0].id, "all", "First is All");
    assertEqual(actionSubcats.some((s) => s.id === "preferences"), true, "preferences present");
    assertEqual(actionSubcats.some((s) => s.id === "tools"), true, "tools present");

    const allSubcats = getAvailableSubcategories("all", MOCK_ITEMS);
    assertEqual(allSubcats.length, 0, "No subcategories when filter is all");
  });
  // 6. Format-aware file detection & icon precedence
  check(() => {
    assertEqual(detectFileFormat("report.pdf"), "pdf", "PDF file detection");
    assertEqual(detectFileFormat("README.md"), "markdown", "Markdown file detection");
    assertEqual(detectFileFormat("notes.txt"), "text", "Text file detection");
    assertEqual(detectFileFormat("main.rs"), "code", "Rust file detection");
    assertEqual(detectFileFormat("app.tsx"), "code", "TSX file detection");
    assertEqual(detectFileFormat("archive.zip"), "archive", "ZIP file detection");
    assertEqual(detectFileFormat("photo.png"), "image", "PNG file detection");
    assertEqual(detectFileFormat("movie.mp4"), "video", "MP4 file detection");
    assertEqual(detectFileFormat("song.mp3"), "audio", "MP3 file detection");
    assertEqual(detectFileFormat("doc.docx"), "document", "DOCX file detection");
    assertEqual(detectFileFormat("unknown.xyz"), "generic", "Generic file detection");

    assertEqual(getFileFormatBadge("pdf"), "PDF", "PDF badge");
    assertEqual(getFileFormatBadge("markdown"), "MD", "Markdown badge");
    assertEqual(getFileFormatBadge("code"), "CODE", "Code badge");
    assertEqual(getFileFormatBadge("archive"), "ZIP", "Archive badge");

    // Icon precedence tests
    const pdfItem: LauncherItem = {
      category: "file",
      name: "report.pdf",
      subtitle: "",
      keywords: "",
      rightLabel: "PDF",
      primaryLabel: "Open PDF",
      actions: [],
    };
    const pdfIcon = getItemDisplayIcon(pdfItem);
    assertEqual(pdfIcon.type, "format", "PDF uses format icon");
    assertEqual(pdfIcon.badge, "PDF", "PDF display icon badge");

    const imageWithThumb: LauncherItem = {
      category: "file",
      name: "photo.jpg",
      subtitle: "",
      keywords: "",
      rightLabel: "JPG",
      primaryLabel: "Open Image",
      thumbnail: "data:image/jpeg;base64,123",
      isImage: true,
      actions: [],
    };
    const thumbIcon = getItemDisplayIcon(imageWithThumb);
    assertEqual(thumbIcon.type, "thumb", "Image with thumbnail uses thumb type");
  });

  return passed;
}

const totalPassed = runClassificationTests();
console.log(`Classification test suite passed (${totalPassed} scenarios).`);
