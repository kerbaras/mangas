import { describe, expect, it } from "vitest";
import { nextActive } from "./motion";

describe("nextActive", () => {
  it("picks the step that became visible", () => {
    expect(nextActive(0, [{ index: 2, visible: true }])).toBe(2);
  });

  it("keeps the current step when one only leaves", () => {
    expect(nextActive(2, [{ index: 2, visible: false }])).toBe(2);
  });

  it("prefers the later step when two report at once", () => {
    const hits = [
      { index: 1, visible: true },
      { index: 3, visible: true },
      { index: 0, visible: false },
    ];
    expect(nextActive(0, hits)).toBe(3);
  });

  it("ignores targets that are not steps", () => {
    expect(nextActive(1, [{ index: -1, visible: true }])).toBe(1);
  });

  it("keeps the current step with no hits", () => {
    expect(nextActive(3, [])).toBe(3);
  });
});
