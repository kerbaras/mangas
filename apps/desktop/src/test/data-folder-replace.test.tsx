import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { DataFolderField } from "@/components/data-folder-field";
import { TooltipProvider } from "@/components/ui/tooltip";
import { type Backend, setBackend } from "@/lib/ipc";

vi.mock("@/hooks/use-downloads", () => ({ useDownloads: () => new Map(), activeCount: () => 0 }));
vi.mock("@/lib/settings", () => ({
  useSettings: () => [{ dataDir: "/data" }],
  loadSettings: vi.fn(),
}));
vi.mock("@/lib/dialogs", () => ({ pickFolder: async () => "/mnt/big" }));

afterEach(() => setBackend(null));

const moved = { data_dir: "/mnt/big", files: 3, bytes: 300, leftovers: [] };

function field(partial: { path: string; bytes: number } | null) {
  const moveDataDir = vi.fn(async () => moved);
  setBackend({
    moveTargetPartial: async () => partial,
    moveDataDir,
    onMoveProgress: async () => () => {},
  } as unknown as Backend);
  render(
    <TooltipProvider>
      <DataFolderField onMoved={() => {}} partial={null} />
    </TooltipProvider>,
  );
  return moveDataDir;
}

describe("DataFolderField: a folder an interrupted move left a partial copy in", () => {
  it("asks once to replace the partial copy, then moves replacing it", async () => {
    const moveDataDir = field({ path: "/mnt/big", bytes: 250_000_000 });
    fireEvent.click(screen.getByRole("button", { name: /Move/ }));
    expect(await screen.findByText("Replace the partial copy and move?")).toBeTruthy();
    expect(screen.getByText(/holds a partial copy \(250 MB\)/)).toBeTruthy();
    expect(screen.getByText(/files added there by hand stay/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Replace and move" }));
    await waitFor(() => expect(moveDataDir).toHaveBeenCalledWith("/mnt/big", true));
  });

  it("moves into a clean folder without replacing anything", async () => {
    const moveDataDir = field(null);
    fireEvent.click(screen.getByRole("button", { name: /Move/ }));
    expect(await screen.findByText("Move the data folder?")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Move" }));
    await waitFor(() => expect(moveDataDir).toHaveBeenCalledWith("/mnt/big", false));
  });
});
