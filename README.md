<p align="center">
  <img src=".github/assets/banner.svg" alt="Lux banner" width="100%">
</p>

<p align="center">
  <img src="docs/images/logo-horizontal.svg" alt="Lux" width="240">
</p>

<p align="center">
  A faster, more focused way to work on your desktop.
</p>

<p align="center">
  A free, open-source, keyboard-first desktop launcher for Windows and Linux.
</p>

<p align="center">
  <a href="LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/License-MIT-blue.svg"></a>
  <img alt="Status: Early Development" src="https://img.shields.io/badge/status-early%20development-orange.svg">
  <img alt="Built with Tauri" src="https://img.shields.io/badge/Tauri-2-24C8DB.svg">
  <img alt="Built with Rust" src="https://img.shields.io/badge/Rust-backend-black.svg">
</p>

---

## Overview

Lux is a keyboard-first desktop launcher designed to make everyday desktop tasks faster and easier to access.

Inspired by tools such as Spotlight and Raycast, Lux aims to bring search, calculations, and useful actions into a compact interface that stays out of your way until you need it.

Built with Rust and Tauri, Lux combines native desktop integration with a modern frontend.

**Lux is under active development.** Features, interfaces, and platform behavior may change as the project evolves.

## Preview

<table>
  <tr>
    <td align="center" width="50%">
      <strong>Launcher</strong><br><br>
      <img src="docs/images/launcher.png" alt="Lux launcher screenshot" width="100%"><br>
      <sub>Search and access actions from a compact interface.</sub>
    </td>
    <td align="center" width="50%">
      <strong>Calculator</strong><br><br>
      <img src="docs/images/calculator.png" alt="Lux calculator screenshot" width="100%"><br>
      <sub>Evaluate mathematical expressions without leaving your workflow.</sub>
    </td>
  </tr>
</table>

## Current Features

The current implementation includes work on:

- **Keyboard-first launcher** — a compact interface designed for quick interaction.
- **Global shortcut** — show or hide the launcher using a configured keyboard shortcut.
- **Background operation** — keep the application available while working.
- **System tray integration** — access relevant application controls.
- **Built-in calculator** — evaluate mathematical expressions using a dedicated parser.
- **Quick actions** — access actions associated with selected results.
- **Windows integration** — native Acrylic backdrop support, with alternative behavior on other platforms.

Lux is still evolving. Some functionality and platform-specific behavior may be incomplete or subject to change.

## Design Principles

- **Fast** — prioritize responsive interactions and low idle resource usage.
- **Focused** — keep the interface compact and keyboard-driven.
- **Local-first** — favor local processing and user control.
- **Private by default** — avoid unnecessary data collection and unauthorized data transmission.
- **Maintainable** — prefer straightforward implementations over unnecessary complexity.
- **Cross-platform** — treat Windows and Linux as first-class targets.
- **Open source** — make the code available for inspection, use, and improvement.

## Privacy and Security

Privacy is a core design principle of Lux.

The project prioritizes local functionality, minimal data collection, and explicit control over operations that may expose user information.

Network functionality that transmits application or user data requires explicit project authorization. Security-sensitive functionality should follow the principle of least privilege and validate untrusted inputs.

For details, see [SECURITY.md](SECURITY.md).

## Technology Stack

| Area | Technology |
| --- | --- |
| Desktop framework | Tauri 2 |
| Backend | Rust |
| Frontend framework | Svelte 5 |
| Application framework | SvelteKit |
| Language | TypeScript |
| Package manager | pnpm |

## Getting Started

These instructions are intended for developers running Lux from source, not end users looking for a packaged installer.

### Prerequisites

Install the following tools:

- [Node.js](https://nodejs.org/)
- [pnpm](https://pnpm.io/)
- [Rust and Cargo](https://www.rust-lang.org/tools/install)
- The platform-specific dependencies required by [Tauri 2](https://v2.tauri.app/start/prerequisites/)

### Clone the Repository

```bash
git clone https://github.com/ressiws/Lux.git
cd Lux
```

### Install Dependencies

```bash
pnpm install
```

### Run in Development Mode

```bash
pnpm tauri dev
```

Lux is designed to remain available in the background. Use the configured global shortcut to open the launcher.

### Run Checks

```bash
pnpm check
pnpm build
```

Run the relevant Rust checks from the directory containing the applicable `Cargo.toml`:

```bash
cargo clippy --all-targets -- -D warnings
cargo test
```

Some checks may depend on your operating system and installed system libraries.

### Build for Distribution

Packaging and distribution procedures may evolve during early development. Consult the repository's current configuration and documentation before attempting to produce release packages.

## Roadmap

Planned areas of development include:

- Application discovery and launching.
- Additional search providers and quick actions.
- Extensible functionality.
- Improved Windows and Linux integration.
- Continued performance and resource-usage improvements.
- Accessibility and usability refinements.

These are development goals, not guarantees that the functionality is already implemented or scheduled for a particular release.

Check the [open issues](https://github.com/ressiws/Lux/issues) for ongoing work and proposed improvements.

## Contributing

Contributions are welcome, including bug reports, feature proposals, documentation improvements, and code changes.

Before making substantial changes, please discuss the proposed approach with the maintainers.

- [Contributing Guide](CONTRIBUTING.md) — development setup, coding guidelines, and pull requests.
- [Issues Guide](ISSUES.md) — reporting bugs and proposing features.
- [Security Policy](SECURITY.md) — privately reporting security vulnerabilities.
- [Agent Guidelines](AGENTS.md) — development rules for AI coding agents.

Please keep contributions focused, test relevant changes, and document verification results accurately.

## License

Lux is distributed under the [MIT License](LICENSE).

The MIT License permits use, modification, distribution, and commercial use, provided that the applicable copyright and license notices are preserved.

## Acknowledgements

Lux is an independent open-source project inspired by the broader category of desktop launchers, including Spotlight and Raycast.

Lux is not affiliated with, endorsed by, or sponsored by Apple or Raycast.
