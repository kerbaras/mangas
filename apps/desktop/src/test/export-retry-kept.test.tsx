import { act, fireEvent, render, renderHook, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ExportDialog } from "@/components/export-dialog";
import { TooltipProvider } from "@/components/ui/tooltip";
import { retryOf, useExport } from "@/hooks/use-export";
import { ExportStopped } from "@/lib/export-stopped";
import { type ExportResult, setBackend } from "@/lib/ipc";
import { chapters, enabledButton, exportBackend, solo } from "./export-backend";

const stats = (title: string) => ({
  pages: 6,
  strips: 0,
  source_rows: 0,
  pagination_ms: 0,
  filters_ms: 0,
  skipped: 0,
  output: `/k/${title}.epub`,
});
const stoppedAt = (written: string[], failed: string, rest: string[] = []) =>
  new ExportStopped({
    written: written.map(stats),
    failed: { title: failed, output: `/k/${failed}.epub`, reason: "io: bad page" },
    not_written: rest.map((t) => ({ title: t, output: `/k/${t}.epub` })),
  });
const book = (title: string): ExportResult => ({
  output: `/k/${title}.epub`,
  parts: [],
  pages: 6,
  skipped: 0,
});
const outputs = (results: ExportResult[] = []) => results.map((r) => r.output);

afterEach(() => setBackend(null));

describe("Try again after a stopped export (#337)", () => {
  it("useExport: retries only the failed and later books; the kept ones carry over", async () => {
    const { result } = renderHook(() => useExport());
    const stop = (e: ExportStopped) => () => Promise.reject(e);
    await act(() => result.current.run(stop(stoppedAt(["Solo v1"], "Solo v2", ["Solo v3"]))));
    let retry = retryOf(result.current.state);
    expect(retry.skip).toBe(1);
    expect(outputs(retry.kept)).toEqual(["/k/Solo v1.epub"]);

    await act(() => result.current.run(stop(stoppedAt(["Solo v2"], "Solo v3"))));
    retry = retryOf(result.current.state);
    expect(retry.skip).toBe(2);
    expect(outputs(retry.kept)).toEqual(["/k/Solo v1.epub", "/k/Solo v2.epub"]);
    expect(result.current.settled).toBe(0);

    let written: ExportResult[] | null = null;
    await act(async () => {
      written = await result.current.run(async () => [book("Solo v3")]);
    });
    expect(written).toEqual([book("Solo v3")]);
    const done = result.current.state;
    expect(done.kind === "done" && outputs(done.kept)).toEqual([
      "/k/Solo v1.epub",
      "/k/Solo v2.epub",
    ]);
    expect(retryOf(done)).toEqual({ skip: 0, kept: [] });
    expect(result.current.settled).toBe(1);
  });

  it("useExport: a plain failure between tries keeps what the stopped run wrote", async () => {
    const { result } = renderHook(() => useExport());
    await act(() => result.current.run(() => Promise.reject(stoppedAt(["Solo v1"], "Solo v2"))));
    await act(() => result.current.run(() => Promise.reject(new Error("io: disk full"))));
    expect(result.current.state.kind).toBe("error");
    const retry = retryOf(result.current.state);
    expect(retry.skip).toBe(1);
    expect(outputs(retry.kept)).toEqual(["/k/Solo v1.epub"]);
    expect(result.current.settled).toBe(0);
  });

  it.each([false, true])(
    "dialog (replace %s): Try again writes only the unwritten book and says the kept one is there",
    async (replace) => {
      const exportVolumes = vi
        .fn()
        .mockRejectedValueOnce(stoppedAt(["Solo v1"], "Solo v2"))
        .mockResolvedValueOnce([book("Solo v2")]);
      exportBackend({ exportVolumes });
      render(
        <TooltipProvider>
          <ExportDialog open onOpenChange={() => {}} picks={[{ manga: solo, chapters }]} />
        </TooltipProvider>,
      );
      if (replace) fireEvent.click(await screen.findByRole("switch", { name: /Replace/ }));
      fireEvent.click(await enabledButton(/^Export 2 books/));
      expect(await screen.findByText("Export stopped")).toBeTruthy();
      expect(exportVolumes.mock.calls[0][0]).toMatchObject({ skip: 0, replace });
      fireEvent.click(await enabledButton(/^Try again$/));
      expect(
        await screen.findByText(/Already there from the last try: Solo v1\.epub/),
      ).toBeTruthy();
      expect(exportVolumes).toHaveBeenCalledTimes(2);
      expect(exportVolumes.mock.calls[1][0]).toMatchObject({ skip: 1, replace });
    },
  );

  it.each([
    [false, ["/k/Solo v1.epub", "/k/Solo v2.epub"]],
    [true, ["/k/Solo v1.epub", "/k/Solo v2.epub"]],
  ])(
    "USB (Kindle at the first try %s): every kept book reaches the Kindle once",
    async (plugged, copied) => {
      const kindle = { name: "Kindle", root: "/media/k", documents: "/media/k/documents" };
      const findKindles = vi.fn(async () => [kindle]);
      if (!plugged) findKindles.mockResolvedValueOnce([]);
      const sendUsb = vi.fn(async (file: string) => ({ path: file, already_there: false }));
      const exportVolumes = vi
        .fn()
        .mockRejectedValueOnce(stoppedAt(["Solo v1"], "Solo v2"))
        .mockResolvedValueOnce([book("Solo v2")]);
      exportBackend({ exportVolumes, findKindles, sendUsb });
      render(
        <TooltipProvider>
          <ExportDialog open onOpenChange={() => {}} picks={[{ manga: solo, chapters }]} />
        </TooltipProvider>,
      );
      fireEvent.mouseDown(await screen.findByRole("tab", { name: "Kindle over USB" }));
      fireEvent.click(await enabledButton(/Export 2 books and copy/));
      expect(await screen.findByText("Export stopped")).toBeTruthy();
      await waitFor(() => expect(findKindles).toHaveBeenCalledTimes(1));
      fireEvent.click(await enabledButton(/^Try again$/));
      await waitFor(() => expect(sendUsb.mock.calls.map((c) => c[0])).toEqual(copied));
    },
  );
});
