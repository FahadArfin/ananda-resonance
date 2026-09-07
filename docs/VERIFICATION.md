# Verification — September 6, 2026

## Automated checks

- TypeScript production build: passed.
- Frontend tests: 11 passed. Nine response/validation tests and two WebMCP adapter tests covering native delegation, access revocation and absent browser support.
- Rust tests: 18 passed (exact preset values, Gaming midpoint, filter types, text roundtrips, unsafe directives, bounds, device scoping, Stock, locked destination, external edits, rapid writes, interrupted commit, corrupt database preservation, MCP handshake/error handling/schema validation/framing).
- Windows x64 NSIS installer: built and installed successfully, including upgrade with existing profiles and integration retained.

## Packaged app checks

Verified with Equalizer APO 1.4.2 and connected Fosi K7 endpoint `{63f8b8a3-7684-462c-b6ab-5a68228fab30}`:

- Handover backed up the original `Include: peace.txt`, Peace data and database; stopped Peace; installed the owned include. Exact endpoint scoping is present in active output. No matching Peace autostart entries were found on this machine.
- Initial recoverable backup: `%APPDATA%\com.anandacontrol.desktop\backups\e2592941-6021-4842-9710-be952eccb93e`.
- Six observed profile commits took 5.24–7.18 ms. Timings exclude audible APO reload and do not independently measure visible tray paint latency.
- Temporary profile duplicate, rename, filter autosave, rejected invalid Q, undo, JSON roundtrip, rejected unsupported Include, GraphicEQ import, headphone management and cleanup. All six original profiles retained.
- Hotkey conflict reported for Movies assigned to Ctrl+Alt+1; restored Ctrl+Alt+2. Actual Ctrl+Alt+E opened the quick switcher with all choices.
- Closing the control center destroyed its webview while the Rust app stayed alive. Native profile switching remained available.
- Windows startup preference writes a quoted executable path with `--background`. Native background launch restored saved state with no main window. This is not an actual Windows sign-in test.
- Installed path: `%USERPROFILE%\Applications\Ananda Control\ananda-control.exe`.

## Native MCP and WebMCP

Real installed-process acceptance (`scripts/test_mcp.py`, result in `test-results/mcp-acceptance.json`) passed eight grouped checks:

1. MCP initialize, initialized notification and discovery of 17 tools.
2. Native status with main webview closed; auto-start from an absent tray process.
3. Temporary profile creation, preamp/filter patch, filter addition and removal.
4. Invalid bounds, unsafe imports, unsupported arguments and agent permission changes rejected without partial import.
5. Native JSON import/export roundtrip.
6. Tray-only profile switching and EQ off/on. Observed commits 5.60–10.09 ms across the two installed builds tested.
7. Wrong relay token rejected.
8. All six originals preserved exactly; selection, remembered enabled profile and notifications restored.

Additional installed checks: disabling agent control rejected execution; restoring access worked. A stale GUI save after a native edit was rejected and kept the newer value. Settings exposed the access switch and accurately reported unavailable WebMCP. Native `control_window` opened Settings and closed the main webview normally.

Final installed build also passed malformed JSON-RPC rejection and a second `--background` invocation with one tray process and no main window. It was left in Stock with Gaming remembered, matching the user's pre-test state. No test debugger listeners remained.

The current WebView2 exposes neither `document.modelContext` nor `navigator.modelContext`. WebMCP adapter behavior is unit-tested; actual browser-native tool discovery remains unverified until a supporting runtime is available. Native MCP over stdio is verified and the personal Codex plugin installed successfully.

Tray-only idle sample: 10 seconds, 0.00 additional CPU seconds at process-counter resolution, 16.06 MiB working set and 3.71 MiB private memory. No Ananda WebView2 or development-server process continued. This is a short sample, not a long-duration soak. Debugger flags used for installed UI verification are removed from the final launch.

## Physical / user-assisted acceptance

The user corrected their initial response and explicitly confirmed **“i do hear the change”** during the K7 profile-switch listening test. The app records that user-confirmed audible result. No automated process or file-write test is substituted for it.

Still requires user-assisted testing:

- Confirm the tray checkmark, notification and repeated global profile-hotkey response during playback with the main window closed.
- Repeat after sign-in and sleep/resume.
- Unplug/reconnect K7 and verify connection status and continued processing.
- Confirm unrelated outputs remain unaffected.
- Assess audible reload time, clicks or other artifacts separately from file commits.

The release is functional and packaged, but those remaining physical acceptance steps are not claimed as completed.
