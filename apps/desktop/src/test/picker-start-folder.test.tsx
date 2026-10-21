import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { DataFolderField } from "@/components/data-folder-field";
import { OutputFolder } from "@/components/output-folder";
import { FolderStep } from "@/components/setup/setup-steps";
import { TooltipProvider } from "@/components/ui/tooltip";
import { pickFolder } from "@/lib/dialogs";

const { open } = vi.hoisted(() => ({ open: vi.fn(async () => null) }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open }));
vi.mock("@/lib/ipc", async (original) => ({
  ...(await original<typeof import("@/lib/ipc")>()),
  isTauri: () => true,
}));
vi.mock("@/hooks/use-downloads", () => ({ useDownloads: () => new Map(), activeCount: () => 0 }));
vi.mock("@/lib/settings", () => ({
  useSettings: () => [{ dataDir: "/home/me/.local/share/mangas" }],
  loadSettings: vi.fn(),
}));

const startedAt = () => (open.mock.calls.at(-1) as unknown as [{ defaultPath?: string }])[0];

describe("folder pickers open at a starting folder", () => {
  beforeEach(() => open.mockClear());

  it("passes the start as defaultPath, and none when it is empty", async () => {
    await pickFolder("Pick", "~/x", "/srv/books");
    expect(startedAt()).toMatchObject({ directory: true, defaultPath: "/srv/books" });
    await pickFolder("Pick", "~/x");
    expect(startedAt().defaultPath).toBeUndefined();
  });

  it("Setup starts at the parent of the suggested data folder", async () => {
    render(<FolderStep onBusy={() => {}} />);
    fireEvent.click(screen.getByRole("button", { name: /Change/ }));
    await waitFor(() => expect(open).toHaveBeenCalled());
    expect(startedAt().defaultPath).toBe("/home/me/.local/share");
  });

  it("Storage Change starts at the parent of the current data folder", async () => {
    render(
      <TooltipProvider>
        <DataFolderField onMoved={() => {}} partial={null} />
      </TooltipProvider>,
    );
    fireEvent.click(screen.getByRole("button", { name: /Move|Change/ }));
    await waitFor(() => expect(open).toHaveBeenCalled());
    expect(startedAt().defaultPath).toBe("/home/me/.local/share");
  });

  it("the export folder picker starts next to the folder it writes to", async () => {
    const { rerender } = render(<OutputFolder value="" fallback="/d/kindle" onChange={() => {}} />);
    fireEvent.click(screen.getByRole("button", { name: "Choose output folder" }));
    await waitFor(() => expect(startedAt().defaultPath).toBe("/d"));
    rerender(<OutputFolder value="/srv/books/kindle" fallback="/d/kindle" onChange={() => {}} />);
    fireEvent.click(screen.getByRole("button", { name: "Choose output folder" }));
    await waitFor(() => expect(startedAt().defaultPath).toBe("/srv/books"));
  });
});
