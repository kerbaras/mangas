import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ExportProgressPanel } from "@/components/export-progress";
import { TooltipProvider } from "@/components/ui/tooltip";
import type { ExportResult } from "@/lib/ipc";

const result = (parts: string[]): ExportResult => ({
  output: parts[0] ?? "/books/Solo.epub",
  parts,
  pages: 123,
  skipped: 0,
  detectors_error: null,
});

const panel = (r: ExportResult) =>
  render(
    <TooltipProvider>
      <ExportProgressPanel state={{ kind: "done", results: [r] }} onReset={() => {}} />
    </TooltipProvider>,
  );

describe("export result panel", () => {
  it("counts the parts of a split book and shows their folder", () => {
    panel(result(["/books/Solo (part 1).epub", "/books/Solo (part 2).epub"]));
    expect(screen.getByText("Exported 123 pages in 2 parts")).toBeTruthy();
    expect(screen.getByText("/books")).toBeTruthy();
  });

  it("shows the file of a book written whole", () => {
    panel(result([]));
    expect(screen.getByText("Exported 123 pages")).toBeTruthy();
    expect(screen.getByText("/books/Solo.epub")).toBeTruthy();
  });
});
