import { findInlineCompletion } from "./autocomplete";
import {
  MOCK_ITEMS,
  buildSections,
  getCategoryPlaceholder,
  getItemMetadata,
  hasOnlyFallbackSections,
  mapBackendApp,
  mapBackendFile,
  scoreItem,
  splitHighlight,
} from "./mock-data";
import type { IndexStats, SearchResponse, SearchResultItem } from "./types";

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

  // 1b. Test result preview metadata
  check(() => {
    const appMeta = getItemMetadata(mapBackendApp(sampleApp));
    assertEqual(appMeta[0][1], "Application", "App preview kind");
    assertEqual(appMeta[1][1], sampleApp.location, "App preview location");
    assertEqual(appMeta[2][1], "1.0.9000", "App preview version");
    assertEqual(appMeta[3][1], "120 MB", "App preview size");

    const imageMeta = getItemMetadata(mapBackendFile(sampleImage));
    assertEqual(imageMeta[0][1], "Image", "Image preview kind");
    assertEqual(imageMeta[1][1], sampleImage.location, "Image preview location");
    assertEqual(imageMeta[2][1], "2.4 MB", "Image preview size");

    const docMeta = getItemMetadata(mapBackendFile(sampleDoc));
    assertEqual(docMeta[0][1], "TXT", "Document preview kind");

    const folderMeta = getItemMetadata(mapBackendFile(sampleFolder));
    assertEqual(folderMeta[0][1], "Folder", "Folder preview kind");
    assertEqual(folderMeta[1][1], "D:\\Projects", "Folder preview path");
    assertEqual(folderMeta[2][1], "—", "Missing folder size uses fallback");
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

  // 13. Test dynamic category placeholders
  check(() => {
    assertEqual(getCategoryPlaceholder("all"), "Search Anything…", "Default category placeholder");
    assertEqual(getCategoryPlaceholder("apps"), "Search Applications…", "Apps category placeholder");
    assertEqual(getCategoryPlaceholder("files"), "Search Files…", "Files category placeholder");
    assertEqual(getCategoryPlaceholder("actions"), "Search Actions…", "Actions category placeholder");
  });

  // 14. Test browse mode with empty query
  check(() => {
    // Apps browse mode returns all available apps when testing
    const appSections = buildSections("", undefined, {
      filter: "apps",
      mockItemsForTesting: MOCK_ITEMS,
    });
    assertEqual(appSections.length, 1, "Apps browse section present");
    assertEqual(appSections[0].category, "app", "Apps category");
    assertEqual(appSections[0].items.length > 0, true, "Apps populated in browse mode");

    // Files browse mode returns all available files when testing
    const fileSections = buildSections("", undefined, {
      filter: "files",
      mockItemsForTesting: MOCK_ITEMS,
    });
    assertEqual(fileSections.length, 1, "Files browse section present");
    assertEqual(fileSections[0].category, "file", "Files category");
    assertEqual(fileSections[0].items.length > 0, true, "Files populated in browse mode");
  });

  // 15. Test subcategory filtering in browse mode
  check(() => {
    // Filter apps by devtools
    const devtoolsSections = buildSections("", undefined, {
      filter: "apps",
      subcategory: "devtools",
      mockItemsForTesting: MOCK_ITEMS,
    });
    assertEqual(devtoolsSections.length, 1, "Devtools section present");
    assertEqual(
      devtoolsSections[0].items.every((item) =>
        item.name.includes("Visual Studio") || item.name.includes("Terminal"),
      ),
      true,
      "Devtools only contains developer tools",
    );

    // Filter files by folders
    const folderSections = buildSections("", undefined, {
      filter: "files",
      subcategory: "folders",
      mockItemsForTesting: MOCK_ITEMS,
    });
    assertEqual(folderSections.length, 1, "Folders section present");
    assertEqual(
      folderSections[0].items.every((item) => item.rightLabel === "Folder"),
      true,
      "Folders subcategory contains only folders",
    );

    // Filter actions by preferences
    const prefSections = buildSections("", undefined, {
      filter: "actions",
      subcategory: "preferences",
    });
    assertEqual(prefSections.length, 1, "Preferences actions present");
    assertEqual(
      prefSections[0].items.every(
        (item) => item.name.includes("Theme") || item.name.includes("Settings"),
      ),
      true,
      "Preferences only contains theme and settings",
    );
  });

  // 16. Test browse mode with query and subcategory combined
  check(() => {
    const sections = buildSections("visual", undefined, {
      filter: "apps",
      subcategory: "devtools",
      mockItemsForTesting: MOCK_ITEMS,
    });
    assertEqual(sections.length, 1, "Search within subcategory section present");
    assertEqual(sections[0].items[0].name, "Visual Studio Code", "Matched Visual Studio Code");

    // Searching for an app that doesn't belong to the selected subcategory returns empty
    const noMatch = buildSections("spotify", undefined, {
      filter: "apps",
      subcategory: "devtools",
      mockItemsForTesting: MOCK_ITEMS,
    });
    assertEqual(noMatch.length, 0, "Spotify not in devtools");
  });

  // 17. Test switching primary categories preserves query
  check(() => {
    const query = "wedding";
    const appResult = buildSections(query, undefined, {
      filter: "apps",
      mockItemsForTesting: MOCK_ITEMS,
    });
    assertEqual(appResult.length, 0, "No wedding apps");

    const fileResult = buildSections(query, undefined, {
      filter: "files",
      mockItemsForTesting: MOCK_ITEMS,
    });
    assertEqual(fileResult.length, 1, "Wedding files found");
    assertEqual(fileResult[0].items.length > 0, true, "Files matched query");
  });

  // 18. Test empty state behavior when file search disabled
  check(() => {
    const disabledFileSearch = buildSections("", undefined, {
      filter: "files",
      enableFileSearch: false,
      mockItemsForTesting: MOCK_ITEMS,
    });
    assertEqual(disabledFileSearch.length, 0, "Empty when file search disabled");
  });

  // 19. Verify no mock data in production search or browse when backend response is absent
  check(() => {
    // Empty backend response returns 0 items in production, never fictional mock items
    const prodAppSections = buildSections("", undefined, { filter: "apps" });
    assertEqual(prodAppSections.length, 0, "No mock apps in production browse");

    const prodFileSections = buildSections("", undefined, { filter: "files" });
    assertEqual(prodFileSections.length, 0, "No mock files in production browse");

    const prodSearchSections = buildSections("anything", undefined, { filter: "all" });
    assertEqual(
      prodSearchSections.every((s) => s.category !== "app" && s.category !== "file"),
      true,
      "No mock apps or files in production search when backend response is absent",
    );
  });
  // 20. Test deduplication of backend apps and files
  check(() => {
    const duplicateResponse: SearchResponse = {
      apps: [sampleApp, { ...sampleApp }, { ...sampleApp, name: "Discord" }],
      files: [sampleDoc, { ...sampleDoc }, sampleDoc],
    };

    const sections = buildSections("discord", duplicateResponse, { filter: "all" });
    const appSection = sections.find((s) => s.category === "app");
    assertEqual(appSection?.items.length, 1, "Duplicate apps must be deduplicated to 1");

    const fileSections = buildSections("notes", duplicateResponse, { filter: "files" });
    assertEqual(fileSections[0]?.items.length, 1, "Duplicate files must be deduplicated to 1");

    // Multiple distinct files are preserved without false deduplication
    const multiResponse: SearchResponse = {
      apps: [sampleApp],
      files: [sampleDoc, { ...sampleDoc }, sampleImage, { ...sampleImage }],
    };
    const multiSections = buildSections("", multiResponse, { filter: "files" });
    assertEqual(multiSections[0]?.items.length, 2, "Distinct files are preserved (doc and image)");
  });

  // 21. Test IndexStats lifecycle contract
  check(() => {
    const stats: IndexStats = {
      totalApps: 42,
      totalFiles: 100,
      isIndexed: true,
      isIndexing: false,
    };
    assertEqual(stats.isIndexed, true, "IndexStats isIndexed flag");
    assertEqual(stats.isIndexing, false, "IndexStats isIndexing flag");
    assertEqual(stats.totalApps, 42, "IndexStats totalApps");
    assertEqual(stats.totalFiles, 100, "IndexStats totalFiles");
  });

  // 22. Test fallback web search section with default browser metadata
  check(() => {
    const emptyResponse: SearchResponse = { apps: [], files: [] };
    const sections = buildSections("unmatched query term", emptyResponse, {
      filter: "all",
      defaultBrowser: {
        name: "Google Chrome",
        icon: "data:image/png;base64,chromeicon",
      },
    });

    assertEqual(sections.length, 1, "Fallback section present when no items match");
    assertEqual(sections[0].category, "fb", "Section category is fb");
    assertEqual(sections[0].items.length, 2, "Fallback section has 2 items");

    const webItem = sections[0].items[0];
    assertEqual(webItem.name, "Search the web for “unmatched query term”", "Web item name");
    assertEqual(webItem.subtitle, "Google Chrome", "Web item subtitle is browser name");
    assertEqual(webItem.icon, "data:image/png;base64,chromeicon", "Web item icon");
    assertEqual(webItem.primaryLabel, "Search the Web", "Web item primary label");

    const fileItem = sections[0].items[1];
    assertEqual(fileItem.name, "Search files named “unmatched query term”", "File item name");
    assertEqual(fileItem.primaryLabel, "Search Files", "File item primary label");
  });

  // 23. Test web search URL encoding
  check(() => {
    const query = "rust & svelte 5? + tauri/2.0";
    const encoded = encodeURIComponent(query.trim());
    const targetUrl = `https://www.google.com/search?q=${encoded}`;
    assertEqual(
      targetUrl,
      "https://www.google.com/search?q=rust%20%26%20svelte%205%3F%20%2B%20tauri%2F2.0",
      "URL encoding properly escapes special characters and spaces"
    );
  });

  // 24. Test Escape key transition contracts
  check(() => {
    // Contract: when in category / subcategory mode, Escape returns to Search Anything preserving query
    type LauncherState = {
      activeFilter: string;
      activeSubcategory: string;
      actionsOpen: boolean;
      query: string;
      dismissed: boolean;
    };

    function simulateEscape(state: LauncherState): LauncherState {
      const next = { ...state };
      if (next.actionsOpen) {
        next.actionsOpen = false;
        return next;
      }
      if (next.activeFilter !== "all" || next.activeSubcategory !== "all") {
        next.activeFilter = "all";
        next.activeSubcategory = "all";
        // query is preserved, not dismissed
        return next;
      }
      next.dismissed = true;
      return next;
    }

    // Case A: Actions open
    const stA = simulateEscape({
      activeFilter: "apps",
      activeSubcategory: "all",
      actionsOpen: true,
      query: "git",
      dismissed: false,
    });
    assertEqual(stA.actionsOpen, false, "Actions closed");
    assertEqual(stA.activeFilter, "apps", "Filter preserved");
    assertEqual(stA.dismissed, false, "Not dismissed");

    // Case B: In Category mode
    const stB = simulateEscape({
      activeFilter: "files",
      activeSubcategory: "all",
      actionsOpen: false,
      query: "report",
      dismissed: false,
    });
    assertEqual(stB.activeFilter, "all", "Returned to all categories");
    assertEqual(stB.query, "report", "Query preserved");
    assertEqual(stB.dismissed, false, "Not dismissed");

    // Case C: In Subcategory mode
    const stC = simulateEscape({
      activeFilter: "apps",
      activeSubcategory: "dev",
      actionsOpen: false,
      query: "code",
      dismissed: false,
    });
    assertEqual(stC.activeFilter, "all", "Returned to all categories from subcategory");
    assertEqual(stC.activeSubcategory, "all", "Subcategory cleared");
    assertEqual(stC.query, "code", "Query preserved");
    assertEqual(stC.dismissed, false, "Not dismissed");

    // Case D: In Search Anything root
    const stD = simulateEscape({
      activeFilter: "all",
      activeSubcategory: "all",
      actionsOpen: false,
      query: "something",
      dismissed: false,
    });
    assertEqual(stD.dismissed, true, "Launcher dismissed in Search Anything root");
  });

  // 25. Test Files and Folders Browse & Subcategory filtering
  check(() => {
    const mockBrowseResponse: SearchResponse = {
      apps: [],
      files: [sampleFolder, sampleDoc, sampleImage],
    };

    // A: Browse mode - All subcategories
    const allSections = buildSections("", undefined, {
      filter: "files",
      subcategory: "all",
      browseResponse: mockBrowseResponse,
      enableFileSearch: true,
    });
    assertEqual(allSections.length, 1, "Browse mode files section present");
    assertEqual(allSections[0].items.length, 3, "Browse mode contains folder, doc, image");

    // B: Browse mode - Folders subcategory only
    const folderSections = buildSections("", undefined, {
      filter: "files",
      subcategory: "folders",
      browseResponse: mockBrowseResponse,
      enableFileSearch: true,
    });
    assertEqual(folderSections.length, 1, "Folders subcategory section present");
    assertEqual(folderSections[0].items.length, 1, "Only folder item returned");
    assertEqual(folderSections[0].items[0].name, "Projects", "Folder name matches");
    assertEqual(folderSections[0].items[0].rightLabel, "Folder", "Right label is Folder");

    // C: Browse mode - Images subcategory only
    const imageSections = buildSections("", undefined, {
      filter: "files",
      subcategory: "images",
      browseResponse: mockBrowseResponse,
      enableFileSearch: true,
    });
    assertEqual(imageSections.length, 1, "Images subcategory section present");
    assertEqual(imageSections[0].items.length, 1, "Only image item returned");
    assertEqual(imageSections[0].items[0].name, "portfolio.png", "Image name matches");

    // D: Query mode - Folders subcategory filtering
    const searchFolderResp: SearchResponse = {
      apps: [],
      files: [sampleFolder, sampleDoc],
    };
    const queryFolderSections = buildSections("pro", searchFolderResp, {
      filter: "files",
      subcategory: "folders",
      enableFileSearch: true,
    });
    assertEqual(queryFolderSections.length, 1, "Query folder section present");
    assertEqual(queryFolderSections[0].items.length, 1, "Only folder item matching query");
    assertEqual(queryFolderSections[0].items[0].name, "Projects", "Projects folder matches");
  });

  // 26. Inline autocomplete
  check(() => {
    const items = [
      mapBackendApp(sampleApp),
      mapBackendFile(sampleDoc),
      mapBackendFile(sampleFolder),
    ];

    assertEqual(
      findInlineCompletion("dis", items)?.completion,
      "cord",
      "app prefix completes",
    );
    assertEqual(
      findInlineCompletion("DISC", items)?.completion,
      "ord",
      "completion is case-insensitive",
    );
    assertEqual(
      findInlineCompletion("pro", items)?.completion,
      "jects",
      "folder prefix completes",
    );
    assertEqual(
      findInlineCompletion("discord", items),
      undefined,
      "exact names have no completion",
    );
    assertEqual(
      findInlineCompletion("dis ", items),
      undefined,
      "trailing whitespace disables completion",
    );
    assertEqual(
      findInlineCompletion("zzz", items),
      undefined,
      "missing prefixes have no completion",
    );
  });

    // 27. Test math query suppresses apps/files/commands sections
    check(() => {
      const mockResponse: SearchResponse = {
        apps: [sampleApp],
        files: [sampleDoc],
      };
      const sections = buildSections("15 * 4", mockResponse, { filter: "all" });
      assertEqual(sections.length, 1, "Only calc section for math query");
      assertEqual(sections[0].category, "calc", "First section is calc");
      assertEqual(sections[0].items[0].name, "60", "Calculated value");
      const hasApps = sections.some((s) => s.category === "app");
      const hasFiles = sections.some((s) => s.category === "file");
      const hasCmds = sections.some((s) => s.category === "cmd");
      assertEqual(hasApps, false, "No apps section for math query");
      assertEqual(hasFiles, false, "No files section for math query");
      assertEqual(hasCmds, false, "No commands section for math query");
    });

    // 28. Test math query with test items also suppresses other sections
    check(() => {
      const sections = buildSections("2 + 2", undefined, {
        filter: "all",
        mockItemsForTesting: MOCK_ITEMS,
      });
      assertEqual(sections.length, 1, "Only calc section for math query with test items");
      assertEqual(sections[0].category, "calc", "First section is calc");
    });

    // 29. Test non-math query still shows all sections
    check(() => {
      const mockResponse: SearchResponse = {
        apps: [sampleApp],
        files: [sampleDoc],
      };
      const sections = buildSections("discord", mockResponse, { filter: "all" });
      const hasCalc = sections.some((s) => s.category === "calc");
      const hasApps = sections.some((s) => s.category === "app");
      const hasFiles = sections.some((s) => s.category === "file");
      assertEqual(hasCalc, false, "No calc section for non-math query");
      assertEqual(hasApps, true, "Apps section present for non-math query");
      assertEqual(hasFiles, true, "Files section present for non-math query");
    });

    // 30. Test inline autocomplete function works (setting checked in UI layer)
    check(() => {
      const items = [
        mapBackendApp(sampleApp),
        mapBackendFile(sampleDoc),
      ];
      const completion = findInlineCompletion("dis", items);
      assertEqual(completion?.completion, "cord", "autocomplete function works");
    });

  return passed;
}

const totalPassed = runSearchIntegrationTests();
console.log(`Search integration test suite passed (${totalPassed} assertions).`);
