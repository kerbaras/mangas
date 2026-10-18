import { renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useClearOnChange } from "@/hooks/use-clear-on-change";

describe("useClearOnChange", () => {
  it("clears when the choice changes, and only then", () => {
    const clear = vi.fn();
    const { rerender } = renderHook(({ choice }) => useClearOnChange(choice, false, clear), {
      initialProps: { choice: "epub" },
    });
    rerender({ choice: "epub" });
    expect(clear).not.toHaveBeenCalled();
    rerender({ choice: "pdf" });
    expect(clear).toHaveBeenCalledOnce();
  });

  it("clears a change made mid-export once the export is done", () => {
    const clear = vi.fn();
    const { rerender } = renderHook(({ choice, busy }) => useClearOnChange(choice, busy, clear), {
      initialProps: { choice: "epub", busy: true },
    });
    rerender({ choice: "pdf", busy: true });
    expect(clear).not.toHaveBeenCalled();
    rerender({ choice: "pdf", busy: false });
    expect(clear).toHaveBeenCalledOnce();
  });
});
