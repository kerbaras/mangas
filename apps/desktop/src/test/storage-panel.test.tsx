import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { StoragePanel } from "@/components/storage-panel";
import { type Backend, setBackend } from "@/lib/ipc";
import type { StorageUsage } from "@/lib/ipc.storage";

const usage: StorageUsage = {
  data_dir: "/data",
  chapters: 500_000_000,
  models: 0,
  covers: 1_500,
  library: 0,
  logs: 0,
  titles: [
    { source: "mangadex", manga_id: "a", title: "Alpha", chapters: 3, bytes: 400_000_000 },
    { source: "local", manga_id: "l", title: "Scanned", chapters: 1, bytes: 100_000_000 },
  ],
};

afterEach(() => setBackend(null));

describe("storage panel", () => {
  it("lists titles by size and deletes one's downloads after confirming", async () => {
    const storageUsage = vi.fn(async () => usage);
    const deleteDownloads = vi.fn(async () => ({ chapters: 3, bytes: 400_000_000, kept: [] }));
    setBackend({ storageUsage, deleteDownloads } as unknown as Backend);
    render(<StoragePanel />);
    expect(await screen.findByText("Alpha")).toBeTruthy();
    expect(screen.getByText("3 chapters · 400 MB")).toBeTruthy();
    expect(screen.queryByRole("button", { name: /downloads of Scanned/ })).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Delete the downloads of Alpha" }));
    expect(await screen.findByText("Delete the downloads of Alpha?")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Delete downloads" }));
    await waitFor(() => expect(deleteDownloads).toHaveBeenCalledWith("mangadex", "a"));
    await waitFor(() => expect(storageUsage).toHaveBeenCalledTimes(2));
  });

  it("clears the cover cache and measures again", async () => {
    const storageUsage = vi.fn(async () => usage);
    const clearCache = vi.fn(async () => ({ files: 1, bytes: 1_500 }));
    setBackend({ storageUsage, clearCache } as unknown as Backend);
    render(<StoragePanel />);
    fireEvent.click(await screen.findByRole("button", { name: /Clear cover cache/ }));
    await waitFor(() => expect(clearCache).toHaveBeenCalledWith("covers"));
    await waitFor(() => expect(storageUsage).toHaveBeenCalledTimes(2));
  });
});
