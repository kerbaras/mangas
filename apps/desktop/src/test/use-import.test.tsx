import { act, render, renderHook, screen } from "@testing-library/react";
import { Toaster } from "sonner";
import { describe, expect, it, vi } from "vitest";
import { useImport } from "@/hooks/use-import";
import { type ImportReport, importLocal } from "@/lib/ipc";

vi.mock("@/lib/dialogs", () => ({ pickImportPaths: vi.fn(async () => ["/books/Berserk"]) }));
vi.mock("@/lib/ipc", () => ({ importLocal: vi.fn(), isChallenge: () => false }));

const report = {
  manga: { id: "berserk", source: "local", title: "Berserk", description: "", status: "unknown" },
  imported: [{ number: "1", title: null, pages: 20, path: "/data/local/berserk/1" }],
  skipped: [],
} as unknown as ImportReport;

describe("useImport", () => {
  it("reports the import in a toast and hands the report on", async () => {
    vi.mocked(importLocal).mockResolvedValueOnce(report);
    render(<Toaster />);
    const onImported = vi.fn();
    const { result } = renderHook(() => useImport(onImported));

    await act(() => result.current.start("folder"));

    expect(await screen.findByText("Imported 1 chapter (20 pages) of Berserk")).toBeInTheDocument();
    expect(onImported).toHaveBeenCalledWith(report);
    expect(result.current.busy).toBe(false);
  });

  it("shows a failed import as an error toast with the core message", async () => {
    vi.mocked(importLocal).mockRejectedValueOnce("No chapters with images in /books/Empty");
    render(<Toaster />);
    const onImported = vi.fn();
    const { result } = renderHook(() => useImport(onImported));

    await act(() => result.current.start("folder"));

    expect(await screen.findByText("Couldn't import")).toBeInTheDocument();
    expect(screen.getByText("No chapters with images in /books/Empty")).toBeInTheDocument();
    expect(onImported).not.toHaveBeenCalled();
    expect(result.current.busy).toBe(false);
  });
});
