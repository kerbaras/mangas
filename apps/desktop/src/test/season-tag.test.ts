/// <reference types="vite/client" />
import { describe, expect, it } from "vitest";
import type { Chapter } from "@/lib/ipc";
import { chapterTag, seasonTag } from "@/lib/season-tag";
import fixture from "../../../../packages/core/tests/fixtures/season_tags.json?raw";

const chapter = (number: string, title: string | null): Chapter => ({
  id: `c${number}`,
  manga_id: "m",
  title,
  language: "en",
  volume: null,
  number,
  local_path: null,
});

describe("seasonTag", () => {
  it("agrees with the core's domain::season_tag fixture", () => {
    const cases = JSON.parse(fixture) as { title: string; tag: string | null }[];
    expect(cases.length).toBeGreaterThan(5);
    for (const c of cases) expect(seasonTag(chapter("1", c.title)), c.title).toBe(c.tag);
  });

  it("falls back to the chapter number", () => {
    expect(chapterTag(chapter("79", "S2 Prologue"))).toBe("S2 Prologue");
    expect(chapterTag(chapter("12", "The Return"))).toBe("ch 12");
  });
});
