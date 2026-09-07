# Ananda Resonance

Home of **Ananda Control** — your sound, one click away.

A Windows tray-first frontend for **Equalizer APO**, built with Tauri 2, Rust, React, and TypeScript. Peace is neither launched nor required during daily use. Equalizer APO remains the audio processor.

## Install and connect

1. Install Equalizer APO once and enable it on your output using its Device Selector. This machine already has Equalizer APO 1.4.2 and a Fosi K7 endpoint.
2. Run the generated `Ananda Control_0.1.0_x64-setup.exe`. The installer installs for the current Windows user and handles a missing WebView2 runtime.
3. Open Ananda Control. Review the output and the one-time migration screen, then choose **Back up & connect Equalizer APO**.
4. Setup backs up the original root configuration, Peace EQ/settings, the Ananda database, and identified Peace startup entries. It stops the Peace executable from the APO config directory and disables its matching startup entries. Peace remains installed.
5. Play audio through your selected output and perform the listening test under **Device & health**. File installation and endpoint registration do not prove audible processing.

The setup screen replaces the whole root configuration with Ananda's include, rather than appending a second EQ to Peace. Existing root effects will no longer run. Backups are retained under `%APPDATA%\com.anandacontrol.desktop\backups`.

If folder permissions prevent setup, the error screen offers a one-time Windows elevation helper. It grants the initiating account write access only to the APO root staging operation, `config.txt`, and `ananda-control`. The ordinary app remains unelevated. It refuses reparse-point targets. APO device installation itself remains the responsibility of APO's Device Selector.

## Everyday controls

| Control | Default |
| --- | --- |
| Balanced Music | Ctrl+Alt+1 |
| Movies / Cinematic | Ctrl+Alt+2 |
| Gaming | Ctrl+Alt+3 |
| Competitive FPS | Ctrl+Alt+4 |
| Voice / Discord | Ctrl+Alt+5 |
| Stock / EQ Off | Ctrl+Alt+0 |
| Quick switcher | Ctrl+Alt+E |

Right-click the tray icon and choose a profile. Selection writes the owned APO file without an Apply button or audio-service restart. Stock has no filters and a 0 dB preamp. The EQ toggle remembers the last enabled profile.

The control center closes to the tray by default and releases its webview. The Rust process continues handling menus, hotkeys, notifications, and events. Explicit **Exit** terminates the app; APO keeps processing its last configuration. Windows startup launches with `--background`. All five background preferences can be disabled in Settings. Disabling profile restoration does not undo an APO file already saved to disk.

## Native agent interface

The installed **Ananda Control** Codex plugin exposes 17 typed native MCP tools. It can switch profiles, edit filters, manage headphones and read diagnostics with the main window closed. It launches the Rust tray app as needed and uses no browser automation or Node server. Settings includes **Allow native agent control**.

WebMCP registration is also implemented for compatible WebView2 versions; the currently installed WebView2 does not expose WebMCP, so Codex uses the native stdio interface. See [agent setup and tool reference](docs/AGENT-CONTROL.md).

## Profiles and editor

The four supplied text files are bundled unchanged as initial data. Gaming averages the corresponding Music/Movies gains and uses a −7 dB preamp. No model correction or competitive advantage is claimed for these preference presets.

- Changes to valid filter values save after 350 ms; active-profile edits also update APO. Invalid or incomplete values never reach APO. Undo retains up to 40 local editor changes.
- Response graphs include preamp and estimate filter response at 48 kHz. GraphicEQ curves use log-frequency interpolation; the display is not an audio measurement. Headroom is an estimate, and its adjustment is always explicit.
- Supported filters: PK, LSC, HSC, HPQ, LPQ. Imports also normalize basic LS, HS, HP, and LP syntax.
- Import/export: versioned Ananda profile JSON and supported Equalizer APO text, including parametric AutoEQ exports and one GraphicEQ block. Text imports reject unsupported directives rather than partially importing or executing them. Includes, device commands, arbitrary expressions, convolution, and VSTs are not imported.
- Headphone entries are manually selected. Profiles can be moved between entries in the editor. Select a different enabled profile before deleting an active or last-enabled profile. Stock is immutable.
- Shortcut fields accept `Ctrl+Alt+1`, `Shift+F8`, etc. Empty disables a binding. Windows or another app may reserve a shortcut; conflicts are reported without preventing startup.

## Recovery

**Device & health** distinguishes installation, endpoint registration, connected state, configuration ownership, and the user-confirmed listening test. Shared-mode playback should be tested; exclusive-mode/ASIO routes can bypass system effects.

If another frontend changes `config.txt` or `active.txt`, switching stops with a visible conflict. **Review / repair integration** backs up the conflicting files before reclaiming ownership. **Restore previous configuration** restores the initial root backup and recorded Peace startup entries, disables Ananda startup, and retains Ananda profiles. Use this before uninstalling if you want the previous frontend back.

`database.json` is authoritative. Every applied profile transaction journals the next database, atomically replaces the APO file, and commits the database. A failed database commit attempts to restore the old APO file. Startup reconciles interrupted transactions against actual file contents. Corrupt database files are copied aside, and integration is not assumed to be owned.

## Build and test

Requirements: Windows x64, Node.js/npm, Rust stable (MSVC), Visual Studio C++ build tools and Windows SDK, and WebView2.

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
npm.cmd ci
npm.cmd run check
npm.cmd test
cargo test --manifest-path src-tauri/Cargo.toml --lib
npm.cmd run desktop:dev
npm.cmd run desktop:build
```

The installer is generated in `src-tauri/target/release/bundle/nsis/`. The standalone executable is `src-tauri/target/release/ananda-control.exe`. The release does not require Node or a development server. The installer is unsigned; this project does not include a code-signing certificate or an automatic updater.

See [verification notes](docs/VERIFICATION.md) for measured results and remaining physical-device acceptance steps.

## Architecture and references

Rust owns the database, atomic writer, setup/recovery, Win32 endpoint registry monitoring, filesystem watching, global shortcuts, tray, and notifications. React only renders the control center and temporary switcher; state is published using `state-changed`. Narrow commands expose profile, settings, import/export, activation, and diagnostics operations. The UI has no general shell or filesystem permission.

- [Equalizer APO configuration reference](https://sourceforge.net/p/equalizerapo/wiki/Configuration%20reference/): Include, Device scoping, filter syntax.
- [Equalizer APO troubleshooting](https://sourceforge.net/p/equalizerapo/wiki/Documentation/): audio enhancements and trace logging.
- [Tauri system tray](https://v2.tauri.app/learn/system-tray/), [global shortcuts](https://v2.tauri.app/plugin/global-shortcut/), [Windows packaging](https://v2.tauri.app/distribute/windows-installer/).

Equalizer APO is a separate installation and is not redistributed in this app. Bundled preset data came from the user's supplied local files. Icons are Lucide (ISC), and the application icon is original geometric artwork.
