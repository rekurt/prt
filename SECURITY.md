# Security policy

## Supported versions

Security fixes are provided for the latest released minor version.

| Version | Status |
|---|---|
| 0.6.x | Supported |
| Earlier releases | Upgrade required |

If the version on [Crates.io](https://crates.io/crates/prt) is newer than this table, use the newest release and mention the documentation mismatch in your report.

## Report a vulnerability

Do not open a public GitHub issue for a suspected vulnerability.

Use one of these private channels:

- Email `security@rekurt.dev`.
- Submit a report through [GitHub private vulnerability reporting](https://github.com/rekurt/prt/security/advisories/new).

Include:

- affected `prt` version and operating system;
- vulnerability description and potential impact;
- minimal reproduction steps or a proof of concept;
- required privileges and environmental assumptions;
- any mitigation you have already tested.

Do not include real credentials, private keys, production hostnames, or unrelated personal data.

## Response targets

- Acknowledgment within 48 hours
- Initial assessment within one week
- A fix or mitigation target of two weeks for critical issues

These are targets, not guarantees. Coordinated disclosure timing will be agreed with the reporter.

## In scope

- Command or argument injection through process, address, SSH, or configuration data
- Privilege escalation or credential exposure in sudo handling
- Terminal escape-sequence injection in the TUI, watch mode, or exported output
- Firewall rule injection or unsafe command construction
- Unsafe process termination or tracing behavior
- Path traversal in configuration or SSH config loading
- Sensitive data exposure through logs, status messages, clipboard actions, or exports

## Normally out of scope

- Issues requiring physical access to an already-unlocked machine
- Denial of service that requires root to deliberately corrupt `/proc`
- Vulnerabilities that exist only in an unsupported release and are fixed in the latest version
- Dependency vulnerabilities without a demonstrated impact on `prt`; report upstream as well, but tell us if `prt` is affected

## Privileged and state-changing actions

Scanning, filtering, exporting, and navigation are read-only. The contextual action menu opened with `Space` can perform operations with broader impact:

- **Terminate process** asks for SIGTERM or SIGKILL before sending a signal.
- **Block remote IP** asks for confirmation and invokes `iptables` on Linux or `pfctl` on macOS with suitable privileges.
- **Trace system calls** attaches `strace` or `dtruss` and therefore depends on platform tracing permissions.
- **SSH forward** starts an `ssh` subprocess using values from the reviewed tunnel form or saved configuration.

Run `prt` with the least privilege needed. Prefer an unprivileged session and elevate only when missing process visibility or a chosen action requires it.
