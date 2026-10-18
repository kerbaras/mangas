import { describe, expect, it } from "vitest";
import { languageLabel, languageList } from "@/lib/format";

describe("languageLabel", () => {
  it("names MangaDex's -la and -ro suffixes instead of reading them as regions", () => {
    expect(languageLabel("es-la")).toBe("Spanish (Latin America)");
    expect(languageLabel("ES-LA")).toBe("Spanish (Latin America)");
    expect(languageLabel("ja-ro")).toBe("Japanese (Romanized)");
  });

  it("uses the platform names for plain codes and real regions", () => {
    expect(languageLabel("en")).toBe("English");
    expect(languageLabel("pt-br")).toBe("Brazilian Portuguese");
    expect(languageLabel("zz-not-a-code!")).toBe("zz-not-a-code!");
  });
});

describe("languageList", () => {
  it("joins language names with 'or'", () => {
    expect(languageList(["es-la"])).toBe("Spanish (Latin America)");
    expect(languageList(["es-la", "en"])).toBe("Spanish (Latin America) or English");
    expect(languageList(["en", "fr", "es"])).toBe("English, French, or Spanish");
  });
});
