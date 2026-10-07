# Zesp Connect

A cross-platform desktop app (Windows, Linux, macOS) to manage Android devices over
**adb** and **scrcpy** — one codebase: **Tauri 2 + React + TypeScript + Rust**.

Zesp Connect lists your devices, shows real app names (not package ids), mirrors the
screen and camera, drives quick toggles, browses files, exposes an adb console and
handles wireless pairing — all from a single window.

> Status: **M0–M2** are implemented — the architecture skeleton, tool discovery, the push-based
> device tracker (`adb track-devices`), device list/dedupe, the header switcher, onboarding,
> wireless (QR) pairing, the system tray, and an installed-apps manager (real names, search/filter,
> launch, uninstall, clear data, install APK). Extracted app icons land in a later pass.

## Requirements

- **Node 20+** and npm
- **Rust** (stable) with `cargo`
- **adb** (Android platform-tools) — required
- **scrcpy 3.x+** — required for mirroring

Zesp Connect detects both tools on `PATH` and at well-known install locations, and lets
you point it at a custom binary from the Settings screen.

## Tool management

Open **Settings** to manage both binaries:

- See each tool's status, resolved path, version and how it was found.
- **Change location…** to point at your own `adb`/`scrcpy`, or **Use auto-detected** to
  clear the override.
- **Install** (also on the onboarding screen) installs a missing tool via your package
  manager — winget/scoop/choco on Windows, Homebrew on macOS, apt/dnf/pacman on Linux —
  streaming the installer output live into the UI. Installs are always user-triggered;
  nothing runs on its own.

After an install or a path change the app re-scans and restarts the device tracker, so
the new binary is used immediately.

## Development

```bash
npm install
npm run tauri dev
```

Build a production bundle:

```bash
npm run tauri build
```

### Testing without a phone

The app can run against the bundled fake adb:

- Windows: set the adb path to `scripts/fake-adb.cmd`
- macOS/Linux: set the adb path to `scripts/fake-adb.sh`

That returns a canned device list and device info so the UI is fully exercisable.

## Scripts

| Command                                          | What it does                            |
| ------------------------------------------------ | --------------------------------------- |
| `npm run typecheck`                              | TypeScript, no emit                     |
| `npm run lint` / `lint:fix`                      | ESLint                                  |
| `npm run format` / `format:check`                | Prettier                                |
| `npm run test`                                   | Vitest (frontend)                       |
| `npm run build`                                  | `tsc` + Vite build                      |
| `npm run rust:fmt` / `rust:clippy` / `rust:test` | Rust fmt, clippy (`-D warnings`), tests |

## Architecture

```
React UI ──(typed invoke / events via src/lib/ipc.ts)──> #[tauri::command] ──> services ──> adb / scrcpy
   ▲                                                                              │
   └──────────── events: devices:changed, mirror:exited  ◄────────────────────────┘
```

- `src/features/*` — one folder per feature (components, hooks, index). Features never
  import each other; they share through `components/`, `lib/` and the Zustand store.
- `src/lib/ipc.ts` — the **only** place that calls `invoke()`; typed wrappers for the
  whole command surface.
- `src-tauri/src/commands/*` — thin `#[tauri::command]` functions.
- `src-tauri/src/services/*` — the real logic, with **no Tauri types**, unit-tested
  against fixtures in `src-tauri/tests/fixtures/`.
- Processes are spawned with an **argv array** (never a shell string) and with
  `CREATE_NO_WINDOW` on Windows.

## Milestones

1. **M0 Scaffold** — tooling, architecture skeleton, error type, typed IPC, CI. ✅
2. **M1 Core + devices** — tool discovery, `adb track-devices`, device list/dedupe,
   header switcher, onboarding. ✅
   - Settings: tool status, custom paths, and one-click package-manager install. ✅
3. **M2 Apps** — real app names (cache → scrcpy `--list-apps` → `pm` → prettify), search/filter,
   launch, uninstall, clear data, install APK. ✅ (icons extracted from APKs still to come)
4. **M3 Mirroring** — screen, camera, app-window (with the Android 12–13 virtual-display
   fallback), running windows list.
5. **M4 Controls** — quick toggles, coalesced brightness/volume sliders, reboot.
6. **M5 Wireless** — QR + code pairing, connect/disconnect, TCP/IP, duplicate-transport merge.
7. **M6 Files, Console, Clipboard, AVDs.**
8. **M7 Polish & release** — tray, settings, installers, signed builds.

## License

TBD.
