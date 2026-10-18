import { act, render, screen } from "@testing-library/react";
import { Toaster } from "sonner";
import { describe, expect, it, vi } from "vitest";
import { notifyExported } from "@/lib/export-summary";

vi.mock("@/lib/ipc.diagnostics", () => ({ openLogs: vi.fn() }));

const result = { output: "/books/Solo ch 1.epub", pages: 30, skipped: 0 };

describe("export toast", () => {
  it("drops the skipped line and Open logs when a clean export replaces a warning", async () => {
    render(<Toaster />);
    act(() => notifyExported({ ...result, skipped: 1 }));
    expect(await screen.findByText("1 unsupported file skipped — see logs")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Open logs" })).toBeInTheDocument();

    act(() => notifyExported(result));
    expect(await screen.findByText("Exported 30 pages to Solo ch 1.epub")).toBeInTheDocument();
    expect(screen.queryByText(/unsupported file/)).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Open logs" })).not.toBeInTheDocument();
  });
});
