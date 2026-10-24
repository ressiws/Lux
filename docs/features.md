# Lux — User Feature Guide

Welcome to **Lux**, a lightweight, fast, and privacy-first desktop launcher for Windows and Linux. Lux helps you search applications, browse local files, calculate practical problems, convert units, manage clipboard history, and access quick actions instantly with minimal keystrokes.

---

## 1. Getting Started

### Launching Lux
- **Global Hotkey**: Press <kbd>Alt</kbd> + <kbd>Space</kbd> (default) anywhere in your operating system to open Lux.
- **Dismissing Lux**: Press <kbd>Esc</kbd> or click anywhere outside the launcher window.
- **Focusing Search**: The search field is automatically focused whenever Lux appears.

### Launcher Interface
- **Minimal Idle Mode**: When idle with minimal density, Lux presents a sleek Spotlight-inspired search bar with the Lux icon, placeholder, category triggers, and settings button.
- **Active Search**: As soon as you type, results populate instantly beneath the search input without lag or jitter.
- **Preview Panel**: When enabled, inspecting items displays a preview panel on the right with file details, paths, application icons, and metadata.

---

## 2. Search and Applications

### Application Discovery
Lux indexes your installed system applications directly from Windows Start Menu directories, system paths, and standard locations.

- **Fast Launching**: Type the application name (e.g., `Firefox`, `Calculator`, `Settings`, `Terminal`) and press <kbd>Enter</kbd>.
- **Abbreviations**: Word-initial abbreviations work automatically, so `vsc` finds Visual Studio Code while exact names keep their higher ranking.
- **Result Types**: Results are labeled from their source metadata as `Application`, `System Setting`, `File`, or `Folder`, so settings pages such as Control Panel are not shown as applications.
- **Run as Administrator**: Highlight an application and press <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>Enter</kbd> (or select "Run as Administrator" from the Actions menu with <kbd>Ctrl</kbd> + <kbd>K</kbd>).
- **Open File Location**: Highlight an application and press <kbd>Ctrl</kbd> + <kbd>O</kbd> to reveal the shortcut or executable in Windows Explorer.
- **Flow Launcher Prefix Support**: If enabled in Settings, prefixes such as `>` or `et` can be configured or ignored according to your workflow preferences.

---

## 3. Files, Folders, and Categories

### Real File Search
Lux indexes your local user directories (Desktop, Documents, Downloads, Music, Pictures, Videos) and system drives using a local SQLite index.
- **No Mock or Fabricated Data**: Lux only presents genuine local files discovered on your machine.
- **Folders First**: Folder browsing presents directories at the top for intuitive file system traversal.
- **Incremental Loading**: Browsing large collections loads results smoothly in batches of 50 items as you scroll, preventing UI freezes.

### Category Filtering & Browse Mode
Beside the search input are three primary category triggers:
- **Apps**: Browse and filter indexed applications.
- **Files**: Browse files and folders across your system.
- **Actions**: Browse available system and launcher commands.

#### Back Navigation
When browsing a category:
- The three category icons hide to maximize search input focus.
- The active category icon sits on the left of the search bar and acts as an interactive **Back Button**.
- **Hovering** over the category icon displays a left-arrow (<svg>←</svg>).
- **Clicking** the icon or pressing <kbd>Esc</kbd> on an empty query returns to the default `Search Anything…` mode while preserving your search term.

---

## 4. Quick Answers

Lux provides instant answers as you type, eliminating the need to open a browser for everyday calculations, conversions, and references. All mathematical, date, time, unit, color, and password calculations execute entirely offline on your local machine.

### Calculations and Arithmetic
- **Basic Arithmetic**: `2 + 2`, `(128 * 1.19) / 3`, `17 % 5`
- **Powers & Roots**: `2 ^ 8`, `2 ** 10`, `sqrt(144)`, `cbrt(27)`
- **Trigonometry & Functions**: `sin(pi / 2)`, `cos(0)`, `log(1000)`, `ln(e)`, `abs(-42.5)`
- **Implicit Multiplication**: `2(3 + 4)`, `2pi`, `3sqrt(16)`
- **European Decimals**: Both period (`.`) and comma (`,`) decimal separators are supported: `12,5 * 2 = 25`.

