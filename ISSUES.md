# Issues and Bug Reports

This document explains how to report bugs, suggest features, and submit other project-related issues for Lux.

A good issue helps maintainers understand the problem, evaluate its relevance, and determine the appropriate next steps.

Before opening an issue, please check existing issues and pull requests to avoid duplicates.

## General Guidelines

When opening an issue:

- Use a clear, descriptive title.
- Keep the report focused on one problem or proposal.
- Provide enough context to understand the situation.
- Include reproducible steps whenever possible.
- Distinguish observed behavior from assumptions.
- Use a respectful and constructive tone.
- Avoid including personal information, credentials, or sensitive data.

Issues are intended to improve Lux. They are not guarantees that a proposed change will be implemented.

## Bug Reports

Use a bug report to describe functionality that does not behave as expected.

A useful bug report should include the following information.

### Description

Describe the problem clearly and explain why the behavior is unexpected.

### Steps to Reproduce

Provide numbered, minimal steps that allow someone else to reproduce the problem.

### Expected Behavior

Explain what should happen.

### Actual Behavior

Explain what happens instead.

### Environment

Include relevant details, such as:

- Lux version or development commit.
- Operating system and version.
- Relevant hardware or system configuration.
- Steps taken before the problem occurred.

### Logs and Evidence

Include relevant error messages, logs, screenshots, or recordings when they help explain the problem.

Remove personal information, credentials, private file paths, and sensitive data before sharing diagnostic information.

If the issue is intermittent, explain how frequently it occurs and under what conditions.

## Feature Requests

Use a feature request to propose new functionality or a meaningful improvement to existing behavior.

Explain:

- **Problem:** What practical problem are you trying to solve?
- **Proposed solution:** What behavior would address it?
- **Use case:** When and why would users need it?
- **Alternatives:** Could existing functionality or a simpler approach solve the problem?
- **Considerations:** Are there relevant performance, accessibility, compatibility, security, or privacy implications?

Feature requests should align with Lux's core goals:

- Fast, keyboard-first interactions.
- Low idle resource usage.
- Reliable and predictable behavior.
- A clean, customizable interface.
- Privacy by default and local-first operation.
- Windows and Linux compatibility.
- Minimal unnecessary complexity.

Features that introduce network communication, telemetry, external services, or transmission of user data require explicit project authorization before implementation.

## Performance Issues

Performance reports are especially useful when they include measurable evidence.

When applicable, provide:

- The operation that feels slow or consumes excessive resources.
- Approximate startup or response times.
- Idle or active CPU and memory usage.
- Steps to reproduce the performance problem.
- Whether the issue occurs consistently.
- The affected operating system and hardware, where relevant.

Distinguish measured values from subjective observations.

Do not assume that a performance problem is caused by a particular component without evidence.

## Documentation Issues

Use a documentation issue to report:

- Incorrect or outdated instructions.
- Missing setup requirements.
- Unclear explanations.
- Broken links.
- Missing information about platform-specific behavior.

Include the relevant file, section, or link and explain what should be corrected.

Small, obvious documentation corrections may be submitted directly as pull requests.

## Platform-Specific Issues

Lux targets both Windows and Linux.

If a problem affects only one platform, state that explicitly.

Include the relevant operating system version and any platform-specific details necessary to reproduce the problem.

Do not assume that behavior observed on one platform also occurs on the other.

## Security Issues

**Do not report security vulnerabilities through public issues.**

Follow [SECURITY.md](SECURITY.md) for the appropriate reporting procedure.

Public issues are suitable for ordinary bugs and feature requests, but not for unpatched vulnerabilities or sensitive exploit details.

## Before Opening an Issue

Please check the following:

- [ ] I searched for existing issues and pull requests.
- [ ] The title clearly describes the problem or proposal.
- [ ] I included the relevant context and environment.
- [ ] I provided reproduction steps when applicable.
- [ ] I removed sensitive information from logs and attachments.
- [ ] This is not a security vulnerability that requires private disclosure.

Not every item applies to every issue. Provide the information relevant to your report.

## What Happens Next?

Maintainers may:

- Request additional information.
- Attempt to reproduce the problem.
- Ask for clarification about the proposed behavior.
- Label or prioritize the issue.
- Link the issue to existing work.
- Close duplicate, incomplete, out-of-scope, or non-actionable reports.
- Accept the proposal for future consideration.

Response times and implementation priorities depend on project activity, issue severity, complexity, and available maintenance resources.

Please avoid repeatedly posting the same report to request updates.

## Contributing a Fix

You are welcome to work on an issue once its scope is sufficiently clear.

For substantial changes, discuss the proposed implementation with maintainers before starting work.

Follow [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, coding guidelines, testing, and pull request instructions.

Opening an issue does not reserve it indefinitely or guarantee that a contribution will be merged.

## Thank You

Clear, reproducible, well-scoped issues help keep Lux reliable, maintainable, and useful to its users.

Thank you for taking the time to report problems and help improve the project.
