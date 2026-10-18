import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ChapterRow } from "@/components/chapter-row";
import type { Chapter, DownloadProgress } from "@/lib/ipc";

const chapter: Chapter = {
  id: "c1",
  manga_id: "m",
  title: "Casca",
  language: "en",
  volume: "1",
  number: "8",
  local_path: null,
};

const running = (page: number, pages: number): DownloadProgress => ({
  chapter_id: "c1",
  state: "running",
  page,
  pages,
  done: false,
  error: null,
  retrying: null,
});

const row = (progress: DownloadProgress) =>
  render(
    <table>
      <tbody>
        <ChapterRow chapter={chapter} selected={false} progress={progress} onToggle={() => {}} />
      </tbody>
    </table>,
  );

afterEach(() => vi.restoreAllMocks());

describe("chapter row progress", () => {
  it("is indeterminate, without warnings, until the page count is known", () => {
    const warn = vi.spyOn(console, "error");
    row(running(0, 0));
    const bar = screen.getByRole("progressbar");
    expect(bar.getAttribute("aria-valuemax")).not.toBe("0");
    expect(bar.getAttribute("data-state")).toBe("indeterminate");
    expect(screen.getByText("Starting")).toBeTruthy();
    expect(warn).not.toHaveBeenCalled();
  });

  it("shows pages once the total is known", () => {
    row(running(3, 20));
    expect(screen.getByRole("progressbar").getAttribute("aria-valuemax")).toBe("20");
    expect(screen.getByText("3/20")).toBeTruthy();
  });
});
