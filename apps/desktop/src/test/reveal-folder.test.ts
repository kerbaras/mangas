import { beforeEach, describe, expect, it, vi } from "vitest";

const openPath = vi.hoisted(() => vi.fn(async (_path: string) => {}));
const notifyError = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/plugin-opener", () => ({ openPath }));
vi.mock("@/lib/notify", () => ({ notifyError }));
vi.mock("@/lib/ipc", () => ({ isTauri: () => true }));

import { revealFolder } from "@/lib/dialogs";

describe("revealFolder", () => {
  beforeEach(() => vi.clearAllMocks());

  it("opens the folder", async () => {
    await revealFolder("/home/me/Exports");
    expect(openPath).toHaveBeenCalledWith("/home/me/Exports");
    expect(notifyError).not.toHaveBeenCalled();
  });

  it("says so when the opener refuses instead of failing silently", async () => {
    const denied = new Error("opener.open_path not allowed");
    openPath.mockRejectedValueOnce(denied);
    await revealFolder("/home/me/Exports");
    expect(notifyError).toHaveBeenCalledWith("Couldn't open the folder", denied);
  });
});
