# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Workspace architecture: `prt-core` library + `prt` TUI binary
- Multilingual support: English (default), Russian, Chinese
- Auto-detect system locale (`PRT_LANG` env or `--lang` flag override)
- Process tree view with parent chain
- Network interface details tab
- Connection details tab with all process ports
- Sudo password input from TUI for elevated scanning
- Session struct for shared scan/diff/sort logic
- Live change tracking: new (green), gone (red, 5s fade)
- Filter by port, process, PID, protocol, state, user
- Sort by any column (Tab/Shift+Tab)
- Kill process with confirmation (SIGTERM/SIGKILL)
- Copy to clipboard (line or PID)
- Export to JSON/CSV (`--export json|csv`)
- Batch `ps` calls on macOS (2 calls per scan, not 4*N)
- Panic hook for terminal recovery
- CI/CD with GitHub Actions (lint, test, release)
- cargo-deny for dependency auditing
