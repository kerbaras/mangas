import { afterEach, describe, expect, it, vi } from "vitest";
import { copyText } from "@/lib/clipboard";

const pluginWrite = vi.hoisted(() => vi.fn(async (_text: string) => {}));
vi.mock("@tauri-apps/plugin-clipboard-manager", () => ({ writeText: pluginWrite }));

afterEach(() => {
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  vi.clearAllMocks();
});

describe("copyText", () => {
  it("uses the Tauri clipboard plugin inside the app", async () => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
    const webWrite = vi.fn(async () => {});
    Object.defineProperty(navigator, "clipboard", {
      value: { writeText: webWrite },
      configurable: true,
    });

    await copyText("report");

    expect(pluginWrite).toHaveBeenCalledWith("report");
    expect(webWrite).not.toHaveBeenCalled();
  });

  it("uses the web clipboard in the browser", async () => {
    const webWrite = vi.fn(async () => {});
    Object.defineProperty(navigator, "clipboard", {
      value: { writeText: webWrite },
      configurable: true,
    });

    await copyText("report");

    expect(webWrite).toHaveBeenCalledWith("report");
    expect(pluginWrite).not.toHaveBeenCalled();
  });
});
