import { act, renderHook } from "@testing-library/react";
import { toast } from "sonner";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useExport } from "@/hooks/use-export";
import { ExportStopped } from "@/lib/export-stopped";

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), {
    success: vi.fn(),
    error: vi.fn(),
    warning: vi.fn(),
    info: vi.fn(),
    dismiss: vi.fn(),
  }),
}));
vi.mock("@/lib/ipc", async (original) => ({
  ...(await original<typeof import("@/lib/ipc")>()),
  onExportProgress: async () => () => {},
}));

afterEach(() => vi.clearAllMocks());

const disk = () => Promise.reject(new Error("io: No space left on device (os error 28)"));
const stopped = () =>
  Promise.reject(
    new ExportStopped({
      written: [],
      failed: { title: "Solo v2", output: "/k/Solo v2.epub", reason: "io: disk full" },
      not_written: [],
    }),
  );

describe("export failure surface (#299)", () => {
  it("shows a failure only in the open dialog: no toast, an earlier one dismissed", async () => {
    const { result } = renderHook(() => useExport({ visible: () => true }));
    await act(() => result.current.run(disk));
    expect(result.current.state.kind).toBe("error");
    await act(() => result.current.run(stopped));
    expect(result.current.state.kind).toBe("stopped");
    expect(toast.error).not.toHaveBeenCalled();
    expect(toast.dismiss).toHaveBeenCalledWith("export");
  });

  it("toasts the failure when the dialog is closed", async () => {
    let open = true;
    const { result } = renderHook(() => useExport({ visible: () => open }));
    open = false;
    await act(() => result.current.run(disk));
    expect(toast.error).toHaveBeenCalledWith(
      "Export failed",
      expect.objectContaining({ id: "export" }),
    );
    await act(() => result.current.run(stopped));
    expect(toast.error).toHaveBeenLastCalledWith(
      "Export stopped",
      expect.objectContaining({ id: "export" }),
    );
  });
});
