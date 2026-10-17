import { afterEach, describe, expect, it, vi } from "vitest";
import { exportSummary, notifyExported } from "@/lib/export-summary";
import { toExportResult } from "@/lib/ipc.wire";

const toast = vi.hoisted(() => ({ success: vi.fn(), warning: vi.fn() }));
vi.mock("sonner", () => ({ toast }));

const result = { output: "/books/Solo ch 1.epub", pages: 30, skipped: 0 };

afterEach(() => vi.clearAllMocks());

describe("exportSummary", () => {
  it("names the page count and file", () => {
    expect(exportSummary(result)).toEqual({
      title: "Exported 30 pages to Solo ch 1.epub",
      warning: null,
    });
  });
  it("adds a warning line when files were skipped", () => {
    expect(exportSummary({ ...result, pages: 1, skipped: 2 })).toEqual({
      title: "Exported 1 page to Solo ch 1.epub",
      warning: "2 unsupported files skipped — see logs",
    });
    expect(exportSummary({ ...result, skipped: 1 }).warning).toBe(
      "1 unsupported file skipped — see logs",
    );
  });
});

describe("notifyExported", () => {
  it("shows success when nothing was skipped", () => {
    notifyExported(result);
    expect(toast.success).toHaveBeenCalledWith("Exported 30 pages to Solo ch 1.epub", {
      id: "export",
    });
    expect(toast.warning).not.toHaveBeenCalled();
  });
  it("warns, with an Open logs action, when files were skipped", () => {
    notifyExported({ ...result, skipped: 2 });
    expect(toast.success).not.toHaveBeenCalled();
    const [, opts] = toast.warning.mock.calls[0] ?? [];
    expect(opts).toMatchObject({ description: "2 unsupported files skipped — see logs" });
    expect(opts.action.label).toBe("Open logs");
  });
});

describe("toExportResult", () => {
  it("keeps output, pages and skipped from the Rust stats", () => {
    const stats = {
      pages: 29,
      strips: 0,
      source_rows: 0,
      pagination_ms: 1,
      filters_ms: 2,
      skipped: 1,
      output: "/b.epub",
    };
    expect(toExportResult(stats)).toEqual({ output: "/b.epub", pages: 29, skipped: 1 });
  });
});
