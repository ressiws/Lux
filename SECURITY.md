# Security Policy

Security is a priority for Lux.

Lux is a local-first, privacy-focused desktop launcher for Windows and Linux. We take security vulnerabilities seriously and encourage responsible disclosure.

## Supported Versions

Security fixes are prioritized for the latest stable release.

| Version | Security Updates |
| --- | --- |
| Latest stable release | Supported |
| Older releases | Not guaranteed |

Development builds may contain unresolved issues and should not be assumed to provide the same level of security as a stable release.

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

Public disclosure before a vulnerability has been assessed and addressed may expose users to unnecessary risk.

When reporting a vulnerability, include as much of the following information as possible:

- **Description:** A clear explanation of the vulnerability.
- **Affected versions:** The releases or development versions you believe are affected.
- **Impact:** What an attacker could potentially achieve.
- **Reproduction steps:** A minimal, reproducible demonstration.
- **Requirements:** Any permissions, access, or conditions necessary to exploit the issue.
- **Evidence:** Relevant logs, screenshots, or proof-of-concept material, with sensitive information removed.
- **Suggested mitigation:** If known, a possible fix or workaround.

Do not include real credentials, private user data, or other sensitive information in your report.

### Where to Report

Use GitHub's private vulnerability reporting feature for this repository, if it is enabled.

You can find it under the repository's **Security** tab, through the private vulnerability reporting option.

If private vulnerability reporting is unavailable, contact the repository maintainers through an established private communication channel.

Do not publish exploit details while waiting for a response.

## Scope

Security reports are welcome for vulnerabilities affecting Lux or its officially maintained components, including:

- Unauthorized access to local files or sensitive information.
- Path traversal and unsafe filesystem operations.
- Command or code execution vulnerabilities.
- Insecure handling of external or untrusted input.
- Incorrect Tauri permissions or privilege boundaries.
- Exposure of secrets or sensitive diagnostic information.
- Unsafe process execution or application discovery.
- Unintended network communication or data disclosure.
- Vulnerabilities in plugin functionality maintained by the project.
- Dependency vulnerabilities that materially affect Lux.

Reports should explain how the issue affects the application's actual security rather than relying solely on theoretical concerns.

## Security Expectations

Lux aims to follow these principles:

- Privacy by default.
- Minimal permissions and least privilege.
- Local-first operation.
- Explicit authorization for functionality that transmits application or user data.
- Safe handling of untrusted input.
- Minimal collection and retention of sensitive information.
- Transparent communication about relevant security limitations.

The existence of a security policy does not guarantee that every component is free from vulnerabilities.

## Responsible Disclosure

Please allow maintainers reasonable time to investigate and address a reported vulnerability before publicly disclosing technical details.

When reporting a vulnerability, avoid actions that could damage systems, compromise other users' data, or disrupt services.

Do not access, modify, or exfiltrate data that does not belong to you.

We aim to assess reports carefully, communicate relevant findings, and provide fixes or mitigations when appropriate. Response and remediation times depend on the severity, complexity, and available maintenance resources.

## Out of Scope

The following are generally not considered actionable security vulnerabilities without evidence of a concrete security impact:

- Reports based solely on outdated dependencies without an applicable vulnerability.
- Theoretical attacks without a credible exploitation path.
- Issues requiring extensive control of the victim's machine without demonstrating an additional security impact.
- Vulnerabilities in unrelated third-party software.
- General feature requests or ordinary bugs without security implications.

These cases may still be considered when additional evidence demonstrates a meaningful risk to Lux users.

## Updates

This policy may change as the project matures.

Please refer to the current version of this file when submitting a security report.
