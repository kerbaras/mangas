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

describe("DownloadRow failure", () => {
  const row = (error: string) =>
    render(
      <TooltipProvider>
        <ul>
          <DownloadRow job={makeJob({ state: "failed", error })} speed={null} />
        </ul>
      </TooltipProvider>,
    );

  it("shows the ca-certificates hint and no Retry when the system has no certificates", () => {
    row(
      "network: no CA certificates found on this system, so secure connections can't be checked; install the ca-certificates package (e.g. `sudo apt install ca-certificates`)",
    );
    expect(screen.getByText(/Install the ca-certificates package/)).toBeTruthy();
    expect(screen.queryByText(/no CA certificates found/)).toBeNull();
    expect(screen.queryByRole("button", { name: /retry/i })).toBeNull();
  });

  it("keeps Retry for an unreachable host", () => {
    row("network: error sending request: dns error: no such host");
    expect(screen.getByText(/Couldn't reach|connection/i)).toBeTruthy();
    expect(screen.getByRole("button", { name: /retry/i })).toBeTruthy();
  });
});

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

  it("shows a static busy third on a starting row, with no animation", () => {
    expect(bar(makeJob({ state: "running" }))).not.toMatch(/animate-|transition/);
    const fill = screen.getByRole("progressbar").firstElementChild as HTMLElement;
    expect(fill.style.transform).toBe("scaleX(0.33)");
  });

  it("steps a row that is receiving pages by scale, with no transition", () => {
    const job = makeJob({ state: "running", pages_done: 4, pages_total: 20 });
    expect(bar(job, 3.4)).not.toContain("transition");
    const fill = screen.getByRole("progressbar").firstElementChild as HTMLElement;
    expect(fill.style.transform).toBe("scaleX(0.2)");
    expect(screen.getByText(/pages\/s/)).toBeTruthy();
  });
});

describe("DownloadRow group", () => {
  it("names the release's group when the title has several", () => {
    bar(makeJob({ state: "running", title: "Chapter 4", group: "K Manga" }));
    expect(screen.getByText("Chapter 4 · K Manga · English · mangadex")).toBeTruthy();
  });
});
