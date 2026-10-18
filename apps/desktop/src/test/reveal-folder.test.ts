import { beforeEach, describe, expect, it, vi } from "vitest";

const openFolder = vi.hoisted(() => vi.fn(async (_path: string) => {}));
const notifyError = vi.hoisted(() => vi.fn());
vi.mock("@/lib/ipc.folders", () => ({ openFolder }));
vi.mock("@/lib/notify", () => ({ notifyError }));

import { revealFolder } from "@/lib/dialogs";

describe("revealFolder", () => {
  beforeEach(() => vi.clearAllMocks());

  it("opens the folder through the core's checked command", async () => {
    await revealFolder("/home/me/Exports");
    expect(openFolder).toHaveBeenCalledWith("/home/me/Exports");
    expect(notifyError).not.toHaveBeenCalled();
  });

  it("says so when the folder can't be opened instead of failing silently", async () => {
    const refused = new Error("/etc is outside the app's folders");
    openFolder.mockRejectedValueOnce(refused);
    await revealFolder("/etc");
    expect(notifyError).toHaveBeenCalledWith("Couldn't open the folder", refused);
  });
});
