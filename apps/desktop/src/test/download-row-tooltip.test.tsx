import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { DownloadRow } from "@/components/download-row";
import { TooltipProvider } from "@/components/ui/tooltip";
import { type DownloadJob, UNLISTED } from "@/lib/ipc.downloads";
import { makeJob } from "./job-fixture";

const REASON = "refused: webtoon-phinf.pstatic.net resolves to a non-public address 10.0.0.1";
const LABEL = "The image host points at a local or private address, so the app won't fetch it.";

function row(job: DownloadJob) {
  render(
    <TooltipProvider delayDuration={0}>
      <ul>
        <DownloadRow job={job} speed={null} />
      </ul>
    </TooltipProvider>,
  );
}

describe("DownloadRow failure tooltip", () => {
  it("has no native title anywhere in a failed row", () => {
    row(makeJob({ state: "failed", error: REASON }));
    expect(screen.getByRole("listitem").querySelector(`[title="${REASON}"]`)).toBeNull();
    expect(screen.getByText(LABEL)).not.toHaveAttribute("title");
  });

  it("shows the reason in the edge-aware tooltip on keyboard focus", async () => {
    row(makeJob({ state: "failed", error: REASON }));
    const reason = screen.getByText(LABEL);
    await userEvent.tab();
    expect(reason).toHaveFocus();
    expect(await screen.findByRole("tooltip")).toHaveTextContent(REASON);
  });

  it("shows the reason on hover", async () => {
    row(makeJob({ state: "failed", error: REASON }));
    await userEvent.hover(screen.getByText(LABEL));
    expect(await screen.findByRole("tooltip")).toHaveTextContent(REASON);
  });

  it("explains a no-longer-listed badge in the tooltip, not a native title", async () => {
    const error = `${UNLISTED} (ch 7)`;
    row(makeJob({ state: "failed", error }));
    const badge = screen.getByText("No longer listed");
    expect(badge).not.toHaveAttribute("title");
    await userEvent.tab();
    expect(badge).toHaveFocus();
    expect(await screen.findByRole("tooltip")).toHaveTextContent(error);
  });
});
