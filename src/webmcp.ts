import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Snapshot } from "./types";

type Tool = { name: string; description: string; inputSchema: Record<string, unknown>; annotations: { readOnlyHint: boolean; destructiveHint: boolean } };
type Context = { registerTool: (tool: Record<string, unknown>, options?: { signal: AbortSignal }) => void; unregisterTool?: (name: string) => void };
const context = () => (document as Document & { modelContext?: Context }).modelContext ?? (navigator as Navigator & { modelContext?: Context }).modelContext;
export const webMcpAvailable = () => !!context();

// Browser-native registration when the installed WebView2 supports WebMCP.
// MCP stdio uses the same Rust catalog/dispatcher and needs no browser at all.
export async function initializeWebMcp() {
  if (!isTauri() || !context()) return;
  const tools = await invoke<Tool[]>("agent_tools");
  let controller: AbortController | undefined;
  const update = (enabled: boolean) => {
    if (enabled === !!controller) return;
    if (!enabled) {
      controller?.abort(); controller = undefined;
      tools.forEach((tool) => context()?.unregisterTool?.(tool.name));
      return;
    }
    controller = new AbortController();
    for (const tool of tools) context()!.registerTool({
      name: tool.name,
      description: tool.description,
      inputSchema: tool.inputSchema,
      annotations: { readOnlyHint: tool.annotations.readOnlyHint, consequentialHint: tool.annotations.destructiveHint, untrustedContentHint: true },
      execute: async (arguments_: Record<string, unknown>) => JSON.stringify(await invoke("agent_call", { name: tool.name, arguments: arguments_ })),
    }, { signal: controller.signal });
  };
  await listen<Snapshot>("state-changed", ({ payload }) => update(payload.database.settings.agentControl));
  update((await invoke<Snapshot>("get_state")).database.settings.agentControl);
}
