import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { DownloadRow } from "@/components/download-row";
import { TooltipProvider } from "@/components/ui/tooltip";
import type { DownloadJob } from "@/lib/ipc.downloads";
import { makeJob } from "./job-fixture";

function bar(job: DownloadJob, speed: number | null = null) {
  render(
    <TooltipProvider>
      <ul>
        <DownloadRow job={job} speed={speed} />
      </ul>
    </TooltipProvider>,
  );
  const indicator = screen.getByRole("progressbar").firstElementChild;
  return indicator?.className ?? "";
}

describe("DownloadRow progress", () => {
  it("does not pulse or animate a paused row with no page count", () => {
    const classes = bar(makeJob({ state: "paused" }));
    expect(classes).not.toContain("animate-pulse");
    expect(classes).not.toContain("transition");
  });

  it("does not animate a stalled row and hides its frozen speed", () => {
    const job = makeJob({ state: "running", pages_done: 4, pages_total: 20, stalled: true });
    expect(bar(job, 3.4)).not.toContain("transition");
    expect(screen.getByText("Stalled, waiting to resume")).toBeTruthy();
    expect(screen.queryByText(/pages\/s/)).toBeNull();
  });

  it("keeps the busy pulse on a starting row", () => {
    expect(bar(makeJob({ state: "running" }))).toContain("animate-pulse");
  });

  it("animates a row that is receiving pages", () => {
    const job = makeJob({ state: "running", pages_done: 4, pages_total: 20 });
    expect(bar(job, 3.4)).toContain("transition");
    expect(screen.getByText(/pages\/s/)).toBeTruthy();
  });
});

describe("DownloadRow group", () => {
  it("names the release's group when the title has several", () => {
    bar(makeJob({ state: "running", title: "Chapter 4", group: "K Manga" }));
    expect(screen.getByText("Chapter 4 · K Manga · English · mangadex")).toBeTruthy();
  });
});
