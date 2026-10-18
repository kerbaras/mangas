import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { DataFolderField } from "@/components/data-folder-field";
import { TooltipProvider } from "@/components/ui/tooltip";

vi.mock("@/hooks/use-downloads", () => ({
  useDownloads: () => new Map(),
  activeCount: () => 2,
}));

vi.mock("@/lib/settings", () => ({
  useSettings: () => [{ dataDir: "/data" }],
  loadSettings: vi.fn(),
}));

describe("DataFolderField", () => {
  it("says how many downloads block the move", () => {
    render(
      <TooltipProvider>
        <DataFolderField onMoved={() => {}} />
      </TooltipProvider>,
    );
    expect(screen.getByText(/^2 downloads queued or running/)).toBeTruthy();
    expect((screen.getByRole("button", { name: /Move/ }) as HTMLButtonElement).disabled).toBe(true);
  });
});
