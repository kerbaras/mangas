import { describe, expect, it } from "vitest";
import { withQuality } from "@/lib/kindle-size";

describe("withQuality", () => {
  it("sets one device's quality and removes it for the device's own", () => {
    const stored = { "kobo-clara": 60 };
    expect(withQuality(stored, "kindle-scribe", 70)).toEqual({
      "kobo-clara": 60,
      "kindle-scribe": 70,
    });
    expect(withQuality(stored, "kobo-clara", 50)).toEqual({ "kobo-clara": 50 });
    expect(withQuality(stored, "kobo-clara", null)).toEqual({});
  });
});
