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

afterEach(() => setBackend(null));

describe("Remove detector support in Settings (#170)", () => {
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
});
