import { describe, expect, it } from "vitest";
import { providerStatusText } from "@/lib/providers";

describe("providerStatusText", () => {
  it("shows checking while the probe runs", () => {
    expect(providerStatusText(false, undefined, undefined)).toBe("checking…");
    expect(providerStatusText(true, undefined, null)).toBe("connected · checking…");
  });

  it("reads ready, or connected for session sites", () => {
    expect(providerStatusText(false, { state: "ready" }, undefined)).toBe("ready");
    expect(providerStatusText(true, { state: "ready" }, null)).toBe("connected");
  });

  it("tells a missing session from an expired one", () => {
    expect(providerStatusText(true, { state: "needs_session" }, undefined)).toBe(
      "needs a browser session",
    );
    expect(providerStatusText(true, { state: "needs_session" }, "2026-10-16T10:00:00Z")).toBe(
      "session expired, connect again",
    );
  });

  it("includes the reason when unreachable", () => {
    expect(providerStatusText(false, { state: "unreachable", reason: "HTTP 503" }, undefined)).toBe(
      "unreachable (HTTP 503)",
    );
  });
});
