import { describe, expect, it } from "vitest";
import { cn } from "@/lib/cn";

describe("cn", () => {
  it("keeps a text colour next to a type-ramp size", () => {
    expect(cn("text-white", "text-caption")).toBe("text-white text-caption");
  });

  it("still lets a later size replace an earlier one", () => {
    expect(cn("text-body", "text-caption")).toBe("text-caption");
  });
});
