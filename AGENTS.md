# AGENTS.md — Lux

## 1. Project Overview

Lux is a free and open-source, keyboard-first desktop launcher designed for Windows and Linux.

Its primary goals are:

- Extremely low idle resource usage and fast response times.
- Reliable, predictable behavior.
- A clean, minimal, customizable interface.
- Privacy by default and local-first operation.
- Cross-platform compatibility.
- Maintainable, idiomatic code.
- No unnecessary complexity, dependencies, or abstractions.

### Technology Stack

- **Backend:** Rust.
- **Desktop runtime:** Tauri 2.
- **Frontend:** Svelte 5, SvelteKit, and TypeScript.
- **Build tooling:** Use the tooling already established in the repository.
- **Persistence:** Use existing project decisions and introduce storage only when required.

Do not replace the established stack or introduce competing frameworks without explicit approval.

## 2. Mandatory Rules

These rules apply to every task.

1. Read this file and the relevant project documentation before making changes.
2. Inspect the existing implementation before editing it.
3. Preserve working behavior unless the requested task requires changing it.
4. Prefer the smallest correct change.
5. Do not implement architectural changes without explicit approval.
6. Do not modify `main` directly.
7. Never commit, push, merge, or rebase without explicit approval.
8. Never hide failed tests, warnings relevant to the task, or verification limitations.
9. Never introduce network data transmission without explicit authorization.
10. Never add unnecessary abstractions, dependencies, configuration changes, or global state.
11. Do not claim that a task is complete until the applicable verification has been performed.
12. If a requested change conflicts with these rules, stop and request clarification or approval.

## 3. Autonomy and Scope

Operate with controlled autonomy.

You may independently:

- Implement the requested functionality.
- Choose straightforward implementation details within the existing architecture.
- Fix bugs directly related to the requested change.
- Refactor a small amount of code when necessary to implement the change safely.
- Run normal development commands and relevant verification.
- Create a dedicated Git branch when needed.

You must request explicit approval before:

- Changing the architecture or established architectural boundaries.
- Expanding the task beyond directly related work.
- Changing protected configuration files.
- Introducing significant dependencies or replacing existing dependencies.
- Changing existing behavior outside the requested scope.
- Performing destructive, privileged, irreversible, or external operations.
- Introducing new network communication or data transmission.
- Making commits, pushes, merges, or rebases.

Do not interpret a request to implement a feature as permission to redesign the application.

When a small implementation detail is ambiguous, choose the simplest safe interpretation. When ambiguity affects architecture, security, privacy, compatibility, or existing behavior, stop and ask.

## 4. Architecture and Design Principles

### 4.1 Keep It Simple

- Prefer the simplest implementation that correctly satisfies the requirements.
- Avoid speculative features and infrastructure.
- Do not build frameworks for a single use case.
- Do not introduce interfaces, factories, registries, managers, providers, or service layers without a concrete justification.
- Avoid both duplicated logic and abstractions that are more complicated than the duplication they remove.
- Do not refactor unrelated code merely to make it conform to a preferred style.
- Keep responsibilities clear and modules reasonably small.

### 4.2 Project Structure

Use a pragmatic modular structure.

Keep frontend code, Rust backend code, shared components, and feature-specific logic separated according to actual responsibilities.

Create new modules or directories when they improve ownership, clarity, or testability. Do not create empty architectural layers or directories solely to follow a pattern.

Avoid source files containing hundreds of lines. Split files when their responsibilities become difficult to understand or maintain. Do not split code mechanically into excessive tiny files.

### 4.3 Architecture Changes

You may identify and propose architectural improvements.

Do not implement changes to established architecture, module boundaries, communication contracts, or application-wide patterns without explicit approval.

Prefer localized changes that preserve existing contracts.

## 5. Rust Rules

Write idiomatic, safe, maintainable Rust.

- Avoid `unwrap()` and `expect()` on normal execution paths.
- Handle recoverable errors explicitly.
- Never silently discard errors or convert failures into misleading success states.
- Use appropriate error types and clear error propagation.
- Avoid unnecessary cloning, allocations, locking, and expensive work.
- Keep ownership and lifetimes straightforward.
- Avoid `unsafe` unless there is a concrete, documented need and explicit approval.
- Do not introduce unsafe optimizations without evidence that they are necessary.
- Do not use panics as normal control flow.
- Avoid blocking operations on asynchronous execution paths.
- Keep platform-specific code isolated where appropriate.
- Validate data crossing trust boundaries.
- Avoid exposing internal implementation details or sensitive data in errors returned to the frontend.

Do not sacrifice readability for speculative performance gains.

## 6. Svelte 5, SvelteKit, and TypeScript Rules

