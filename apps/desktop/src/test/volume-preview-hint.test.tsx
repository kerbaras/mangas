import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { VolumePreview } from "@/components/volume-preview";
import type { Chapter } from "@/lib/ipc";

const FILE =
  "The Extraordinarily Long Chronicles of a Wandering Swordsman Who Accidentally - Vol. 1.epub";
const chapter: Chapter = {
  id: "c1",
  manga_id: "m",
  title: null,
  language: "en",
  volume: "1",
  number: "1",
  local_path: "/data/c1",
};
const plans = {
  kind: "ready" as const,
  data: [{ number: "1", title: "Vol. 1", file_name: FILE, chapters: [chapter], authors: [] }],
};
const proto = HTMLElement.prototype;

afterEach(() => {
  Reflect.deleteProperty(proto, "scrollWidth");
  Reflect.deleteProperty(proto, "clientWidth");
});

describe("volume preview file names", () => {
  it("show a cut-off file name in the shared Hint on hover and on Tab, not a native title", async () => {
    Object.defineProperty(proto, "scrollWidth", { configurable: true, get: () => 400 });
    Object.defineProperty(proto, "clientWidth", { configurable: true, get: () => 100 });
    const { rerender, container } = render(<VolumePreview plans={plans} folder="/books" />);
    rerender(<VolumePreview plans={plans} folder="/books" />);
    expect(container.querySelector("[title]")).toBeNull();
    const name = screen.getByText(FILE);
    const user = userEvent.setup();
    await user.hover(name);
    expect(await screen.findByRole("tooltip")).toHaveTextContent(FILE);
    await user.unhover(name);
    await user.keyboard("{Escape}");
    await user.tab();
    expect(name).toHaveFocus();
    expect(await screen.findByRole("tooltip")).toHaveTextContent(FILE);
  });
});

describe("volume preview credits", () => {
  it("names the book's authors when the series has some, and nothing when it has none", () => {
    const credited = { ...plans, data: [{ ...plans.data[0], authors: ["Azuma Kiyohiko"] }] };
    const { rerender } = render(<VolumePreview plans={credited} folder="/books" />);
    expect(screen.getByText("By Azuma Kiyohiko")).toBeInTheDocument();
    rerender(<VolumePreview plans={plans} folder="/books" />);
    expect(screen.queryByText(/^By /)).toBeNull();
  });
  it("show a cut-off credit line in full in the shared Hint on hover", async () => {
    Object.defineProperty(proto, "scrollWidth", { configurable: true, get: () => 400 });
    Object.defineProperty(proto, "clientWidth", { configurable: true, get: () => 100 });
    const names = ["Chugong", "Jang Sung-Rak", "Redice Studio", "h-goon"];
    const credited = { ...plans, data: [{ ...plans.data[0], authors: names }] };
    const { rerender, container } = render(<VolumePreview plans={credited} folder="/books" />);
    rerender(<VolumePreview plans={credited} folder="/books" />);
    expect(container.querySelector("[title]")).toBeNull();
    await userEvent.setup().hover(screen.getByText(`By ${names.join(", ")}`));
    expect(await screen.findByRole("tooltip")).toHaveTextContent(names.join(", "));
  });
});
