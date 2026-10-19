import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { DetectorSupportSettings } from "@/components/detector-support-settings";
import { StoragePanel } from "@/components/storage-panel";
import { type Backend, setBackend } from "@/lib/ipc";
import type { StorageUsage } from "@/lib/ipc.storage";
import { detectorState } from "./export-backend";

const MODELS = 157_000_000;

/** A backend whose sizes drop once `removeDetectors` has run, like the real one. */
function backend() {
  let models = MODELS;
  const usage = (): StorageUsage => ({
    data_dir: "/data",
    chapters: 0,
    models,
    covers: 0,
    library: 194_000,
    logs: 0,
    titles: [],
    partial_move: null,
  });
  const storageUsage = vi.fn(async () => usage());
  const detectorsStatus = vi.fn(async () =>
    models
      ? detectorState({ diskBytes: models })
      : detectorState({
          downloaded: false,
          runtime: { state: "not_downloaded", version: "1.28.3" },
          downloadBytes: 190_000_000,
          diskBytes: 0,
        }),
  );
  const removeDetectors = vi.fn(async () => {
    const freed = models;
    models = 0;
    return freed;
  });
  return { storageUsage, detectorsStatus, removeDetectors };
}

const REASON = "onnx runtime /models/onnxruntime/libonnxruntime.so: invalid ELF header";

/** A runtime that failed to load until Storage → Delete detector support empties the folder. */
function brokenRuntime() {
  let models = MODELS;
  const storageUsage = vi.fn(
    async (): Promise<StorageUsage> => ({
      data_dir: "/data",
      chapters: 0,
      models,
      covers: 0,
      library: 194_000,
      logs: 0,
      titles: [],
    }),
  );
  const detectorsStatus = vi.fn(async () =>
    models
      ? detectorState({
          downloaded: false,
          runtime: { state: "failed", version: "1.28.3", reason: REASON },
          diskBytes: models,
        })
      : detectorState({
          downloaded: false,
          runtime: { state: "not_downloaded", version: "1.28.3" },
          downloadBytes: 190_000_000,
          diskBytes: 0,
        }),
  );
  const clearCache = vi.fn(async () => {
    const bytes = models;
    models = 0;
    return { files: 3, bytes };
  });
  return { storageUsage, detectorsStatus, clearCache };
}

afterEach(() => setBackend(null));

describe("Detector support after a delete (#170, #211)", () => {
  it("measures Storage and the detector row again without a focus change", async () => {
    const b = backend();
    setBackend(b as unknown as Backend);
    render(
      <>
        <StoragePanel />
        <DetectorSupportSettings />
      </>,
    );
    expect(
      await screen.findByRole("button", { name: /Delete detector support\s*157 MB/ }),
    ).toBeTruthy();
    fireEvent.click(await screen.findByRole("button", { name: /Remove detector support/ }));

    expect(
      await screen.findByRole("button", { name: /Delete detector support\s*0 B/ }),
    ).toBeTruthy();
    expect(await screen.findByRole("button", { name: /Download detector support/ })).toBeTruthy();
    expect(b.removeDetectors).toHaveBeenCalledOnce();
    expect(b.storageUsage).toHaveBeenCalledTimes(2);
    expect(b.detectorsStatus).toHaveBeenCalledTimes(2);
  });

  it("Storage → Delete detector support drops the old runtime error (#211)", async () => {
    const b = brokenRuntime();
    setBackend(b as unknown as Backend);
    render(
      <>
        <StoragePanel />
        <DetectorSupportSettings />
      </>,
    );
    expect(await screen.findByText(new RegExp(`${REASON}\\. Exports still work`))).toBeTruthy();
    fireEvent.click(
      await screen.findByRole("button", { name: /Delete detector support\s*157 MB/ }),
    );

    expect(await screen.findByRole("button", { name: /Download detector support/ })).toBeTruthy();
    expect(screen.queryByText(new RegExp(REASON))).toBeNull();
    expect(screen.queryByRole("button", { name: /Try again/ })).toBeNull();
    expect(screen.getByText(/Download them to turn this on/)).toBeTruthy();
    expect(b.clearCache).toHaveBeenCalledWith("models");
    expect(b.detectorsStatus).toHaveBeenCalledTimes(2);
  });
});
