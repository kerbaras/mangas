import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ExportProgressPanel } from "@/components/export-progress";

const failed = (message: string) =>
  render(<ExportProgressPanel state={{ kind: "error", message }} onReset={() => {}} />);

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
});
