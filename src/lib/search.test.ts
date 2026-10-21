import {
  buildSections,
  hasOnlyFallbackSections,
  mapBackendApp,
  mapBackendFile,
  scoreItem,
  splitHighlight,
} from "./mock-data";
import type { SearchResponse, SearchResultItem } from "./types";

function assertEqual<T>(actual: T, expected: T, message: string): void {
  if (actual !== expected) {
    throw new Error(
      `${message}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`,
    );
  }
}

export function runSearchIntegrationTests(): number {
  let passed = 0;
  const check = (fn: () => void): void => {
    fn();
    passed += 1;
  };

  const sampleApp: SearchResultItem = {
    id: "app:Discord.lnk",
    category: "app",
    name: "Discord",
    subtitle: "~/Start Menu/Discord",
    path: "C:\\Users\\swisser\\AppData\\Roaming\\Microsoft\\Windows\\Start Menu\\Programs\\Discord.lnk",
    location: "C:\\Users\\swisser\\AppData\\Local\\Discord\\app.exe",
    icon: null,
    thumbnail: null,
    color: "#5865f2",
    isImage: false,
    rightLabel: "Application",
    size: "120 MB",
    version: "1.0.9000",
  };

  const sampleImage: SearchResultItem = {
    id: "file:portfolio.png",
    category: "file",
    name: "portfolio.png",
    subtitle: "~/Pictures/portfolio.png",
    path: "C:\\Users\\swisser\\Pictures\\portfolio.png",
    location: "C:\\Users\\swisser\\Pictures",
    icon: null,
    thumbnail: "data:image/png;base64,sample",
    color: null,
    isImage: true,
    rightLabel: "Image",
    size: "2.4 MB",
    version: null,
  };

  const sampleDoc: SearchResultItem = {
    id: "file:notes.txt",
    category: "file",
    name: "notes.txt",
    subtitle: "~/Documents/notes.txt",
    path: "C:\\Users\\swisser\\Documents\\notes.txt",
    location: "C:\\Users\\swisser\\Documents",
    icon: null,
    thumbnail: null,
    color: null,
    isImage: false,
    rightLabel: "TXT",
    size: "1.2 KB",
    version: null,
  };

  // 1. Test mapping functions
  check(() => {
    const mappedApp = mapBackendApp(sampleApp);
    assertEqual(mappedApp.name, "Discord", "Mapped app name");
    assertEqual(mappedApp.primaryLabel, "Open Application", "Primary label");
    assertEqual(mappedApp.actions.length, 4, "App action count");
    assertEqual(mappedApp.actions[0].label, "Open Application", "Primary action");
    assertEqual(mappedApp.actions[1].label, "Reveal in Folder", "Reveal action");
    assertEqual(mappedApp.actions[3].label, "Run as Administrator", "Admin action");
  });

  check(() => {
    const mappedImg = mapBackendFile(sampleImage);
    assertEqual(mappedImg.name, "portfolio.png", "Image file name");
    assertEqual(mappedImg.isImage, true, "Is image");
    assertEqual(mappedImg.primaryLabel, "Open Image", "Primary label");
    assertEqual(mappedImg.actions[0].label, "Open Image", "Primary action");
  });

  check(() => {
    const mappedDoc = mapBackendFile(sampleDoc);
    assertEqual(mappedDoc.name, "notes.txt", "Doc name");
    assertEqual(mappedDoc.isImage, false, "Is image false");
    assertEqual(mappedDoc.primaryLabel, "Open TXT", "Primary label");
    assertEqual(mappedDoc.actions[0].label, "Open TXT", "Primary action");
  });

  const sampleFolder: SearchResultItem = {
    id: "file:Projects",
    category: "file",
    name: "Projects",
    subtitle: "D:\\",
    path: "D:\\Projects",
    location: "D:\\Projects",
    icon: null,
    thumbnail: null,
    color: null,
    isImage: false,
    rightLabel: "Folder",
    size: null,
    version: null,
  };

  check(() => {
    const mappedFolder = mapBackendFile(sampleFolder);
    assertEqual(mappedFolder.name, "Projects", "Folder name");
    assertEqual(mappedFolder.rightLabel, "Folder", "Right label is Folder");
    assertEqual(mappedFolder.primaryLabel, "Open Folder", "Primary label is Open Folder");
    assertEqual(mappedFolder.actions[0].label, "Open Folder", "Primary action is Open Folder");
  });

  // 2. Test empty query suggestions from backend response
  check(() => {
    const mockResponse: SearchResponse = {
      apps: [sampleApp],
      files: [],
    };
    const sections = buildSections("", mockResponse);
    assertEqual(sections[0].category, "app", "First section category");
    assertEqual(sections[0].title, "Suggestions", "First section title");
    assertEqual(sections[0].items[0].name, "Discord", "Suggested app name");
    assertEqual(sections.length, 1, "Only suggestions section when query is empty");
  });

  // 3. Test real query with search response
  check(() => {
    const mockResponse: SearchResponse = {
      apps: [sampleApp],
      files: [sampleImage, sampleDoc],
    };
    const sections = buildSections("disc", mockResponse);
    assertEqual(sections[0].category, "app", "Apps section category");
    assertEqual(sections[0].items[0].name, "Discord", "App match");
    assertEqual(sections[1].category, "file", "Files section category");
    assertEqual(sections[1].items.length, 2, "Files count");
    assertEqual(sections[1].items[0].name, "portfolio.png", "File 1 name");
  });

  // 4. Test math expression with search response
  check(() => {
    const mockResponse: SearchResponse = {
      apps: [sampleApp],
      files: [],
    };
    const sections = buildSections("15 * 4", mockResponse);
    assertEqual(sections[0].category, "calc", "Math section is first");
    assertEqual(sections[0].items[0].name, "60", "Calculated value");
    assertEqual(sections[0].items[0].primaryLabel, "Copy Result", "Calc primary label");
  });

  // 5. Test fallback section when no results match
  check(() => {
    const emptyResponse: SearchResponse = {
      apps: [],
      files: [],
    };
    const sections = buildSections("nonexistentquery12345", emptyResponse);
    assertEqual(sections[0].category, "fb", "Fallback category");
    assertEqual(hasOnlyFallbackSections(sections), true, "Has only fallback sections");
  });

  // 6. Test highlight split
  check(() => {
    const hl = splitHighlight("Visual Studio Code", "Studio");
    assertEqual(hl.before, "Visual ", "Highlight before");
    assertEqual(hl.match, "Studio", "Highlight match");
    assertEqual(hl.after, " Code", "Highlight after");
  });

  // 7. Test scoreItem ranking
  check(() => {
    const item = mapBackendApp(sampleApp);
    assertEqual(scoreItem(item, "disc") > 0, true, "Prefix match score > 0");
    assertEqual(scoreItem(item, "xyz"), 0, "No match score is 0");
  });

  // 8. Test enableFileSearch option
  check(() => {
    const mockResponse: SearchResponse = {
      apps: [sampleApp],
      files: [sampleDoc],
    };
    const sectionsWithFiles = buildSections("notes", mockResponse, { enableFileSearch: false });
    const hasFiles = sectionsWithFiles.some((s) => s.category === "file");
    assertEqual(hasFiles, false, "File section excluded when enableFileSearch is false");
  });

  // 9. Test search filter: apps only
  check(() => {
    const mockResponse: SearchResponse = {
      apps: [sampleApp],
      files: [sampleDoc],
    };
    const sections = buildSections("disc", mockResponse, { filter: "apps" });
    assertEqual(sections.length, 1, "Only apps section");
    assertEqual(sections[0].category, "app", "Apps section category");
    assertEqual(sections[0].items[0].name, "Discord", "App match");
  });

  // 10. Test search filter: files only
  check(() => {
    const mockResponse: SearchResponse = {
      apps: [sampleApp],
      files: [sampleDoc],
    };
    const sections = buildSections("notes", mockResponse, { filter: "files" });
    assertEqual(sections.length, 1, "Only files section");
    assertEqual(sections[0].category, "file", "Files section category");
    assertEqual(sections[0].items[0].name, "notes.txt", "File match");
  });

  // 11. Test search filter: actions only with query
  check(() => {
    const mockResponse: SearchResponse = {
      apps: [sampleApp],
      files: [sampleDoc],
    };
    const sections = buildSections("theme", mockResponse, { filter: "actions" });
    assertEqual(sections.length, 1, "Only actions section");
    assertEqual(sections[0].category, "cmd", "Actions category");
    assertEqual(sections[0].items[0].name, "Toggle Theme", "Command match");
  });

  // 12. Test search filter: actions only idle
  check(() => {
    const sections = buildSections("", undefined, { filter: "actions" });
    assertEqual(sections.length, 1, "Actions section present");
    assertEqual(sections[0].category, "cmd", "Actions category");
    assertEqual(sections[0].items.length > 0, true, "Available actions populated");
  });

  return passed;
}

const totalPassed = runSearchIntegrationTests();
console.log(`Search integration test suite passed (${totalPassed} assertions).`);