Write simple, strictly typed TypeScript and maintainable Svelte 5 components.

- Never use `any`.
- Never bypass type errors using unjustified type assertions or suppression directives.
- Do not use `@ts-ignore` to conceal errors.
- If a suppression is genuinely necessary, document why and keep its scope minimal.
- Prefer inferred types when they are clear and explicit types when they improve contracts.
- Avoid oversized components with unrelated responsibilities.
- Keep component state local unless shared state is genuinely required.
- Do not introduce global state management libraries without explicit approval.
- Avoid unnecessary effects, derived state, reactive indirection, and helper abstractions.
- Extract reusable components and modules only when reuse or separation of responsibility justifies them.
- Keep UI logic readable and avoid excessive indirection.
- Handle loading, empty, success, and error states where applicable.
- Ensure interactive elements behave consistently and remain accessible.

### Naming

- Use `camelCase` for variables, functions, and ordinary TypeScript identifiers.
- Follow the established conventions for Svelte components, SvelteKit routes, types, and constants.
- Use descriptive names rather than excessive comments.

## 7. Frontend–Backend Communication

Use a small, typed frontend layer to centralize communication with Tauri commands.

- Avoid scattering raw Tauri invocations throughout unrelated components.
- Define clear request and response contracts.
- Validate inputs at relevant trust boundaries.
- Treat frontend-provided data as untrusted.
- Handle backend errors explicitly and present useful, user-friendly messages.
- Keep business logic out of presentation components when separation provides a real benefit.
- Do not introduce a full services-and-repositories architecture without a demonstrated need.

Never expose privileged backend functionality to the frontend without an appropriate permission and validation boundary.

## 8. Error Handling and Logging

Use structured, meaningful error handling.

- Prefer typed Rust errors and explicit propagation.
- Keep internal diagnostic information separate from user-facing messages.
- Log useful operational information without secrets, credentials, personal data, or sensitive file contents.
- Never log entire clipboard entries, private file contents, authentication material, or other sensitive data unnecessarily.
- Avoid swallowing errors, empty catch blocks, and success fallbacks that conceal failure.
- Do not add extensive observability infrastructure without justification.

Errors should be actionable without exposing sensitive implementation details.

## 9. Security and Privacy

Privacy and security are mandatory requirements, not optional enhancements.

### 9.1 Network Access

**No functionality may transmit data over the Internet without explicit authorization.**

This applies to:

- Telemetry and analytics.
- Crash reporting and diagnostics.
- Update checks that transmit application or user information.
- Cloud AI and external AI providers.
- Plugins and third-party integrations.
- Remote configuration and synchronization.
- Usage statistics.
- Background requests.
- Any other functionality that sends application or user data externally.

Do not assume that a feature being optional, disabled by default, or provided by a third-party library grants permission to transmit data.

Before implementing a feature that requires network communication, explain:

1. Why network access is necessary.
2. What data would be transmitted.
3. Which destination would receive it.
4. Whether a local alternative is possible.

Wait for explicit approval before implementing the transmission.

Do not add hidden network activity or enable network behavior indirectly through a dependency.

### 9.2 Local Data

- Minimize the collection and storage of user data.
- Store data locally when practical.
- Do not persist information without a functional reason.
- Respect deletion and retention requirements established by the project.
- Protect sensitive local data where appropriate.
- Do not assume local data is safe merely because it never leaves the machine.

### 9.3 Inputs and Permissions

- Validate untrusted inputs.
- Use safe filesystem operations.
- Avoid path traversal and unintended file access.
- Apply least privilege.
- Prefer Tauri's security mechanisms and permission model.
- Justify native commands, subprocesses, and additional permissions.
- Never execute arbitrary commands or code from untrusted inputs.
- Do not weaken security settings to make a feature work more easily.

### 9.4 Secrets

- Never commit secrets or credentials.
- Never include secrets in logs, errors, test fixtures, or generated documentation.
- Do not introduce unnecessary credential storage.
- Never claim a secret has been secured without verifying the relevant implementation.

## 10. Plugin System

The plugin system must remain pragmatic and permission-aware.

- Define clear interfaces when actual plugin functionality requires them.
- Keep plugin capabilities limited to what they need.
- Do not introduce a complex plugin framework before real use cases justify it.
- Do not grant plugins unrestricted access to files, processes, network communication, or application internals by default.
- Validate plugin inputs and outputs.
- Treat plugin-provided data as untrusted.
- Do not assume that third-party plugins are safe.
- Require explicit authorization before introducing plugin capabilities that transmit data externally.

Do not claim that plugins are sandboxed or isolated unless that protection actually exists and has been verified.

## 11. Windows and Linux Compatibility

Windows and Linux are first-class targets.

