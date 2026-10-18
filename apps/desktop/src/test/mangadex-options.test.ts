import { describe, expect, it } from "vitest";
import {
  MANGADEX_DEFAULTS,
  parseGroups,
  parseMangaDexOptions,
  toggleLanguage,
  toggleRating,
} from "@/lib/mangadex-options";

describe("parseMangaDexOptions", () => {
  it("defaults to safe + suggestive, full quality, no groups", () => {
    expect(parseMangaDexOptions(undefined)).toEqual(MANGADEX_DEFAULTS);
    expect(parseMangaDexOptions("nope")).toEqual(MANGADEX_DEFAULTS);
  });

  it("keeps known values and drops the rest", () => {
    expect(
      parseMangaDexOptions({
        content_ratings: ["erotica", "bogus", 3],
        data_saver: true,
        preferred_groups: ["A", 1],
      }),
    ).toEqual({ content_ratings: ["erotica"], data_saver: true, preferred_groups: ["A"] });
  });
});

describe("toggles", () => {
  it("ratings stay in MangaDex order and never empty", () => {
    expect(toggleRating(["suggestive"], "safe")).toEqual(["safe", "suggestive"]);
    expect(toggleRating(["safe"], "safe")).toEqual(["safe"]);
  });

  it("languages keep pick order and never empty", () => {
    expect(toggleLanguage(["en"], "es-la")).toEqual(["en", "es-la"]);
    expect(toggleLanguage(["en", "es-la"], "en")).toEqual(["es-la"]);
    expect(toggleLanguage(["en"], "en")).toEqual(["en"]);
  });

  it("groups split on commas", () => {
    expect(parseGroups(" A , B ,, c ")).toEqual(["A", "B", "c"]);
  });
});
