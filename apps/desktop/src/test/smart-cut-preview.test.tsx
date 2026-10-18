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
};

function open() {
  const cut = mockCutPreview();
  const cancelCutPreview = vi.fn(async (_panel: string) => {});
  const previewCut = vi.fn(cut.previewCut);
  const setExportPreset = vi.fn(cut.setExportPreset);
  setBackend({ ...cut, cancelCutPreview, previewCut, setExportPreset } as unknown as Backend);
  render(
    <SmartCutButton
      series={{ source: "webtoons", id: "w", title: "Tower" }}
      chapters={[{ id: "c1", label: "Chapter 1", dir: "/d/1" }]}
      device={device}
      format="epub"
      cutPreset="balanced"
      detectors={false}
      saved={null}
      onSaved={() => {}}
    />,
  );
  fireEvent.click(screen.getByRole("button", { name: /Smart-cut options/ }));
  return { cancelCutPreview, previewCut, setExportPreset };
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