- Consider both platforms when designing new functionality.
- Avoid unnecessary operating-system-specific dependencies.
- Isolate platform-specific behavior when doing so improves maintainability.
- Prefer portable APIs when practical.
- Do not assume paths, filesystem behavior, process management, permissions, shortcuts, or application discovery behave identically across platforms.
- Handle unsupported platform capabilities explicitly.
- Do not silently substitute a different behavior on one platform.
- Test both platforms when the environment allows it.
- Clearly report platform-specific limitations and any untested behavior.

Do not claim cross-platform compatibility based only on a successful build on one operating system.

Do not introduce elaborate compatibility layers unless concrete differences require them.

## 12. Performance and Resource Usage

Low resource usage and responsiveness are core Lux requirements.

Prioritize:

- Fast startup and command execution.
- Low idle CPU usage.
- Low idle RAM usage.
- Minimal unnecessary background work.
- Efficient filesystem and application searches.
- Responsive keyboard interactions.
- Predictable resource cleanup.
- Avoiding unnecessary allocations, repeated computation, and excessive state updates.

However:

- Prefer measurement over speculation.
- Optimize demonstrated bottlenecks.
- Avoid premature micro-optimizations.
- Do not introduce complex caching, concurrency, or custom infrastructure without a measurable benefit.
- Do not trade correctness, security, or maintainability for insignificant performance gains.
- Consider performance implications when introducing polling, watchers, indexing, background tasks, or persistent processes.

When performance is relevant, describe what was measured and distinguish measured results from estimates.

## 13. UI and Design System

Follow the existing design documentation in `../docs/`.

Before changing UI behavior or styling, consult the relevant documentation, including applicable files covering:

- Colors.
- Typography.
- Components.
- Layout.
- Interactions.
- Accessibility.
- Customization.
- Theme scheduling.

Rules:

- Preserve visual consistency.
- Respect the established neutral visual direction and light/dark theme behavior.
- Support customization through the existing intended mechanisms.
- Avoid arbitrary colors, typography, spacing, or component variants when established tokens or rules apply.
- Do not introduce a competing design system.
- Keep keyboard interaction predictable.
- Respect accessibility requirements.
- Account for relevant loading, empty, error, focus, and disabled states.

Validate visual changes when practical. Do not claim visual verification if the interface was not actually inspected.

## 14. Configuration and Dependencies

### 14.1 Protected Configuration

Do not modify configuration files without explicit approval when the change affects build behavior, dependencies, linting, CI, tooling, or established development conventions.

This includes, where applicable:

- `package.json` and package-manager lockfiles.
- `Cargo.toml` and `Cargo.lock`.
- Tauri configuration.
- TypeScript configuration.
- Lint and formatting configuration.
- Build configuration.
- CI workflows.
- Toolchain and project-wide settings.

Functional configuration changes that alter existing behavior also require approval.

Do not update configuration merely to accommodate a preferred coding style or tool.

### 14.2 Dependencies

Before introducing a dependency, evaluate whether existing functionality can solve the problem adequately.

Prefer dependencies that are:

- Small and focused.
- Actively maintained.
- Compatible with the project's licenses.
- Appropriate for both target platforms where relevant.
- Justified by a concrete use case.

Small, justified dependencies may be added within the agreed scope. Important dependency upgrades, replacements, or changes with significant security, compatibility, licensing, or architectural impact require explicit approval.

Do not add multiple packages for overlapping functionality.

Never assume a dependency's license or behavior. Verify relevant details before relying on them.

## 15. Testing and Verification

Testing must be proportional to the risk and impact of a change.

Add or update tests for:

- Important business logic.
- Error handling.
- Regression fixes.
- Input validation.
- Platform-specific behavior where practical.
- Security-sensitive behavior.
- Other functionality where testing provides meaningful confidence.

Avoid redundant tests that merely duplicate implementation details.

### Required Verification

Before reporting completion, run the applicable checks available in the repository:

1. Relevant lint checks.
2. Type checking.
3. Relevant automated tests.
4. Applicable build checks.
5. Additional checks required by the changed functionality.

For Rust and frontend changes, select the relevant checks for each affected area.

Do not change project configuration or add dependencies simply to make a verification command pass without authorization.

If a check cannot run:

- State which check was not run.
- Explain the reason.
- Report any alternative verification performed.

If a check fails:

- Report the failure honestly.
- Investigate and fix it when it is within scope.
- Do not conceal failures or claim success based on unrelated passing checks.

Do not claim a platform has been tested unless it actually has been tested on that platform.

## 16. Documentation

Keep the project's documentation in `../docs/` synchronized with meaningful implementation changes.

Update relevant documentation when a change affects:

