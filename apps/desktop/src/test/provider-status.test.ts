import { describe, expect, it } from "vitest";
import { providerDetail } from "@/lib/providers";

const base = {
  site: "https://mangadot.net",
  needsSession: true,
  savedAt: undefined,
  status: undefined,
  checkedAt: undefined,
  ago: () => "2 min ago",
};

describe("providerDetail", () => {
  it("shows checking while the probe runs", () => {
    expect(providerDetail({ ...base, needsSession: false })).toBe("mangadot.net · checking…");
    expect(providerDetail(base)).toBe("mangadot.net · not connected · checking…");
  });

  it("tells a missing session from an expired one", () => {
    const status = { state: "needs_session" } as const;
    expect(providerDetail({ ...base, status, checkedAt: 1 })).toBe(
      "mangadot.net · not connected · checked 2 min ago",
    );
    expect(providerDetail({ ...base, status, checkedAt: 1, savedAt: null })).toBe(
      "mangadot.net · connected · session expired · checked 2 min ago",
    );
  });

  it("includes the reason when unreachable", () => {
    const status = { state: "unreachable", reason: "HTTP 503" } as const;
    expect(providerDetail({ ...base, needsSession: false, status, checkedAt: 1 })).toBe(
      "mangadot.net · HTTP 503 · checked 2 min ago",
    );
  });
});
