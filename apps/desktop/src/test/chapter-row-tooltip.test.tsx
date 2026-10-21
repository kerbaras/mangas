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

async function tooltip(proxied: boolean) {
  render(
    <TooltipProvider delayDuration={0}>
      <table>
        <tbody>
          <ChapterRow
            index={0}
            chapter={chapter}
            selected={false}
            progress={failed}
            onToggle={() => {}}
            proxied={proxied}
          />
        </tbody>
      </table>
    </TooltipProvider>,
  );
  const label = screen.getByText("Failed");
  expect(label).not.toHaveAttribute("title");
  await userEvent.setup().hover(label);
  return screen.findByRole("tooltip");
}

describe("chapter row failure tooltip", () => {
  it("shows the error in a collision-aware tooltip, not a native title", async () => {
    const tip = await tooltip(false);
    expect(tip).toHaveTextContent(failed.error ?? "");
    expect(tip).toHaveTextContent("so the app won't fetch it");
  });

  it("says a download goes through the proxy when one is in use", async () => {
    const tip = await tooltip(true);
    expect(tip).toHaveTextContent("Retry fetches it through the configured proxy");
    expect(tip).toHaveTextContent(failed.error ?? "");
  });
});
