import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ChaptersTable } from "@/components/chapters-table";
import { chapterSummary } from "@/lib/chapters";
import type { Chapter } from "@/lib/ipc";
import { countChapters, markExported } from "@/lib/mock-library";

const chapter = (n: number, over: Partial<Chapter> = {}): Chapter => ({
  id: `c${n}`,
  manga_id: "m",
  title: `Chapter ${n}`,
  language: "en",
  volume: null,
  number: String(n),
  local_path: null,
  ...over,
});

const list = [
  chapter(1, { local_path: "/d/1", exported_at: "2026-10-12T19:30:00Z" }),
  chapter(2, { local_path: "/d/2" }),
  chapter(3),
  chapter(4, { availability: { kind: "locked" } }),
  chapter(1, { id: "c1-es", language: "es" }),
];

describe("chapter state", () => {
  it("shows not downloaded, on disk and exported in the state column", () => {
    render(
      <ChaptersTable
        chapters={list.slice(0, 3)}
        selected={new Set()}
        downloads={new Map()}
        onToggle={() => {}}
        onToggleAll={() => {}}
      />,
    );
    expect(screen.getByRole("columnheader", { name: "State" })).toBeInTheDocument();
    const cells = screen.getAllByRole("row").slice(1);
    expect(cells[0]).toHaveTextContent("Exported");
    expect(cells[1]).toHaveTextContent("On disk");
    expect(cells[2]).toHaveTextContent("—");
  });

  it("summarises and counts chapters once across languages, exported as read", () => {
    expect(chapterSummary(list)).toBe("5 chapters · 2 on disk · 1 exported");
    expect(countChapters(list)).toEqual({ total: 4, downloaded: 2, exported: 1, unread: 2 });
  });

  it("marks the exported folders in the mock like the core does", () => {
    const copy = list.map((c) => ({ ...c }));
    markExported([copy], ["/d/2", "/elsewhere"]);
    expect(copy[1].exported_at).toBeTruthy();
    expect(countChapters(copy).unread).toBe(1);
  });
});
