# Rustcord

Rustcord is a lightweight, gaming-optimized desktop Discord client built with Rust and Tauri v2, using the native system WebView2 instead of Electron.

The setup installer size is only ~3.9 MB (compared to 150+ MB for the official Electron client), with significantly reduced RAM and CPU consumption.

---

## Key Features

### 1. Minimal Resource Usage and Stable WebRTC
- Built on Microsoft Edge WebView2 (Blink/Chromium), utilizing OS-shared runtimes.
- Configured background flags:
  - `--disable-background-timer-throttling`
  - `--disable-backgrounding-occluded-windows`
  - `--disable-renderer-backgrounding`
  - `--enable-features=WebRTCPeerConnectionWithBlockIceRegathering`
- Voice communication (WebRTC) stays connected without stuttering or audio drops when minimized to the tray during demanding games.

### 2. Built-in Equicord Client Mod Support
- Automatically retrieves the latest `browser.js` and `browser.css` from the official Equicord GitHub repository to `%LOCALAPPDATA%\com.rustcord.app\equicord`.
- Bundled fallback assets ensure immediate launch even offline.
- Injected via Tauri's `initialization_script` before Discord scripts load.
- Full Cloud Integration support (Equicord Cloud and Vencord Cloud OAuth) via a native backend HTTP proxy, bypassing browser Content Security Policy (CSP) and CORS restrictions.
- Comprehensive Themes support: load local `.theme.css` files from `%LOCALAPPDATA%\com.rustcord.app\themes\` or paste online theme URLs.

### 3. Native Titlebar and Pure OLED Dark Theme
- Borderless window with native Windows drop shadow.
- Dedicated 26px top titlebar positioned cleanly above Discord:
  - Discord icon and wordmark on the left
  - Draggable header area (double-click to maximize/restore)
  - Dedicated window controls on the right: Minimize, Maximize / Restore, and Close (minimize to tray)
- Pure OLED dark theme (`#000000` deep black background with a `#F84800` accent window border).

### 4. Gaming Mode
- Toggleable via system tray or a global keyboard shortcut.
- Disables resource-intensive UI animations and rendering overhead:
  - Accelerates CSS Transitions and Keyframe Animations to `0.001ms`.
  - Removes heavy `backdrop-filter` blur effects from modals.
  - Hides animated Nitro avatars, profile effects, and banner canvases/videos.
  - Pauses autoplay for animated GIFs and stickers.

### 5. System Tray and Global Hotkeys
- Closing the window (X) minimizes the client to the system tray rather than killing the process.
- System Tray Context Menu:
  - Open Rustcord
  - Mute Microphone
  - Deafen Audio
  - Toggle Game Mode
  - Quit
- Global shortcuts (functional over full-screen games without losing focus):
  - `Ctrl + Shift + M` — Toggle Microphone (Mute / Unmute)
  - `Ctrl + Shift + D` — Toggle Audio (Deafen / Undeafen)
  - `Ctrl + Shift + G` — Toggle Game Mode
  - `F12` or `Ctrl + Shift + I` — Open Developer Tools (DevTools)

### 6. Automated Updates (Auto-updater)
- Powered by `tauri-plugin-updater` v2.
- Automatically checks for new versions on GitHub Releases at application startup.
- Cryptographically verified update packages signed with minisign keys.

---

## Building from Source

### Prerequisites
- Rust 1.77+ (MSVC toolchain on Windows)
- Microsoft Visual Studio 2022 C++ Build Tools
- Node.js (optional)

### Build Commands

```bash
# Navigate to project directory
cd rustcord

# Check compilation
cargo check --manifest-path src-tauri/Cargo.toml

# Build release NSIS installer
cargo tauri build
```

The resulting installer is generated at:
`src-tauri/target/release/bundle/nsis/Rustcord_1.0.0_x64-setup.exe`

---

## License

MIT License. Discord is a trademark of Discord Inc. Rustcord is an independent open-source project and is not affiliated with or endorsed by Discord Inc.
