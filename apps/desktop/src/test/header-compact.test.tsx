import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Plus, RefreshCw } from "lucide-react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { HeaderAction } from "@/components/header-action";
import { COMPACT_BELOW, ScreenHeader } from "@/components/screen-header";
import { TooltipProvider } from "@/components/ui/tooltip";

/** Fires every observer once with `width` as the observed element's clientWidth. */
function stubWidth(width: number) {
  vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockReturnValue(width);
  vi.stubGlobal(
    "ResizeObserver",
    class {
      constructor(private cb: (entries: { target: Element }[]) => void) {}
      observe(target: Element) {
        this.cb([{ target }]);
      }
      unobserve() {}
      disconnect() {}
    },
  );
}

const header = () =>
  render(
    <TooltipProvider delayDuration={0}>
      <ScreenHeader title="Library">
        <HeaderAction icon={<RefreshCw />} label="Check for updates" />
        <HeaderAction variant="primary" keepLabel icon={<Plus />} label="Follow a manga" />
      </ScreenHeader>
    </TooltipProvider>,
  );

afterEach(() => {
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

describe("screen header actions", () => {
  it("show their labels and no Hint when the header is wide", async () => {
    stubWidth(COMPACT_BELOW);
    header();
    const check = screen.getByRole("button", { name: "Check for updates" });
    expect(check).toHaveTextContent("Check for updates");
    await userEvent.setup().hover(check);
    expect(screen.queryByRole("tooltip")).toBeNull();
  });

  it("go icon-only with the label in the Hint (hover and focus) when it is narrow", async () => {
    stubWidth(COMPACT_BELOW - 1);
    header();
    const check = screen.getByRole("button", { name: "Check for updates" });
    expect(check).toHaveTextContent("");
    expect(check).not.toHaveAttribute("title");
    const user = userEvent.setup();
    await user.hover(check);
    expect(await screen.findByRole("tooltip")).toHaveTextContent("Check for updates");
    await user.unhover(check);
    await user.keyboard("{Escape}");
    check.blur();
    await user.tab();
    expect(check).toHaveFocus();
    expect(await screen.findByRole("tooltip")).toHaveTextContent("Check for updates");
  });

  it("keep the main action's label when narrow", () => {
    stubWidth(COMPACT_BELOW - 1);
    header();
    expect(screen.getByRole("button", { name: "Follow a manga" })).toHaveTextContent(
      "Follow a manga",
    );
  });
});
