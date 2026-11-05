# Changelog

All notable changes to Zesp Connect are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses
[Semantic Versioning](https://semver.org/).

## [0.1.0] - 2026-11-05

The first public release of Zesp Connect, a free and open source desktop app that brings
your Android phone to your Windows, macOS or Linux computer. It works over USB or Wi-Fi
using adb and scrcpy, with no companion app, no account and no cloud.

### Mirror and control

- Mirror the phone's screen with mouse, touch and keyboard input, in a resizable window
  with always-on-top and a hover control bar.
- Open a single app in its own window, or mirror a phone camera or Android desktop mode.
- Take screenshots from any mirror window and save them to a folder, the clipboard or the
  phone.
- Record mirror windows to MP4, including the phone's sound (Android 13+).
- Drag and drop files onto a mirror window or the main window to send them to the phone.
- Enter sends messages in WhatsApp and Telegram windows, Shift+Enter adds a new line.

### Stay connected

- Play the phone's sound on the computer, switchable per device (Android 11+).
- Two-way clipboard sync for text.
- Now playing: see what is playing on the phone and control playback from the desktop.
- App launcher: press `Ctrl+Shift+Space` (`Cmd+Shift+Space` on macOS) anywhere to search
  for a phone app and open it in its own window. The shortcut can be changed.

### Manage your device

- Overview with device status and a quick settings panel: Wi-Fi, Bluetooth, mobile data,
  airplane mode, dark theme, Do Not Disturb, location, rotation, brightness and volume.
- Apps: browse installed apps by their real names in a list or grid, then launch,
  uninstall, clear data or install an APK.
- Files: browse storage, upload and download, create folders, and preview images, videos,
  audio, PDFs and text without downloading them first.
- Console: run `adb shell` commands and stream logcat with colour-coded severity.
- Emulators: list your Android Virtual Devices and boot them.

### Wireless

- Pair with a QR code or a six-digit pairing code, or switch a USB device to Wi-Fi in one
  click.
- Automatic discovery of phones on your network.
- Saved phones reconnect on their own when they come back on the network.

### Desktop

- Live device detection, with USB and Wi-Fi connections to the same phone shown as one
  device.
- System tray with quick actions and optional launch at startup.
- Guided setup that finds `adb` and `scrcpy`, offers to install them, and tells you when
  updates are available.
- Light and dark themes with several accent colours.

### Install

| Platform              | Download                              |
| --------------------- | ------------------------------------- |
| Windows               | `.exe` (setup) or `.msi`              |
| macOS (Apple silicon) | `aarch64.dmg`                         |
| macOS (Intel)         | `x64.dmg`                             |
| Linux                 | `.AppImage` or `.deb` (Debian/Ubuntu) |

You also need [adb](https://developer.android.com/tools/releases/platform-tools) and
[scrcpy](https://github.com/Genymobile/scrcpy) 3.x or later. If either is missing, the app
offers to install it on first launch. Turn on USB debugging (or Wireless debugging) in the
phone's Developer options.

### Known issues

- The installers are not code-signed yet, so your system may warn you the first time:
  - **Windows:** on the SmartScreen prompt, click **More info**, then **Run anyway**.
  - **macOS:** if the app is blocked, open **System Settings → Privacy & Security** and
    click **Open Anyway**. If macOS says the app is damaged, run
    `xattr -cr "/Applications/Zesp Connect.app"` in Terminal.

[0.1.0]: https://github.com/GauravGhost/Zesp-Connect/releases/tag/v0.1.0
