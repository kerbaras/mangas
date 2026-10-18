import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ExportDialog } from "@/components/export-dialog";
import { TooltipProvider } from "@/components/ui/tooltip";
import { type DetectorsStatus, setBackend } from "@/lib/ipc";
import type { DetectorsProgress } from "@/lib/ipc.detectors";
import { writeSettings } from "@/lib/settings";
import { chapters, detectorState, enabledButton, exportBackend, solo } from "./export-backend";

afterEach(() => {
  setBackend(null);
  writeSettings({ defaultFormat: null, defaultDevice: "kindle-paperwhite12" });
});

const dialog = () => (
  <TooltipProvider>
    <ExportDialog open onOpenChange={() => {}} picks={[{ manga: solo, chapters }]} />
  </TooltipProvider>
);

const exportButton = () => enabledButton(/Export 2 books/);

describe("export dialog: format, smart cut and delivery", () => {
  it("writes the picked format and names the files with its extension", async () => {
    const { exportVolumes } = exportBackend();
    render(dialog());
    fireEvent.click(await screen.findByLabelText("CBZ"));
    expect(await screen.findByText("Solo v1.cbz")).toBeTruthy();
    fireEvent.click(await exportButton());
    await waitFor(() => expect(exportVolumes).toHaveBeenCalledOnce());
    expect(exportVolumes.mock.calls[0][0].format).toBe("cbz");
  });

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

  it("copies every book to the Kindle over USB and says when one is already there", async () => {
    const kindle = {
      name: "Kindle",
      root: "/media/me/Kindle",
      documents: "/media/me/Kindle/documents",
    };
    const sendUsb = vi
      .fn()
      .mockResolvedValueOnce({
        path: "/media/me/Kindle/documents/Solo v1.epub",
        already_there: true,
      })
      .mockResolvedValueOnce({
        path: "/media/me/Kindle/documents/Solo v2.epub",
        already_there: false,
      });
    exportBackend({ findKindles: async () => [kindle], sendUsb });
    render(dialog());
    fireEvent.mouseDown(await screen.findByRole("tab", { name: "Kindle over USB" }));
    fireEvent.click(await enabledButton(/Export 2 books and copy/));
    await waitFor(() => expect(sendUsb).toHaveBeenCalledTimes(2));
    expect(sendUsb).toHaveBeenCalledWith("/k/Solo v1.epub", "Solo v1", kindle.root);
    expect(await screen.findByText(/Already on Kindle: .*Solo v1\.epub/)).toBeTruthy();
    expect(screen.getByText(/Copied to Kindle: .*Solo v2\.epub/)).toBeTruthy();
  });

  it("offers email only for formats Send to Kindle takes", async () => {
    exportBackend();
    writeSettings({ defaultFormat: "cbz" });
    render(dialog());
    const email = await screen.findByRole("tab", { name: "Email to Kindle" });
    expect(email.hasAttribute("disabled")).toBe(true);
    const parent = email.parentElement;
    if (!parent) throw new Error("the disabled tab has no tooltip trigger");
    fireEvent.focus(parent);
    const tip = await screen.findByRole("tooltip");
    expect(tip.textContent).toMatch(/takes EPUB or PDF, not a CBZ/);
  });
});