- Requirements or planned functionality.
- User-visible behavior.
- Architecture or module responsibilities.
- Security or privacy behavior.
- Configuration or customization.
- Platform compatibility.
- Development or verification procedures.

Avoid duplicating entire documents or copying extensive reference material into `AGENTS.md`.

Use clear English for project documentation.

If documentation conflicts with the implementation or another requirement, identify the conflict and ask for clarification when it affects the correct implementation.

Do not silently rewrite the project's goals or established decisions.

## 17. Git Workflow

Never modify `main` directly.

### 17.1 Branches by Development Phase

- Use a dedicated branch for each significant development phase.
- Follow descriptive naming conventions such as `phase/02-search` or `phase/03-settings`.
- Create the next phase branch from the appropriate stable branch or completed previous phase.
- Do not create a new branch for every small task.
- Check whether a branch already exists before creating it.
- Never switch branches when doing so could lose or mix uncommitted work.

### 17.2 Frequent, Focused Commits

**Once a requested task is implemented, verified, and working, commit it before starting the next task.**

- Treat each independently completed task as a separate commit.
- If task A is completed and committed, task B should build on that commit rather than accumulating both tasks into one commit.
- Do not wait until an entire development phase is finished to commit completed work.
- Use Conventional Commits with concise, descriptive messages, such as `feat: add application search` or `fix: prevent launcher from closing unexpectedly`.
- Do not commit incomplete, unverified, unrelated, or experimental changes as completed work.
- Inspect the diff and Git status before every commit.
- Preserve unrelated user modifications and never include them accidentally.

### 17.3 Commit Authorization

The user explicitly authorizes automatic commits as part of the normal development workflow.

- You may create commits without asking for approval after each task, provided the task passes its relevant verification and the changes are limited to the authorized scope.
- A request to implement a feature authorizes the necessary focused implementation and its normal verification.
- Do not interpret this authorization as permission to push, merge, rebase, reset, or perform destructive Git operations.
- Never push to a remote unless explicitly requested.
- If unexpected changes, conflicts, secrets, or potential data loss are discovered, stop and ask before proceeding.

### 17.4 Before Finishing a Task

- Confirm the current branch.
- Review the diff for unintended changes.
- Run the relevant verification.
- Commit the completed task if verification passes.
- Report the commit hash, commit message, current branch, and verification results.
- If verification fails, do not claim success or commit the changes as a completed feature. Report the failure and preserve the work.

**Core principle: one completed task, one focused commit; one significant development phase, one dedicated branch.**

## 18. Code Quality and Formatting

- Follow the existing project conventions and configuration.
- Keep changes focused and readable.
- Avoid unrelated formatting changes.
- Do not introduce Prettier or replace existing tooling without approval.
- Do not reformat entire files unnecessarily.
- Do not suppress warnings merely to obtain a clean result.
- Remove temporary debugging code introduced during the task.
- Avoid dead code, unused dependencies, and redundant configuration.
- Do not add comments that simply restate the code.
- Do not leave placeholder implementations or pretend functionality is complete.

When a required implementation cannot be completed, clearly identify the unfinished portion.

## 19. Task Completion and Reporting

A task is complete only after the implementation has been reviewed and the applicable verification has been performed.

Before finishing:

1. Compare the implementation with the original request.
2. Review the diff and confirm the scope is appropriate.
3. Check for accidental changes to configuration or dependencies.
4. Review error handling and relevant security implications.
5. Run applicable verification.
6. Check whether relevant documentation needs updating.
7. Report any remaining limitations.

Use a concise, practical final report containing:

- **Summary:** What changed.
- **Files:** The important files modified.
- **Verification:** Checks run and their results.
- **Limitations:** Untested platforms, failed checks, or unfinished work.
- **Approval needed:** Decisions or follow-up changes requiring the user's authorization.
- **Git:** Report the current branch and, when a completed task was committed, its commit hash and message. Do not leave completed, verified tasks uncommitted merely to batch them with future work.

Do not claim that tests passed if they were not run. Do not claim that a feature works on both platforms without appropriate evidence.

## 20. Priority Order

When requirements compete, prioritize:

1. Security and privacy.
2. Correctness and data integrity.
3. Windows/Linux compatibility.
4. Simplicity and maintainability.
5. Performance and resource efficiency.
6. Development convenience.

Performance remains a core Lux objective, but optimizations must not compromise security, correctness, or reasonable maintainability.

## 21. Final Rule

**Make the smallest safe, correct, well-tested change that satisfies the request.**

Do not invent requirements, expand scope, hide failures, transmit data without authorization, alter protected configuration without approval, or introduce complexity without a concrete reason.

When an important decision exceeds the authority granted by this document, stop and ask the user.