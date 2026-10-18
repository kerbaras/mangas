import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ExportDialog } from "@/components/export-dialog";
import { TooltipProvider } from "@/components/ui/tooltip";
import { clearFailure, downloadState } from "@/lib/detector-download";
import { type DetectorsStatus, setBackend } from "@/lib/ipc";
import type { DetectorsProgress } from "@/lib/ipc.detectors";
import { writeSettings } from "@/lib/settings";
import { chapters, detectorState, enabledButton, exportBackend, solo } from "./export-backend";

afterEach(async () => {
  await waitFor(() => expect(downloadState().downloading).toBe(false));
  clearFailure();
  setBackend(null);
  writeSettings({ defaultFormat: null, defaultDevice: "kindle-paperwhite12" });
});

const dialog = () => (
  <TooltipProvider>
    <ExportDialog open onOpenChange={() => {}} picks={[{ manga: solo, chapters }]} />
  </TooltipProvider>
);

const exportButton = () => enabledButton(/Export 2 books/);

describe("export dialog: detector support", () => {
  it("turns the detectors on when the build has them and the models are on disk", async () => {
    const { exportVolumes } = exportBackend();
    render(dialog());
    const toggle = await screen.findByRole("switch", { name: "Use detectors" });
    await waitFor(() => expect(toggle.getAttribute("data-state")).toBe("checked"));
    expect(toggle.hasAttribute("disabled")).toBe(false);
    fireEvent.click(await exportButton());
    await waitFor(() => expect(exportVolumes).toHaveBeenCalledOnce());
    expect(exportVolumes.mock.calls[0][0].detectors).toBe(true);
  });

  it("disables the detectors when the build has no detector support, models or not", async () => {
    const { exportVolumes } = exportBackend(
      {},
      detectorState({ available: false, runtime: { state: "unavailable" } }),
    );
    render(dialog());
    expect(await screen.findByText(/This build has no detector support/)).toBeTruthy();
    const toggle = screen.getByRole("switch", { name: "Use detectors" });
    expect(toggle.hasAttribute("disabled")).toBe(true);
    expect(toggle.getAttribute("data-state")).toBe("unchecked");
    fireEvent.click(await exportButton());
    await waitFor(() => expect(exportVolumes).toHaveBeenCalledOnce());
    expect(exportVolumes.mock.calls[0][0].detectors).toBe(false);
  });

  it("downloads detector support from the dialog and turns the switch on without a restart", async () => {
    const missing = detectorState({
      downloaded: false,
      runtime: { state: "not_downloaded", version: "1.28.3" },
      downloadBytes: 189_714_187,
      diskBytes: 0,
    });
    const events: { progress?: (p: DetectorsProgress) => void } = {};
    let current = missing;
    let finish: () => void = () => {};
    const downloadDetectors = vi.fn(
      () =>
        new Promise<DetectorsStatus>((resolve) => {
          finish = () => {
            current = detectorState();
            resolve(current);
          };
        }),
    );
    exportBackend(
      {
        downloadDetectors,
        detectorsStatus: async () => current,
        onDetectorsProgress: async (cb) => {
          events.progress = cb;
          return () => {};
        },
      },
      missing,
    );
    render(dialog());
    expect(await screen.findByText(/Download them to turn this on/)).toBeTruthy();
    const toggle = screen.getByRole("switch", { name: "Use detectors" });
    expect(toggle.hasAttribute("disabled")).toBe(true);
    fireEvent.click(screen.getByRole("button", { name: /Download detector support \(~190 MB\)/ }));
    await waitFor(() => expect(downloadDetectors).toHaveBeenCalledOnce());
    events.progress?.({ item: "onnxruntime", done: 4_000_000, total: 9_130_098 });
    expect(await screen.findByText("ONNX Runtime: 4.00 MB of 9.13 MB")).toBeTruthy();
    expect(screen.getAllByRole("button", { name: /Cancel/ }).length).toBeGreaterThan(1);
    finish();
    await waitFor(() => expect(toggle.getAttribute("data-state")).toBe("checked"));
    expect(toggle.hasAttribute("disabled")).toBe(false);
  });

  it("shows the running download at once, so Download cannot be clicked twice", async () => {
    const missing = detectorState({
      downloaded: false,
      runtime: { state: "not_downloaded", version: "1.28.3" },
      downloadBytes: 1000,
    });
    let listen: () => void = () => {};
    const downloadDetectors = vi.fn(async () => detectorState());
    exportBackend(
      {
        downloadDetectors,
        onDetectorsProgress: () =>
          new Promise<() => void>((resolve) => {
            listen = () => resolve(() => {});
          }),
      },
      missing,
    );
    render(dialog());
    fireEvent.click(await screen.findByRole("button", { name: /Download detector support/ }));
    await waitFor(() =>
      expect(screen.queryByRole("button", { name: /Download detector support/ })).toBeNull(),
    );
    listen();
    await waitFor(() => expect(downloadDetectors).toHaveBeenCalledOnce());
  });

  it("still shows a running download after the dialog is closed and reopened", async () => {
    const missing = detectorState({
      downloaded: false,
      runtime: { state: "not_downloaded", version: "1.28.3" },
      downloadBytes: 1000,
    });
    let finish: () => void = () => {};
    const downloadDetectors = vi.fn(
      () => new Promise<DetectorsStatus>((resolve) => (finish = () => resolve(detectorState()))),
    );
    exportBackend({ downloadDetectors }, missing);
    const first = render(dialog());
    fireEvent.click(await screen.findByRole("button", { name: /Download detector support/ }));
    await waitFor(() => expect(downloadDetectors).toHaveBeenCalledOnce());
    first.unmount();
    render(dialog());
    expect(await screen.findByText("Starting download…")).toBeTruthy();
    expect(screen.queryByRole("button", { name: /Download detector support/ })).toBeNull();
    finish();
  });

  it("never starts the download when Cancel is clicked before it begins", async () => {
    const missing = detectorState({
      downloaded: false,
      runtime: { state: "not_downloaded", version: "1.28.3" },
      downloadBytes: 1000,
    });
    let listen: () => void = () => {};
    const downloadDetectors = vi.fn(async () => detectorState());
    exportBackend(
      {
        downloadDetectors,
        onDetectorsProgress: () =>
          new Promise<() => void>((resolve) => {
            listen = () => resolve(() => {});
          }),
      },
      missing,
    );
    render(dialog());
    fireEvent.click(await screen.findByRole("button", { name: /Download detector support/ }));
    fireEvent.click((await screen.findAllByRole("button", { name: /Cancel/ }))[0]);
    listen();
    expect(await screen.findByRole("button", { name: /Download detector support/ })).toBeTruthy();
    expect(downloadDetectors).not.toHaveBeenCalled();
  });

  it("keeps exporting without detectors when the download fails, with Retry", async () => {
    const missing = detectorState({
      downloaded: false,
      runtime: { state: "not_downloaded", version: "1.28.3" },
      downloadBytes: 1000,
    });
    const downloadDetectors = vi.fn(async () => {
      throw new Error("network: connection reset");
    });
    const { exportVolumes } = exportBackend({ downloadDetectors }, missing);
    render(dialog());
    fireEvent.click(await screen.findByRole("button", { name: /Download detector support/ }));
    expect(await screen.findByText(/network: connection reset/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /Try again/ }));
    await waitFor(() => expect(downloadDetectors).toHaveBeenCalledTimes(2));
    fireEvent.click(await exportButton());
    await waitFor(() => expect(exportVolumes).toHaveBeenCalledOnce());
    expect(exportVolumes.mock.calls[0][0].detectors).toBe(false);
  });
});
