import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { DetectorSupportSettings } from "@/components/detector-support-settings";
import { type Backend, setBackend } from "@/lib/ipc";
import type { DetectorsStatus } from "@/lib/ipc.detectors";
import { detectorState } from "./export-backend";

const LEFT = 36_000_000;

/** One model file the remove couldn't delete: not downloaded, but 36 MB still on disk. */
const halfRemoved = (o: Partial<DetectorsStatus> = {}) =>
  detectorState({
    downloaded: false,
    runtime: { state: "not_downloaded", version: "1.28.3" },
    downloadBytes: 154_000_000,
    diskBytes: LEFT,
    ...o,
  });

const empty = detectorState({
  downloaded: false,
  runtime: { state: "not_downloaded", version: "1.28.3" },
  downloadBytes: 190_000_000,
  diskBytes: 0,
});

function backend(status: DetectorsStatus) {
  let left = status.diskBytes;
  const clearCache = vi.fn(async () => {
    const bytes = left;
    left = 0;
    return { files: 1, bytes };
  });
  const removeDetectors = vi.fn(async () => 0);
  setBackend({
    detectorsStatus: async () => (left ? status : empty),
    clearCache,
    removeDetectors,
    downloadDetectors: async () => detectorState(),
    cancelDetectorsDownload: async () => {},
    onDetectorsProgress: async () => () => {},
    getSettings: async () => ({}),
  } as unknown as Backend);
  return { clearCache, removeDetectors };
}

afterEach(() => setBackend(null));

describe("detector card after a remove that left files behind", () => {
  it("shows the size left and removes it through the Storage delete path", async () => {
    const b = backend(halfRemoved());
    render(<DetectorSupportSettings />);
    expect(await screen.findByText(/^36(\.0)? MB of detector files left on disk$/)).toBeTruthy();
    expect(screen.getByRole("button", { name: /Download detector support/ })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Remove" }));
    await waitFor(() => expect(b.clearCache).toHaveBeenCalledWith("models"));
    expect(b.removeDetectors).not.toHaveBeenCalled();
    await waitFor(() => expect(screen.queryByText(/left on disk/)).toBeNull());
    expect(screen.queryByRole("button", { name: "Remove" })).toBeNull();
  });

  it("offers it next to a runtime that failed to load, too", async () => {
    backend(halfRemoved({ runtime: { state: "failed", version: "1.28.3", reason: "bad ELF" } }));
    render(<DetectorSupportSettings />);
    expect(await screen.findByText(/bad ELF/)).toBeTruthy();
    expect(screen.getByRole("button", { name: "Remove" })).toBeTruthy();
  });

  it("shows nothing extra when the folder is empty", async () => {
    backend(empty);
    render(<DetectorSupportSettings />);
    expect(await screen.findByRole("button", { name: /Download detector support/ })).toBeTruthy();
    expect(screen.queryByText(/left on disk/)).toBeNull();
  });
});
