import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

const revealFolder = vi.hoisted(() => vi.fn(async (_path: string) => {}));
vi.mock("@/lib/dialogs", () => ({ revealFolder }));

import { ChapterActions } from "@/components/chapter-actions";
import { chapterFolder, parentFolder } from "@/lib/chapter-folder";
import type { Chapter, Manga } from "@/lib/ipc";

const chapter = (n: number, local_path: string | null): Chapter => ({
  id: `c${n}`,
  manga_id: "m",
  title: null,
  language: "en",
  volume: null,
  number: String(n),
  local_path,
});
const onDisk = [chapter(1, "/data/local/berserk/1"), chapter(2, "/data/local/berserk/2")];
const manga = (source: string): Manga =>
  ({ id: "berserk", source, title: "Berserk", description: "", status: "unknown" }) as Manga;

function actions(source: string, selected: string[], download = vi.fn(async () => {})) {
  render(
    <ChapterActions
      manga={manga(source)}
      pool={onDisk}
      visible={onDisk}
      selected={new Set(selected)}
      onExported={() => {}}
      download={download}
    />,
  );
  return download;
}

describe("Details actions", () => {
  beforeEach(() => vi.clearAllMocks());

  it("offers Open folder instead of downloads for an imported title", async () => {
    const download = actions("local", ["c1", "c2"]);
    expect(screen.queryByRole("button", { name: /download again/i })).toBeNull();
    expect(screen.queryByRole("button", { name: /download selected/i })).toBeNull();
    await userEvent.click(screen.getByRole("button", { name: /open folder/i }));
    expect(revealFolder).toHaveBeenCalledWith("/data/local/berserk");
    expect(download).not.toHaveBeenCalled();
  });

  it("opens the one selected imported chapter's own folder", async () => {
    actions("local", ["c2"]);
    await userEvent.click(screen.getByRole("button", { name: /open folder/i }));
    expect(revealFolder).toHaveBeenCalledWith("/data/local/berserk/2");
  });

  it("keeps Download again for downloaded chapters of a source", async () => {
    const download = actions("mangadex", ["c1"]);
    expect(screen.queryByRole("button", { name: /open folder/i })).toBeNull();
    await userEvent.click(screen.getByRole("button", { name: /download again/i }));
    expect(download).toHaveBeenCalledWith([onDisk[0]], true);
  });
});

describe("chapterFolder", () => {
  it("takes the parent folder on either separator", () => {
    expect(parentFolder("C:\\mangas\\local\\x\\1\\")).toBe("C:\\mangas\\local\\x");
    expect(parentFolder("/d/local/x/1")).toBe("/d/local/x");
    expect(parentFolder("/books")).toBe("/");
    expect(parentFolder("C:\\Books")).toBe("C:\\");
    expect(parentFolder("C:/Books/")).toBe("C:/");
  });

  it("is null when nothing is on disk", () => {
    expect(chapterFolder([chapter(1, null)], true)).toBeNull();
  });
});
