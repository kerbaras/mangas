import { describe, expect, it } from "vitest";
import type { ProviderInfo } from "@/lib/ipc";
import {
  isEnabled,
  languagesFor,
  searchable,
  withEnabled,
  withLanguages,
} from "@/lib/provider-prefs";
import { statusBadge } from "@/lib/providers";
import { ago } from "@/lib/relative-time";

const prefs = {
  providersEnabled: { mangadot: false },
  providerLanguages: { mangadex: ["es", "en"], comick: [] },
  preferredLanguages: ["fr"],
};

const info = (id: string, capabilities: ProviderInfo["capabilities"]): ProviderInfo => ({
  id,
  name: id,
  site: `https://${id}`,
  languages: ["en"],
  capabilities,
  needs_browser_session: false,
  icon_url: null,
});

describe("provider preferences", () => {
  it("enables providers unless switched off", () => {
    expect(isEnabled(prefs, "mangadot")).toBe(false);
    expect(isEnabled(prefs, "mangadex")).toBe(true);
    expect(isEnabled({ providersEnabled: {} }, "added-later")).toBe(true);
    expect(withEnabled(prefs, "mangadot", true)).toEqual({ mangadot: true });
  });

  it("falls back to the app languages", () => {
    expect(languagesFor(prefs, "mangadex")).toEqual(["es", "en"]);
    expect(languagesFor(prefs, "comick")).toEqual(["fr"]);
    expect(languagesFor(prefs, "weebcentral")).toEqual(["fr"]);
  });

  it("drops an emptied language list", () => {
    expect(withLanguages(prefs, "mangadex", [])).toEqual({ comick: [] });
    expect(withLanguages(prefs, "tapas", ["en"])).toMatchObject({ tapas: ["en"] });
  });

  it("searches enabled providers that can search, in order", () => {
    const list = [info("mangadex", ["search"]), info("mangadot", ["search"]), info("x", [])];
    expect(searchable(list, prefs).map((p) => p.id)).toEqual(["mangadex"]);
  });
});

describe("status labels", () => {
  it("names every state", () => {
    expect(statusBadge(undefined)).toEqual({ label: "Checking", tone: "pending" });
    expect(statusBadge({ state: "ready" }).label).toBe("Ready");
    expect(statusBadge({ state: "needs_session" }).label).toBe("Needs connection");
    expect(statusBadge({ state: "unreachable", reason: "x" }).tone).toBe("bad");
  });

  it("says how long ago a check ran", () => {
    const now = Date.UTC(2026, 9, 18, 12);
    expect(ago(now - 5_000, now)).toBe("just now");
    expect(ago(now - 5 * 60_000, now)).toBe("5 min ago");
    expect(ago(now - 3 * 3_600_000, now)).toBe("3 h ago");
    expect(ago(now + 1_000, now)).toBe("just now");
  });
});
