# Contributing to Lux

Thank you for your interest in contributing to Lux!

Lux is a free and open-source, keyboard-first desktop launcher for Windows and Linux, built with Rust, Tauri 2, Svelte 5, SvelteKit, and TypeScript.

We welcome bug reports, feature suggestions, documentation improvements, performance optimizations, and code contributions.

Our goal is to keep Lux fast, reliable, private, maintainable, and free from unnecessary complexity.

## Table of Contents

- [Before Contributing](#before-contributing)
- [Ways to Contribute](#ways-to-contribute)
- [Reporting Bugs](#reporting-bugs)
- [Suggesting Features](#suggesting-features)
- [Development Setup](#development-setup)
- [Making Changes](#making-changes)
- [Code Guidelines](#code-guidelines)
- [Testing and Verification](#testing-and-verification)
- [Submitting a Pull Request](#submitting-a-pull-request)
- [Commit Messages](#commit-messages)
- [Security and Privacy](#security-and-privacy)
- [Questions](#questions)

## Before Contributing

Before starting work, please:

1. Read the project [README](README.md).
2. Check existing issues and pull requests to avoid duplicating work.
3. Review the relevant documentation in `docs/`.
4. Read [AGENTS.md](AGENTS.md) for project-specific development guidelines.
5. For significant changes, open an issue first to discuss the proposed approach.

Please avoid spending time implementing a large feature before confirming that it fits the project's direction.

## Ways to Contribute

You can contribute by:

- Reporting reproducible bugs.
- Suggesting useful, well-scoped features.
- Improving documentation.
- Fixing existing issues.
- Improving performance and resource efficiency.
- Improving accessibility and keyboard navigation.
- Adding meaningful automated tests.
- Identifying unnecessary complexity or maintenance overhead.

Contributions do not need to involve code. Clear bug reports and documentation improvements are valuable too.

## Reporting Bugs

Before opening a bug report, check whether the issue has already been reported.

Include the following information whenever possible:

- **Description:** What went wrong?
- **Expected behavior:** What should have happened?
- **Steps to reproduce:** How can the issue be reproduced?
- **Environment:** Operating system, version, and relevant system details.
- **Logs:** Relevant error messages or diagnostic output.
- **Screenshots or recordings:** When they help explain a visual or interaction issue.

Use a minimal, reproducible example whenever possible.

Remove personal information, credentials, private file paths, and other sensitive data from logs and screenshots before sharing them.

If the problem only occurs on Windows or Linux, explicitly identify the affected platform.

## Suggesting Features

Feature suggestions should address a real use case.

When proposing a feature, explain:

- The problem it solves.
- Why existing functionality is insufficient.
- How you expect the feature to behave.
- Whether a simpler solution could solve the problem.
- Any relevant performance, privacy, security, or cross-platform considerations.

Features should align with Lux's core principles:

- Fast and responsive interactions.
- Low idle resource usage.
- Keyboard-first operation.
- Local-first behavior and privacy by default.
- Consistent Windows and Linux support.
- Minimal dependencies and implementation complexity.

Avoid proposing features that add complexity without a clear, practical benefit.

Features involving network communication, telemetry, external services, or transmission of user data require explicit project authorization before implementation.

## Development Setup

### Prerequisites

Make sure you have the tools required by the project's current setup:

- Git.
- Rust and Cargo.
- Node.js and the package manager used by the repository.
- The system dependencies required by Tauri 2 for your operating system.

Refer to the official Tauri documentation for platform-specific prerequisites:

https://v2.tauri.app/start/prerequisites/

### Getting Started

1. Fork the repository on GitHub.
2. Clone your fork.
3. Create a branch for your changes.
4. Install dependencies using the package manager and instructions established by the repository.
5. Follow the README and project documentation to run the application.

Do not replace the existing package manager, build tooling, or project configuration as part of routine setup.

If the documented setup does not work, report the problem with the relevant operating system, tool versions, and error output.

## Making Changes

Keep contributions focused and easy to review.

Before implementing a change:

- Understand the existing implementation.
- Identify the files and components directly involved.
- Check existing conventions and architectural boundaries.
- Consider whether the problem can be solved without introducing new dependencies or abstractions.

During implementation:

- Preserve unrelated functionality.
- Avoid unrelated refactoring and formatting changes.
- Do not introduce speculative features or infrastructure.
- Do not modify unrelated configuration.
- Keep platform-specific behavior explicit where necessary.
- Update relevant documentation when behavior or development procedures change.

For substantial changes, discuss the proposed approach in an issue before opening a pull request.

## Code Guidelines

### General

- Prefer simple, readable, maintainable implementations.
- Follow existing project conventions.
- Avoid unnecessary abstractions, duplicated infrastructure, and dependencies.
- Do not introduce frameworks or architectural layers without a concrete need.
- Handle errors explicitly rather than hiding failures.
- Avoid unrelated changes in the same pull request.

### Rust

- Write idiomatic, safe Rust.
- Prefer explicit error handling and appropriate error propagation.
- Avoid unnecessary allocations, cloning, blocking operations, and locking.
- Do not use panics as normal control flow.
- Avoid `unsafe` unless there is a clearly justified requirement.
- Validate untrusted inputs and preserve security boundaries.

### Svelte and TypeScript

- Follow existing Svelte 5 conventions.
- Use strict, meaningful types and avoid `any`.
- Keep components focused on their responsibilities.
- Avoid unnecessary reactive indirection and global state.
- Preserve accessibility and predictable keyboard interaction.
- Handle relevant loading, empty, success, and error states.

### Frontend–Backend Boundaries

- Keep communication with Tauri commands consistent with the existing implementation.
- Validate inputs on the backend where required.
- Do not trust frontend-provided data.
- Do not expose privileged operations without appropriate validation and permissions.

### User Interface

- Follow the established design documentation in `docs/`.
- Reuse existing design tokens and components where appropriate.
- Preserve light and dark theme behavior.
- Keep keyboard navigation and accessibility in mind.
- Avoid introducing a competing design system.

## Testing and Verification

Run the checks relevant to your changes using the commands established by the repository.

Depending on the affected code, this may include:

- Linting.
- Type checking.
- Automated tests.
- Rust checks.
- Production builds.
- Manual testing of relevant application behavior.
- Platform-specific verification.

Add or update tests when they provide meaningful coverage, particularly for bug fixes, input validation, error handling, and security-sensitive behavior.

Before submitting a pull request:

1. Confirm that the intended functionality works.
2. Run the applicable checks.
3. Review the complete diff.
4. Remove temporary debugging code.
5. Check for accidental changes, generated files, and sensitive information.
6. Document any checks that could not be completed.

Do not report checks as passing unless they were actually run.

If a check fails, explain the failure and distinguish existing problems from regressions introduced by your changes.

When testing on both Windows and Linux is not possible, clearly state which platform was tested.

## Submitting a Pull Request

Before opening a pull request, ensure that:

- Your branch contains only changes relevant to the contribution.
- The implementation follows the project's guidelines.
- Applicable checks have been run.
- Relevant documentation and tests have been updated.
- No secrets, credentials, or unrelated generated files are included.
- The pull request description explains the change and its motivation.

A pull request should include:

**Summary**
- What changed?
- Why was the change necessary?

**Implementation**
- How was the problem addressed?
- Were any important design or architectural decisions made?

**Verification**
- Which tests and checks were run?
- Which operating systems were tested?
- Are there any known limitations?

**Related issues**
- Reference any relevant issues.

Include screenshots or recordings for meaningful user interface changes when practical.

Keep pull requests focused. Large changes should be discussed with maintainers before implementation.

Maintainers may request changes, additional tests, documentation, or a narrower scope before accepting a contribution.

Opening a pull request does not guarantee that the proposed change will be merged.

## Commit Messages

Use [Conventional Commits](https://www.conventionalcommits.org/) when preparing commits.

Examples:

- `feat: add application search`
- `fix: resolve keyboard navigation issue`
- `perf: reduce unnecessary filesystem scans`
- `docs: improve installation instructions`
- `test: add launcher search coverage`
- `refactor: simplify command handling`

Keep commit messages concise and describe the actual change.

## Security and Privacy

Security vulnerabilities must not be reported through public issues.

Please follow [SECURITY.md](SECURITY.md) for instructions on reporting security concerns privately.

Contributions must respect Lux's privacy requirements.

Do not introduce telemetry, analytics, external AI services, remote configuration, or other network communication that transmits application or user data without explicit authorization.

Do not include sensitive user information in source code, logs, screenshots, test fixtures, or documentation.

Never weaken security settings or permissions simply to make an implementation easier.

## Questions

For questions about contributing, use the repository's available discussion channels or open an issue when appropriate.

For bugs, feature proposals, and security concerns, follow the reporting guidance above.

Thank you for helping make Lux better.

Every contribution should move the project towards a launcher that is faster, simpler, more reliable, and respectful of user privacy.
