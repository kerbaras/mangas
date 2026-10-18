import { afterEach, describe, expect, it, vi } from "vitest";

const invoke = vi.hoisted(() => vi.fn(async () => undefined));
vi.mock("@tauri-apps/api/core", () => ({ invoke }));

import { type Backend, setBackend } from "@/lib/ipc";
import { openLogs } from "@/lib/ipc.diagnostics";
import { type DownloadJob, openJobFolder } from "@/lib/ipc.downloads";
import { tauriFolders } from "@/lib/ipc.tauri-folders";

const fake = (openFolder: (path: string) => Promise<void>) =>
  setBackend({
    diagnostics: async () => ({ log_dir: "/data/mangas/logs", report: "" }),
    openFolder,
  } as unknown as Backend);

describe("openFolder", () => {
  afterEach(() => setBackend(null));

  it("calls the open_folder command with the path", async () => {
    await tauriFolders.openFolder("/data/mangas/kindle");
    expect(invoke).toHaveBeenCalledWith("open_folder", { path: "/data/mangas/kindle" });
  });

  it("opens the logs folder the diagnostics name", async () => {
    const openFolder = vi.fn(async () => {});
    fake(openFolder);
    await openLogs();
    expect(openFolder).toHaveBeenCalledWith("/data/mangas/logs");
  });

  it("opens a download's chapter folder, or says it isn't on disk", async () => {
    const openFolder = vi.fn(async () => {});
    fake(openFolder);
    const job = { id: 1, number: "80", local_path: "/data/chapters/95/80" } as DownloadJob;
    await openJobFolder(job);
    expect(openFolder).toHaveBeenCalledWith("/data/chapters/95/80");
    await expect(openJobFolder({ ...job, local_path: null })).rejects.toThrow("not on disk");
  });
});
