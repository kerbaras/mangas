import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { storageChanged, useStorageChanged } from "@/lib/storage-changed";

describe("storage changed", () => {
  it("reloads every mounted view, and stops after unmount", () => {
    const panel = vi.fn();
    const detectors = vi.fn();
    const a = renderHook(() => useStorageChanged(panel));
    renderHook(() => useStorageChanged(detectors));
    act(() => storageChanged());
    expect(panel).toHaveBeenCalledOnce();
    expect(detectors).toHaveBeenCalledOnce();
    a.unmount();
    act(() => storageChanged());
    expect(panel).toHaveBeenCalledOnce();
    expect(detectors).toHaveBeenCalledTimes(2);
  });
});
