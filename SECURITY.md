# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.3.x   | :white_check_mark: |
| < 0.3   | :x:                |

## Reporting a Vulnerability

If you discover a security vulnerability in `prt`, please report it responsibly.

**Do NOT open a public GitHub issue for security vulnerabilities.**

Instead, please email **security@rekurt.dev** or use [GitHub's private vulnerability reporting](https://github.com/rekurt/prt/security/advisories/new).

### What to include

- Description of the vulnerability
- Steps to reproduce
- Affected version(s)
- Potential impact

### Response timeline

- **Acknowledgment:** within 48 hours
- **Initial assessment:** within 1 week
- **Fix or mitigation:** targeting 2 weeks for critical issues

### Scope

The following are in scope:

- Command injection via process names or port data displayed in TUI
- Privilege escalation through sudo password handling
- Terminal escape sequence injection in output (NDJSON, CSV export, watch mode)
- Firewall rule injection via crafted remote addresses
- Path traversal in config file loading

### Out of scope

- Issues requiring physical access to the machine
- Denial of service via intentionally malformed `/proc` data (Linux root required)
- Issues in dependencies (report upstream, but let us know)

## Security Considerations

`prt` is a diagnostic tool that reads system state. Some features require elevated privileges:

- **Firewall blocking** (`b` key) executes `iptables`/`pfctl` commands — requires sudo
- **Process killing** (`K` key) sends SIGTERM/SIGKILL — requires appropriate permissions
- **Strace attach** (`t` key) attaches to processes — requires ptrace permissions
- **SSH forwarding** (`F` key) spawns `ssh` subprocesses

All destructive actions require user confirmation in the TUI before execution.
