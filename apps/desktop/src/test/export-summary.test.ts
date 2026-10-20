import { afterEach, describe, expect, it, vi } from "vitest";
import { booksOf, combineResults, exportSummary, notifyExported } from "@/lib/export-summary";
import { toExportResult } from "@/lib/ipc.wire";

const toast = vi.hoisted(() => ({ success: vi.fn(), warning: vi.fn(), error: vi.fn() }));
vi.mock("sonner", () => ({ toast }));
const openLogs = vi.hoisted(() => vi.fn());
vi.mock("@/lib/ipc.diagnostics", () => ({ openLogs }));

const result = { output: "/books/Solo ch 1.epub", parts: [], pages: 30, skipped: 0 };

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
    expect(exportSummary({ ...result, detectors_error: "dlopen failed: x" }).warning).toBe(
      "Cut without detectors: dlopen failed: x",
    );
    expect(exportSummary({ ...result, skipped: 1 }).warning).toBe(
      "1 unsupported file skipped — see logs",
    );
  });
});

describe("exportSummary of a split book", () => {
  it("says how many parts were written", () => {
    const parts = ["/books/Solo (part 1).epub", "/books/Solo (part 2).epub"];
    expect(exportSummary({ ...result, output: parts[0], parts }).title).toBe(
      "Exported 30 pages in 2 parts, from Solo (part 1).epub",
    );
  });
});

describe("booksOf", () => {
  const result = { output: "/k/Solo.epub", parts: [], pages: 9, skipped: 0 };
  it("sends a book under its file name", () => {
    expect(booksOf(result)).toEqual([{ file: "/k/Solo.epub", title: "Solo" }]);
  });
  it("sends each part as part n of the total, like the book's own title", () => {
    const parts = ["/k/Solo (part 1).epub", "/k/Solo (part 2).epub"];
    expect(booksOf({ ...result, output: parts[0], parts }).map((b) => b.title)).toEqual([
      "Solo (part 1 of 2)",
      "Solo (part 2 of 2)",
    ]);
  });
});

describe("combineResults", () => {
  it("adds up pages and skipped files across books", () => {
    const books = [result, { output: "/books/Solo v2.epub", parts: [], pages: 10, skipped: 2 }];
    expect(combineResults(books)).toEqual({
      output: "2 books",
      parts: [],
      pages: 40,
      skipped: 2,
      detectors_error: null,
    });
    expect(exportSummary(combineResults(books)).warning).toMatch(/2 unsupported files skipped/);
  });
});

describe("notifyExported", () => {
  it("shows success when nothing was skipped", () => {
    notifyExported(result);
    expect(toast.success).toHaveBeenCalledWith("Exported 30 pages to Solo ch 1.epub", {
      id: "export",
      description: null,
      action: null,
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
  it("says so when the logs folder can't be opened", async () => {
    openLogs.mockRejectedValueOnce(new Error("no file manager"));
    notifyExported({ ...result, skipped: 1 });
    const [, opts] = toast.warning.mock.calls[0] ?? [];
    opts.action.onClick();
    await vi.waitFor(() => expect(toast.error).toHaveBeenCalled());
    expect(toast.error.mock.calls[0]?.[0]).toBe("Couldn't open the logs folder");
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
    expect(toExportResult(stats)).toEqual({
      output: "/b.epub",
      parts: [],
      pages: 29,
      skipped: 1,
      detectors_error: null,
    });
  });
  it("keeps every part path of a split book, in order", () => {
    const parts = [
      { path: "/b (part 1).epub", pages: 90 },
      { path: "/b (part 2).epub", pages: 80 },
    ];
    const stats = { pages: 170, skipped: 0, output: "/b (part 1).epub", parts };
    expect(toExportResult(stats as never).parts).toEqual(["/b (part 1).epub", "/b (part 2).epub"]);
  });
});
