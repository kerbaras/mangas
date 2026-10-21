import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { toast } from "sonner";
import { afterEach, describe, expect, it, vi } from "vitest";
import { DetectorSupportSettings } from "@/components/detector-support-settings";
import { type Backend, setBackend } from "@/lib/ipc";
import { detectorState } from "./export-backend";

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { success: vi.fn(), error: vi.fn(), warning: vi.fn() }),
}));

afterEach(() => {
  setBackend(null);
  vi.clearAllMocks();
});

/** Remove fails once (an immutable model file), then works. */
function backend() {
  let attempts = 0;
  let models = 157_000_000;
  const removeDetectors = vi.fn(async () => {
    attempts += 1;
    if (attempts === 1) throw new Error("io: Operation not permitted (os error 1)");
    const freed = models;
    models = 0;
    return freed;
  });
  const detectorsStatus = vi.fn(async () =>
    models
      ? detectorState({ diskBytes: models })
      : detectorState({ downloaded: false, downloadBytes: 190_000_000, diskBytes: 0 }),
  );
  return { removeDetectors, detectorsStatus };
}

describe("detector Remove toasts (#286)", () => {
  it("a later Remove replaces the earlier failure toast", async () => {
    setBackend(backend() as unknown as Backend);
    render(<DetectorSupportSettings />);
    const remove = await screen.findByRole("button", { name: /Remove detector support/ });

    fireEvent.click(remove);
    await waitFor(() => expect(toast.error).toHaveBeenCalledOnce());
    const [, failed] = vi.mocked(toast.error).mock.calls[0] ?? [];

    fireEvent.click(await screen.findByRole("button", { name: /Remove detector support/ }));
    await waitFor(() => expect(toast.success).toHaveBeenCalledOnce());
    const [title, removed] = vi.mocked(toast.success).mock.calls[0] ?? [];

    expect(title).toBe("Detector support removed");
    expect(failed?.id).toBe("detector-remove");
    expect(removed).toMatchObject({ id: "detector-remove", action: null });
  });
});
