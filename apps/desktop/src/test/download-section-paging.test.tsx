import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { DownloadSection } from "@/components/download-section";
import { TooltipProvider } from "@/components/ui/tooltip";
import type { DownloadJob } from "@/lib/ipc.downloads";
import { makeJob } from "./job-fixture";

const done = (n: number): DownloadJob[] =>
  Array.from({ length: n }, (_, i) =>
    makeJob({ id: i + 1, state: "done", local_path: `/d/${i}`, number: String(i + 1) }),
  );

function Section({ jobs, pageSize }: { jobs: DownloadJob[]; pageSize?: number }) {
  return (
    <TooltipProvider>
      <DownloadSection title="Completed" jobs={jobs} samples={new Map()} pageSize={pageSize} />
    </TooltipProvider>
  );
}

const rows = () => within(screen.getByRole("list")).getAllByRole("listitem");

describe("DownloadSection paging", () => {
  it("shows one page of rows and the full count", () => {
    render(<Section jobs={done(120)} pageSize={50} />);
    expect(rows()).toHaveLength(50);
    expect(screen.getByText("120")).toBeInTheDocument();
    expect(screen.getByText("70 not shown")).toBeInTheDocument();
  });

  it("adds a page per Show more until every row is shown", async () => {
    render(<Section jobs={done(120)} pageSize={50} />);
    await userEvent.click(screen.getByRole("button", { name: "Show 50 more" }));
    expect(rows()).toHaveLength(100);
    await userEvent.click(screen.getByRole("button", { name: "Show 20 more" }));
    expect(rows()).toHaveLength(120);
    expect(screen.queryByRole("button", { name: /more$/ })).not.toBeInTheDocument();
  });

  it("a new completion pushes one row in and one out", () => {
    const jobs = done(120);
    const { rerender } = render(<Section jobs={jobs} pageSize={50} />);
    const fresh = makeJob({ id: 999, state: "done", local_path: "/d/new", number: "999" });
    rerender(<Section jobs={[fresh, ...jobs]} pageSize={50} />);
    expect(rows()).toHaveLength(50);
    expect(rows()[0]).toHaveTextContent("Chapter 999");
    expect(screen.getByText("71 not shown")).toBeInTheDocument();
  });

  it("shows every row without a page size", () => {
    render(<Section jobs={done(120)} />);
    expect(rows()).toHaveLength(120);
  });
});
