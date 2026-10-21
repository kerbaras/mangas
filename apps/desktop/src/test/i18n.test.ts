import { describe, expect, it } from "vitest";
import { messagesFor } from "@/i18n/catalog";
import { common } from "@/i18n/en/common";
import { library } from "@/i18n/en/library";
import { mangaDetails } from "@/i18n/en/manga-details";
import { search } from "@/i18n/en/search";
import { translate } from "@/i18n/translate";

describe("translate", () => {
  const messages = {
    "a.plain": "Library",
    "a.vars": "Couldn't reach {name}: {reason}",
    "a.count": "{count} chapters",
    "a.count_one": "{count} chapter",
  };

  it("returns the text, with each {name} filled", () => {
    expect(translate(messages, "a.plain")).toBe("Library");
    expect(translate(messages, "a.vars", { name: "MangaDex", reason: "timed out" })).toBe(
      "Couldn't reach MangaDex: timed out",
    );
  });

  it("leaves a placeholder it has no value for, and the key for a missing message", () => {
    expect(translate(messages, "a.vars", { name: "MangaDex" })).toBe(
      "Couldn't reach MangaDex: {reason}",
    );
    expect(translate(messages, "a.nope")).toBe("a.nope");
  });

  it("picks the _one form only for a count of 1", () => {
    expect(translate(messages, "a.count", { count: 1 })).toBe("1 chapter");
    expect(translate(messages, "a.count", { count: 0 })).toBe("0 chapters");
    expect(translate(messages, "a.count", { count: 3 })).toBe("3 chapters");
    expect(translate(messages, "a.plain", { count: 1 })).toBe("Library");
  });
});

describe("English catalog", () => {
  const parts = [common, library, mangaDetails, search];

  it("has every screen's messages, none overriding another's", () => {
    const keys = parts.flatMap((p) => Object.keys(p));
    expect(new Set(keys).size).toBe(keys.length);
    expect(Object.keys(messagesFor("en"))).toHaveLength(keys.length);
  });

  it("keys each message by its screen and has no blank text", () => {
    for (const [key, text] of Object.entries(messagesFor("en"))) {
      expect(key).toMatch(/^[a-z]+\.[A-Za-z]+(_one)?$/);
      expect(text.trim()).not.toBe("");
    }
  });
});
