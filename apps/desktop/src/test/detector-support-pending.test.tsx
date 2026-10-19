import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { DetectorSupportActions } from "@/components/detector-support";
import type { DetectorSupport } from "@/hooks/use-detector-support";
import { detectorState } from "./export-backend";

const pending = (
  remove: () => Promise<number>,
  failure: string | null = null,
): DetectorSupport => ({
  status: {
    kind: "ready",
    data: detectorState({
      downloaded: false,
      runtime: { state: "remove_pending", version: "1.28.3", bytes: 23_000_000 },
      downloadBytes: 180_584_089,
      diskBytes: 23_000_000,
    }),
  },
  progress: null,
  downloading: false,
  failure,
  download: vi.fn(async () => {}),
  cancel: vi.fn(),
  remove,
  removeLeftovers: vi.fn(async () => 0),
});

const notice = /Detector models removed; the runtime \(.+\) is still in use — restart Mangas/;

describe("detector support after a partial remove", () => {
  it("says the runtime is still in use and offers Remove again in Settings", () => {
    const remove = vi.fn(async () => 0);
    render(<DetectorSupportActions support={pending(remove)} manage />);
    expect(screen.getByText(notice)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /Remove again/ }));
    expect(remove).toHaveBeenCalledOnce();
  });

  it("says so in the export dialog too, next to Download", () => {
    render(<DetectorSupportActions support={pending(vi.fn(async () => 0))} />);
    expect(screen.getByText(notice)).toBeTruthy();
    expect(screen.getByRole("button", { name: /Download detector support/ })).toBeTruthy();
    expect(screen.queryByRole("button", { name: /Remove again/ })).toBeNull();
  });

  it("keeps the notice and Remove again after a failed download", () => {
    const remove = vi.fn(async () => 0);
    render(<DetectorSupportActions support={pending(remove, "network: timed out")} manage />);
    expect(screen.getByText(/network: timed out/)).toBeTruthy();
    expect(screen.getByText(notice)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /Remove again/ }));
    expect(remove).toHaveBeenCalledOnce();
  });
});
