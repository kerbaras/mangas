import { describe, expect, it, vi } from "vitest";
import type { ProviderStatus } from "@/lib/ipc";

const pending: ((s: ProviderStatus) => void)[] = [];
vi.mock("@/lib/ipc", () => ({
  providerStatus: () => new Promise<ProviderStatus>((resolve) => pending.push(resolve)),
}));

const { checkStatus, readChecks } = await import("@/lib/provider-status");

describe("checkStatus", () => {
  it("keeps the newest probe when an older one answers last", async () => {
    const older = checkStatus("mangadot");
    const newer = checkStatus("mangadot");
    pending[1]({ state: "ready" });
    await newer;
    pending[0]({ state: "needs_session" });
    await older;
    expect(readChecks().mangadot?.status).toEqual({ state: "ready" });
  });
});
