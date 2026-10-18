import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { ChapterRow } from "@/components/chapter-row";
import { TooltipProvider } from "@/components/ui/tooltip";
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

const failed: DownloadProgress = {
  chapter_id: "c1",
  state: "failed",
  page: 0,
  pages: 0,
  done: true,
  error: "refused: cdn.example resolves to a non-public address 10.0.0.1",
  retrying: null,
};

describe("chapter row failure tooltip", () => {
  it("shows the error in a collision-aware tooltip, not a native title", async () => {
    render(
      <TooltipProvider delayDuration={0}>
        <table>
          <tbody>
            <ChapterRow chapter={chapter} selected={false} progress={failed} onToggle={() => {}} />
          </tbody>
        </table>
      </TooltipProvider>,
    );
    const label = screen.getByText("Failed");
    expect(label).not.toHaveAttribute("title");
    await userEvent.setup().hover(label);
    const tip = await screen.findByRole("tooltip");
    expect(tip).toHaveTextContent(failed.error ?? "");
  });
});
