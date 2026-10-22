import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SmartCutButton } from "@/components/smart-cut-preview";
import { type Backend, setBackend } from "@/lib/ipc";
import { mockCutPreview } from "@/lib/mock-cut-preview";

afterEach(() => setBackend(null));

const device = {
  id: "kpw",
  name: "Paperwhite",
  width: 1236,
  height: 1648,
  dpi: 300,
  grayscale: true,
  format: "epub" as const,
  delivery: null,
};

function open(detectorsError: string | null = null, broken: "runtime" | "model" = "runtime") {
  const cut = mockCutPreview();
  const cancelCutPreview = vi.fn(async (_panel: string) => {});
  // Like the backend: a preview asked with detectors records the failure in the detector status.
  let failed: string | null = null;
  const previewCut = vi.fn(async (...args: Parameters<typeof cut.previewCut>) => {
    if (args[2]) failed = detectorsError;
    return { ...(await cut.previewCut(...args)), detectors_error: args[2] ? detectorsError : null };
  });
  const detectorsStatus = vi.fn(async () => ({
    available: true,
    downloaded: failed === null,
    provider: "cpu",
    runtime:
      failed && broken === "runtime"
        ? { state: "failed", version: "1.28.3", reason: failed }
        : { state: "unavailable" },
    downloadBytes: 0,
    diskBytes: 0,
  }));
  const setExportPreset = vi.fn(cut.setExportPreset);
  setBackend({
    ...cut,
    cancelCutPreview,
    previewCut,
    setExportPreset,
    detectorsStatus,
  } as unknown as Backend);
  const button = (detectors: boolean) => (
    <SmartCutButton
      series={{ source: "webtoons", id: "w", title: "Tower" }}
      chapters={[{ id: "c1", label: "Chapter 1", dir: "/d/1" }]}
      device={device}
      format="epub"
      cutPreset="balanced"
      detectors={detectors}
      saved={null}
      onSaved={() => {}}
    />
  );
  const { rerender } = render(button(detectorsError !== null));
  fireEvent.click(screen.getByRole("button", { name: /Smart-cut options/ }));
  // The export dialog turns its switch off once the status says the detectors failed.
  const detectorsOff = () => rerender(button(false));
  return { cancelCutPreview, previewCut, setExportPreset, detectorsOff };
}

describe("smart-cut options panel", () => {
  it("names what Save stores and stores exactly that", async () => {
    const { setExportPreset } = open();
    fireEvent.click(
      await screen.findByRole("button", { name: "Save cut, device and format for this series" }),
    );
    await waitFor(() =>
      expect(setExportPreset).toHaveBeenCalledWith("webtoons", "w", {
        cut_preset: "balanced",
        format: "epub",
        profile: "kpw",
      }),
    );
    expect(await screen.findByText(/Saved cut settings, device and format/)).toBeTruthy();
  });

  it.each([
    [
      "runtime",
      "onnx runtime /models/runtime/1.28.3/libonnxruntime.dylib: dlopen failed: dlopen(…)",
    ],
    ["model", "model comic-text-and-bubble-detector: sha256 mismatch"],
  ] as const)(
    "shows why the detectors couldn't run (%s) and still shows the pages",
    async (broken, reason) => {
      const { previewCut, detectorsOff } = open(reason, broken);
      expect(await screen.findByText(`Cut without detectors: ${reason}`)).toBeTruthy();
      expect(screen.getAllByRole("img", { name: /Page \d+/ }).length).toBeGreaterThan(0);
      expect(previewCut.mock.calls[0][2]).toBe(true);
      detectorsOff();
      await waitFor(() => expect(previewCut).toHaveBeenCalledTimes(2));
      expect(previewCut.mock.calls[1][2]).toBe(false);
      await waitFor(() =>
        expect(screen.getAllByRole("img", { name: /Page \d+/ }).length).toBeGreaterThan(0),
      );
      expect(screen.getByText(`Cut without detectors: ${reason}`)).toBeTruthy();
    },
  );

  it("cancels the panel's previews when closed", async () => {
    const { cancelCutPreview, previewCut } = open();
    await screen.findByText("Cutting pages…");
    await waitFor(() => expect(previewCut).toHaveBeenCalled());
    const panel = previewCut.mock.calls[0][3];
    expect(panel).toMatch(/\w/);
    fireEvent.keyDown(document.activeElement ?? document.body, { key: "Escape" });
    await waitFor(() => expect(cancelCutPreview).toHaveBeenCalledWith(panel));
  });
});
