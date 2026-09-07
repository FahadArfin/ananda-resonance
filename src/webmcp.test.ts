// @vitest-environment node
import { afterEach, expect, test, vi } from "vitest";
const invoke = vi.fn();
const listen = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...args: unknown[]) => invoke(...args), isTauri: () => true }));
vi.mock("@tauri-apps/api/event", () => ({ listen: (...args: unknown[]) => listen(...args) }));
afterEach(() => { vi.unstubAllGlobals(); vi.resetAllMocks(); });
test("registers catalog tools and delegates execution to native dispatcher", async () => {
  const registerTool = vi.fn(); const unregisterTool = vi.fn();
  vi.stubGlobal("document", { modelContext: { registerTool, unregisterTool } });
  vi.stubGlobal("navigator", {});
  invoke.mockImplementation(async (name: string) => name === "agent_tools" ? [{ name: "set_eq", description: "Toggle", inputSchema: {}, annotations: { readOnlyHint: false, destructiveHint: false } }] : name === "get_state" ? { database: { settings: { agentControl: true } } } : { ok: true });
  const { initializeWebMcp } = await import("./webmcp");
  await initializeWebMcp();
  const tool = registerTool.mock.calls[0][0];
  expect(await tool.execute({ enabled: false })).toBe('{"ok":true}');
  expect(invoke).toHaveBeenCalledWith("agent_call", { name: "set_eq", arguments: { enabled: false } });
  listen.mock.calls[0][1]({ payload: { database: { settings: { agentControl: false } } } });
  expect(unregisterTool).toHaveBeenCalledWith("set_eq");
});
test("unsupported WebView2 safely leaves native MCP available", async () => {
  vi.stubGlobal("document", {}); vi.stubGlobal("navigator", {});
  const { initializeWebMcp, webMcpAvailable } = await import("./webmcp");
  expect(webMcpAvailable()).toBe(false); await initializeWebMcp(); expect(invoke).not.toHaveBeenCalled();
});
