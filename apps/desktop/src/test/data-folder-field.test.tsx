import { act, fireEvent, render, screen } from "@testing-library/react";
import axe from "axe-core";
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
        <DataFolderField onMoved={() => {}} partial={null} />
      </TooltipProvider>,
    );
    expect(screen.getByText(/^2 downloads queued or running/)).toBeTruthy();
    const move = screen.getByRole("button", { name: /Move/ }) as HTMLButtonElement;
    expect(move.getAttribute("aria-disabled")).toBe("true");
    expect(move.disabled).toBe(false);
  });

  it("shows why Move is blocked on keyboard focus, and a click does nothing", async () => {
    const { container } = render(
      <TooltipProvider>
        <DataFolderField onMoved={() => {}} partial={null} />
      </TooltipProvider>,
    );
    const move = screen.getByRole("button", { name: /Move/ });
    fireEvent.click(move);
    expect(screen.queryByRole("dialog")).toBeNull();
    await act(async () => move.focus());
    expect(document.activeElement).toBe(move);
    expect((await screen.findByRole("tooltip")).textContent).toMatch(
      /^2 downloads queued or running/,
    );
    const result = await axe.run(container, { rules: { "color-contrast": { enabled: false } } });
    expect(result.violations.map((v) => v.id)).toEqual([]);
  });
});
