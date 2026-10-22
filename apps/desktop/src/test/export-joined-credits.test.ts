import { describe, expect, it, vi } from "vitest";
import type { ExportPick } from "@/lib/export-name";
import type { Manga } from "@/lib/ipc";

const bookCreators = vi.fn(async () => ["Credited by core"]);
vi.mock("@/lib/ipc.volumes", () => ({
  bookCreators,
  exportFileName: vi.fn(async (t: string) => `${t}.epub`),
  planVolumes: vi.fn(),
}));

const { joinedPlan } = await import("@/hooks/use-export-plan");

const pick = (source: string, id: string, authors: string[]): ExportPick => ({
  manga: { id, source, title: id, authors, artists: [" Art "] } as Manga,
  chapters: [],
});

describe("joined export plan credits", () => {
  it("are exactly what core returns for the picked series, not computed in the webview", async () => {
    const plan = await joinedPlan(
      [pick("mangadex", "a", ["\uFEFFAzuma Kiyohiko"]), pick("comick", "b", [])],
      "Mix",
    );
    expect(bookCreators).toHaveBeenCalledWith([
      ["mangadex", "a"],
      ["comick", "b"],
    ]);
    expect(plan.authors).toEqual(["Credited by core"]);
  });
});
