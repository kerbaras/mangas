import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { ScreenHeader } from "@/components/screen-header";

const TITLE =
  "The Extraordinarily Long Chronicles of a Wandering Swordsman Who Accidentally Became the Strongest Librarian in the Realm";
const proto = HTMLHeadingElement.prototype;

/** jsdom has no layout: make every `h1` report `scroll` px of text in a 100 px box. */
function layout(scroll: number) {
  Object.defineProperty(proto, "scrollWidth", { configurable: true, get: () => scroll });
  Object.defineProperty(proto, "clientWidth", { configurable: true, get: () => 100 });
}

afterEach(() => {
  Reflect.deleteProperty(proto, "scrollWidth");
  Reflect.deleteProperty(proto, "clientWidth");
});

/** Renders twice: the second render sees the mounted `h1` (ResizeObserver does this in the app). */
function renderHeader() {
  const { rerender } = render(<ScreenHeader back title={TITLE} />);
  rerender(<ScreenHeader back title={TITLE} />);
  return screen.getByRole("heading", { name: TITLE });
}

describe("screen header title", () => {
  it("shows the full truncated title in the Hint on hover and on Tab", async () => {
    layout(400);
    const heading = renderHeader();
    expect(heading).not.toHaveAttribute("title");
    expect(heading).toHaveAttribute("tabindex", "0");
    const user = userEvent.setup();
    await user.hover(heading);
    expect(await screen.findByRole("tooltip")).toHaveTextContent(TITLE);
    await user.unhover(heading);
    await user.keyboard("{Escape}");
    heading.blur();
    await user.tab();
    await user.tab();
    expect(heading).toHaveFocus();
    expect(await screen.findByRole("tooltip")).toHaveTextContent(TITLE);
  });

  it("has no Hint and no tab stop when the title fits", async () => {
    layout(80);
    const heading = renderHeader();
    expect(heading).not.toHaveAttribute("tabindex");
    await userEvent.setup().hover(heading);
    expect(screen.queryByRole("tooltip")).toBeNull();
  });
});
