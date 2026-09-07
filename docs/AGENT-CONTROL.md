# Native agent control

Ananda Control exposes the same 17 tools through two interfaces:

1. **MCP over stdio**: run the installed `ananda-control.exe --mcp`. This is the working Codex interface and works with the main webview destroyed. It starts the tray app quietly if necessary.
2. **WebMCP**: the control-center document registers tools through `document.modelContext` when available, with a legacy `navigator.modelContext` fallback. The installed WebView2 on the tested machine exposes neither API. Registration and execution delegation are covered by adapter tests; browser-native WebMCP execution remains dependent on a supporting WebView2 release.

No computer-use automation, development server, Node process, browser debugger, or Peace process is needed for MCP operation.

## Codex plugin

The personal `ananda-control` plugin is installed from the personal marketplace. Start a new Codex task to load its tools. Examples:

- “Switch Ananda Control to Movies.”
- “Turn EQ off.”
- “Duplicate Balanced Music as Late Night, then lower its preamp to -8 dB.”
- “Show my active profile and K7 connection status.”

The local plugin source is `%USERPROFILE%\plugins\ananda-control`. Its `.mcp.json` currently points to the installation at `%USERPROFILE%\Applications\Ananda Control\ananda-control.exe`. If moving the application, update that command and reinstall the plugin. Application upgrades at the same path need no plugin change.

For another MCP client, configure:

```json
{
  "mcpServers": {
    "ananda-control": {
      "command": "C:\\Users\\fahad\\Applications\\Ananda Control\\ananda-control.exe",
      "args": ["--mcp"]
    }
  }
}
```

Use the actual installed executable path on another computer. Protocol versions 2025-11-25, 2025-06-18 and 2024-11-05 are negotiated. The server advertises only tools, with text and structured results, and reports execution failures using `isError`.

## Tools

| Tools | Purpose |
| --- | --- |
| `get_status`, `list_profiles`, `get_profile` | Read authoritative state, endpoint health, exact tuning and provenance |
| `activate_profile`, `set_eq` | Switch immediately or select Stock and restore the last enabled profile |
| `set_preamp`, `edit_filter`, `add_filter`, `remove_filter` | Validated edits; active-profile changes apply immediately |
| `manage_profile` | Create, duplicate, rename, delete and reorder |
| `import_profile`, `export_profile` | Transfer APO text or native JSON as data; no arbitrary file operations |
| `get_settings`, `update_settings` | Read or patch preferences and hotkeys; registration conflicts are returned |
| `manage_headphone`, `set_profile_headphone` | Manage and manually assign headphones |
| `control_window` | Open a page or request normal close with pending edits flushed |

Profile selectors accept an ID or an exact case-insensitive name. Duplicate names require IDs. Filter edits require a filter ID returned by `get_profile`. The complete schemas live in `src-tauri/agent-tools.json` and are returned by `tools/list`.

## Access and correctness

**Settings → Allow native agent control** disables tool execution for both interfaces. Only the user-facing native settings command can change this preference; an agent tool cannot grant itself access. The installed plugin remains visible while access is disabled, but calls return an error.

The stdio facade talks to the Rust tray process over authenticated TCP bound only to `127.0.0.1`. The randomly generated token and ephemeral port live in the user's application-data `agent-bridge.json`; they are not returned by tools. Messages are bounded to 2 MiB, connections have timeouts, and concurrency is bounded. This is a same-user trust boundary: another process able to read that user's private files has the same local access. There is no HTTP interface or public listening address.

All mutations use the existing validated storage and APO transaction pipeline. Active configuration ownership is checked before writes. Narrow filter patches read the latest profile under its storage lock. The editor rejects a stale draft rather than overwriting a concurrent change. A lost transport response is not automatically retried after sending a request, avoiding duplicate mutations.

Agents cannot invoke elevated setup, restoration, arbitrary file paths, raw includes, shell commands, Windows volume, or the user-confirmed audible-verification action. Destructive profile operations carry MCP hints, but the connected client is responsible for its own approval policy. File commit success is never presented as a listening test.

## Verification

Run `python scripts/test_mcp.py "<installed executable>"` with the main window closed. It tests real stdio initialization, tool discovery, tray-only switching, temporary profile edits, JSON roundtrip, invalid inputs and unauthorized relay calls. It restores the original selection, remembered profile and notification preference, and removes its temporary profiles.

For a direct native diagnostic call without a client, use `ananda-control.exe --call get_status "{}"` from a process that captures stdout. Windows GUI-subsystem executables do not necessarily make an interactive PowerShell console wait; MCP clients use redirected pipes.

References: [WebMCP imperative API](https://developer.chrome.com/docs/ai/webmcp/imperative-api), [MCP tools](https://modelcontextprotocol.io/specification/2025-11-25/server/tools), [MCP transports](https://modelcontextprotocol.io/specification/2025-11-25/basic/transports).
