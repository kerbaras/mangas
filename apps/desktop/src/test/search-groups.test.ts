import { describe, expect, it } from "vitest";
import type { Manga } from "@/lib/ipc";
import { chipOf, type GroupState, PREVIEW, shown, summary } from "@/lib/search-groups";

const manga = (id: string): Manga => ({
  id,
  source: "a",
  title: id,
  description: "",
  cover_url: null,
  status: "ongoing",
});
const ready = (n: number): GroupState => ({
  kind: "done",
  group: {
    provider: "a",
    status: { state: "ready" },
    items: [...Array(n)].map((_, i) => manga(`${i}`)),
  },
});
const gated: GroupState = {
  kind: "done",
  group: { provider: "b", status: { state: "needs_session" }, items: [] },
};
const searching: GroupState = { kind: "searching" };

describe("search groups", () => {
  it("chips show a spinner, a count or a warning", () => {
    expect(chipOf(undefined)).toEqual({ count: null, searching: false, problem: false });
    expect(chipOf(searching).searching).toBe(true);
    expect(chipOf(ready(3)).count).toBe(3);
    expect(chipOf(gated)).toEqual({ count: null, searching: false, problem: true });
  });

  it("summarises progress, totals and failures", () => {
    expect(summary([searching, searching])).toBe("Searching 2 providers…");
    expect(summary([ready(2), searching])).toBe("2 results so far, 1 still searching…");
    expect(summary([ready(1), ready(0)])).toBe("1 result from 2 providers");
    expect(summary([ready(5), gated, undefined])).toBe(
      "5 results from 1 provider · 1 couldn't search",
    );
  });

  it("previews groups in the all view", () => {
    const items = [...Array(PREVIEW + 3).keys()];
    expect(shown(items, false)).toHaveLength(PREVIEW);
    expect(shown(items, true)).toHaveLength(PREVIEW + 3);
  });
});
