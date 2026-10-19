import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { StoragePanel } from "@/components/storage-panel";
import { type Backend, setBackend } from "@/lib/ipc";
import type { StorageUsage } from "@/lib/ipc.storage";

const TITLE = `The ${"Extraordinarily ".repeat(11)}Long Title`;
const proto = HTMLSpanElement.prototype;

/** jsdom has no layout: make every `span` report `scroll` px of text in a 100 px box. */
function layout(scroll: number) {
  Object.defineProperty(proto, "scrollWidth", { configurable: true, get: () => scroll });
  Object.defineProperty(proto, "clientWidth", { configurable: true, get: () => 100 });
}

const usage: StorageUsage = {
  data_dir: "/data",
  chapters: 400_000_000,
  models: 0,
  covers: 0,
  library: 0,
  logs: 0,
  titles: [{ source: "mangadex", manga_id: "a", title: TITLE, chapters: 3, bytes: 400_000_000 }],
  partial_move: null,
};

afterEach(() => {
  setBackend(null);
  Reflect.deleteProperty(proto, "scrollWidth");
  Reflect.deleteProperty(proto, "clientWidth");
});

async function renderPanel() {
  setBackend({ storageUsage: vi.fn(async () => usage) } as unknown as Backend);
  const { container } = render(<StoragePanel />);
  return { container, title: await screen.findByText(TITLE) };
}

describe("storage usage list hints", () => {
  it("uses no native tooltip anywhere in the panel", async () => {
    layout(400);
    const { container } = await renderPanel();
    expect(container.querySelectorAll("[title]")).toHaveLength(0);
  });

  it("shows a cut-off title in the Hint on hover and on keyboard focus", async () => {
    layout(400);
    const { title } = await renderPanel();
    const user = userEvent.setup();
    await user.hover(title);
    expect(await screen.findByRole("tooltip")).toHaveTextContent(TITLE);
    await user.unhover(title);
    await user.keyboard("{Escape}");
    expect(title).toHaveAttribute("tabindex", "0");
    title.blur();
    // The data-folder field and buttons come first in tab order.
    for (let i = 0; i < 20 && document.activeElement !== title; i++) await user.tab();
    expect(title).toHaveFocus();
    expect(await screen.findByRole("tooltip")).toHaveTextContent(TITLE);
  });

  it("has no Hint when the title fits", async () => {
    layout(80);
    const { title } = await renderPanel();
    await userEvent.setup().hover(title);
    expect(screen.queryByRole("tooltip")).toBeNull();
    expect(title).not.toHaveAttribute("tabindex");
  });

  it("names the delete button in the Hint", async () => {
    layout(80);
    await renderPanel();
    const user = userEvent.setup();
    await user.hover(screen.getByRole("button", { name: `Delete the downloads of ${TITLE}` }));
    expect(await screen.findByRole("tooltip")).toHaveTextContent("Delete downloads");
  });
});