### Percentages and Financial Math
- **Percentage of Number**: `15% of 240` → `36`
- **Adding VAT / Tax**: `VAT 23% on 100` or `100 + 23% VAT` → `123.00` (shows tax amount `23.00`)
- **Extracting VAT / Net Price**: `remove 23% VAT from 123` → `100.00` (shows tax included `23.00`)
- **Splitting Expenses**: `split 87.50 between 4` → `21.88 each`

### Time Zones & Time Differences
Lux automatically detects your operating system's configured time zone without requiring GPS or network permissions.
- **Current Time Worldwide**: `time in Tokyo`, `time in London`, `time New York`
- **Time Conversions**: `10 PM here to London`, `9:30 Lisbon to New York`
  - Handles day transitions crossing midnight with clear `(+1 day)` notes.
- **Time Difference**: `time difference Lisbon London`, `time diff New York Tokyo`
- **Supported Cities & IANA**: Recognizes major world cities (London, Lisbon, Tokyo, Paris, New York, San Francisco, Sydney, etc.) and all official IANA time zone identifiers (e.g., `Europe/London`, `America/New_York`).

### Dates & Durations
- **Future & Past Dates**: `90 days from today`, `2 weeks from now`, `30 days ago`
- **Holiday & Event Countdown**: `days until Christmas`, `days until new year`, `days until 25 Dec`
- **Duration Between Dates**: `days between 12 Oct and 21 Oct`
- **Deterministic Calendar Arithmetic**: Accurately accounts for leap years, variable month lengths, and calendar day boundaries.

### Unit Conversions
Lux rigorously differentiates decimal SI units from binary IEC storage units:
- **Digital Storage (SI vs IEC)**:
  - `1 GiB to MB` → `1,073.741824 MB` (Binary IEC to Decimal SI)
  - `1 GB to GiB` → `0.931323 GiB` (Decimal SI to Binary IEC)
  - Supports `bit`, `byte`, `KB`, `MB`, `GB`, `TB`, `KiB`, `MiB`, `GiB`, `TiB`.
- **Distance**: `100 km to miles`, `5 miles to km`, `6 feet to meters`, `10 inches to cm`
- **Temperature**: `100 C to F`, `32 F to C`, `0 C to K`
- **Weight**: `50 kg to lbs`, `150 lbs to kg`, `500 grams to oz`

### Color Conversions & Swatches
- **Hex to RGB / HSL**: `#798F78 to RGB` → `rgb(121, 143, 120)`, `#798F78 to HSL` → `hsl(117, 9%, 52%)`
- **RGB to Hex / HSL**: `rgb(121, 143, 120) to HEX` → `#798f78`
- **HSL to Hex / RGB**: `hsl(117, 9%, 52%) to HEX` → `#7b907a`
- **Visual Swatch**: Displays a live colored chip directly in the launcher row for quick visual confirmation.

### Secure Password Generation
- **Command**: `generate password 24` or `generate password` (default: 16 characters)
- **Local & Cryptographically Secure**: Uses `crypto.getRandomValues()` with an 86-character pool (uppercase, lowercase, numbers, and symbols).
- **Safety**: Passwords are never logged, persisted, or transmitted across a network. Pressing <kbd>Enter</kbd> copies the generated password directly to your clipboard.
- **False Positive Prevention**: Ordinary searches like `password manager` remain normal application/file searches.

---

## 5. Currency Conversion

Lux features built-in currency conversion supporting 160+ fiat currencies worldwide:
- **Example Queries**: `25 EUR to USD`, `100 USD in GBP`, `10,5 eur in jpy`
- **Exchange Rate Provider**: Exchange rates are retrieved from open reference data (European Central Bank via open.er-api.com). No API keys or paid subscriptions are required.
- **Smart Local Caching**: Rates are cached locally on your device. Lux **never** makes network requests on keystrokes.
- **Staleness Indication**: If offline or using cached data, Lux clearly indicates the cache age and timestamp.
- **Offline Reliability**: If your machine is disconnected from the internet, Lux uses your last saved rates without interruption.

