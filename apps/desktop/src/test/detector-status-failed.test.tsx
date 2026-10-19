import { render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { ExportDialog } from "@/components/export-dialog";
import { TooltipProvider } from "@/components/ui/tooltip";
import { setBackend } from "@/lib/ipc";
import { chapters, detectorState, exportBackend, solo } from "./export-backend";

afterEach(() => setBackend(null));

const reason =
  "onnx runtime /models/onnxruntime/libonnxruntime.so: dlopen failed: invalid ELF header";

describe("detector status with a runtime that won't load", () => {
  it("shows the loader error on open, before any preview or export", async () => {
    exportBackend(
      {},
      detectorState({
        downloaded: false,
        runtime: { state: "failed", version: "1.28.3", reason },
      }),
    );
    render(
      <TooltipProvider>
        <ExportDialog open onOpenChange={() => {}} picks={[{ manga: solo, chapters }]} />
      </TooltipProvider>,
    );
    expect(await screen.findByText(new RegExp(`${reason}\\. Exports still work`))).toBeTruthy();
    expect(screen.getByText(/Off: the ONNX Runtime didn't load/)).toBeTruthy();
    expect(screen.queryByText(/Running on cpu/)).toBeNull();
    const toggle = screen.getByRole("switch", { name: "Use detectors" });
    await waitFor(() => expect(toggle.getAttribute("data-state")).toBe("unchecked"));
  });
});
