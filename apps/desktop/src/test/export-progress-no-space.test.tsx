import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ExportProgressPanel } from "@/components/export-progress";
import { ExportStoppedPanel } from "@/components/export-stopped-panel";
import { ExportStopped } from "@/lib/export-stopped";

const failed = (message: string) =>
  render(<ExportProgressPanel state={{ kind: "error", message }} onRetry={() => {}} />);

describe("export failed: out of disk space", () => {
  it("says so plainly, with what to do and no Try again", () => {
    failed("not enough space in /media/me/USB");
    expect(screen.getByText("not enough space in /media/me/USB")).toBeTruthy();
    expect(screen.getByText(/Free up space on that disk or choose another folder/)).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Try again" })).toBeNull();
  });

  it("keeps Try again for other failures", () => {
    failed("io: Permission denied (os error 13)");
    expect(screen.getByRole("button", { name: "Try again" })).toBeTruthy();
  });

  it("a volume that stopped on a full disk has no Try again either", () => {
    const stopped = (reason: string) =>
      new ExportStopped({
        written: [],
        failed: { title: "Vol 2", output: "/k/Vol 2.epub", reason },
        not_written: [],
      });
    const { unmount } = render(
      <ExportStoppedPanel stopped={stopped("not enough space in /k")} onReset={() => {}} />,
    );
    expect(screen.getByText(/Free up space on that disk or choose another folder/)).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Try again" })).toBeNull();
    unmount();
    render(<ExportStoppedPanel stopped={stopped("io: Permission denied")} onReset={() => {}} />);
    expect(screen.getByRole("button", { name: "Try again" })).toBeTruthy();
  });
});