---

## 6. Clipboard History

Lux includes a built-in clipboard manager accessible directly or via a dedicated shortcut:
- **Default Shortcut**: <kbd>Alt</kbd> + <kbd>Shift</kbd> + <kbd>V</kbd> (configurable in Settings).
- **Information Density Support**: Respects the global information density setting (`Minimal`, `Normal`, `Detailed`) to hide or display metadata, character counts, and timestamps.
- **Privacy Filters**: Includes built-in filters to ignore or hide sensitive items:
  - Secrets and API tokens (AWS, GitHub, Bearer tokens).
  - Credit card numbers.
  - Custom regular expressions with an interactive test runner in Settings.
- **Encrypted Local Storage**: Clipboard contents remain on your computer and are never synced externally.

---

## 7. Actions and Keyboard Shortcuts

### Global Shortcuts
| Shortcut | Action | Description |
|---|---|---|
| <kbd>Alt</kbd> + <kbd>Space</kbd> | Toggle Launcher | Opens or closes the Lux search bar. |
| <kbd>Alt</kbd> + <kbd>Shift</kbd> + <kbd>V</kbd> | Open Clipboard | Opens Clipboard History directly. |

### Launcher Navigation
| Key | Action | Description |
|---|---|---|
| <kbd>↑</kbd> / <kbd>↓</kbd> | Select Item | Navigate through search results. |
| <kbd>Enter</kbd> | Primary Action | Open app, open file, copy quick answer, or copy password. |
| <kbd>Ctrl</kbd> + <kbd>K</kbd> | Actions Menu | Open the secondary actions popover for the selected item. |
| <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>Enter</kbd> | Run as Admin | Launch selected application with administrator privileges. |
| <kbd>Ctrl</kbd> + <kbd>O</kbd> | Reveal in Explorer | Open parent folder in Windows File Explorer. |
| <kbd>Ctrl</kbd> + <kbd>C</kbd> | Copy Path / Value | Copy file path or primary item text. |
| <kbd>Ctrl</kbd> + <kbd>Shift</kbd> + <kbd>C</kbd> | Copy Expression | Copy quick answer calculation formula. |
| <kbd>Esc</kbd> | Close / Back | Close popover, return from category browse to all, or hide launcher. |

---

## 8. Customization and Settings

Access Settings by clicking the **gear icon** in the launcher or through the system tray icon.

### Settings Tabs
- **General**: Configure startup behavior, global hotkey (<kbd>Alt</kbd> + <kbd>Space</kbd>), and default search behavior.
- **Appearance**:
  - **Information Density**: Choose `Minimal` (essential info only), `Normal` (balanced), or `Detailed` (extra metadata).
  - **Layout Density**: Choose `Compact`, `Normal`, or `Comfortable` row padding.
  - **Theme**: `System`, `Light`, or `Dark`.
  - **Layout Mode**: Toggle between `List` and `Grid` view.
  - **Text Caret**: Enable or disable the blinking search cursor.
- **Search**: Toggle file and folder indexing, configure Flow Launcher prefix compatibility, and manage ignored prefixes.
- **Clipboard**: Enable/disable clipboard capture, configure clipboard hotkey, set ignore/hide filter modes, and manage custom regex privacy filters.
- **Features**: Toggle the side preview panel, indexing progress bar, and trigger manual index rebuilds.
- **Help & Shortcuts**: Interactive in-app cheat sheet of all query examples and keyboard shortcuts.
- **About**: View Lux version, build info, and architecture specifications.

---

## 9. Privacy and Network Requirements

Lux is designed from the ground up with local-first privacy:
- **Local Indexing**: File and application search indexes are stored locally in SQLite (`lux.db`). No search queries are ever transmitted over the network.
- **Offline By Default**: Calculations, time zones, dates, units, colors, and password generations are 100% offline.
- **Minimal Network Usage**: The only optional network feature is periodic exchange rate retrieval (once per 24 hours in the background). No personal identifiable information (PII), device telemetry, or query text is ever sent.
