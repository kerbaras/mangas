import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterAll, beforeAll, beforeEach, describe, expect, it } from "vitest";
import { App } from "@/app";
import { setBackend } from "@/lib/ipc";
import { mockBackend } from "@/lib/ipc.mock";
import { forgetScreens } from "@/lib/screen-memory";
import { forgetSearch } from "@/lib/search-session";
import { loadSettings } from "@/lib/settings";

// jsdom has no layout, so give every element a scrollTop that keeps what was set.
const tops = new WeakMap<Element, number>();
const original = Object.getOwnPropertyDescriptor(Element.prototype, "scrollTop");
beforeAll(() => {
  window.matchMedia ??= ((query: string) => ({
    matches: false,
    media: query,
    addEventListener: () => {},
    removeEventListener: () => {},
    addListener: () => {},
    removeListener: () => {},
  })) as unknown as typeof window.matchMedia;
  Object.defineProperty(Element.prototype, "scrollTop", {
    configurable: true,
    get(this: Element) {
      return tops.get(this) ?? 0;
    },
    set(this: Element, v: number) {
      tops.set(this, v);
    },
  });
});
afterAll(() => original && Object.defineProperty(Element.prototype, "scrollTop", original));
beforeEach(async () => {
  window.localStorage.clear();
  forgetScreens();
  forgetSearch();
  setBackend(mockBackend);
  await loadSettings();
});

const T = { timeout: 4000 };
const paneOf = (el: Element) => el.closest(".overflow-auto") as HTMLElement;
const items = () => [...document.querySelectorAll<HTMLElement>("main [data-nav-item]")];

async function scrollOpenAndBack(pane: HTMLElement, open: HTMLElement) {
  pane.scrollTop = 640;
  fireEvent.scroll(pane);
  fireEvent.click(open);
  fireEvent.click(await screen.findByRole("button", { name: "Back" }, T));
}

describe("Back from Details keeps the list where it was", () => {
  it("Library: scroll offset, category and title filter", async () => {
    render(<App />);
    await waitFor(() => expect(items().length).toBeGreaterThan(1), T);
    fireEvent.mouseDown(screen.getByRole("tab", { name: /Weekly/ }));
    expect(screen.getByRole("tab", { name: /Weekly/ })).toHaveAttribute("data-state", "active");
    const title = items()[0].textContent?.trim().slice(0, 3) ?? "";
    fireEvent.change(screen.getByLabelText("Filter by title"), { target: { value: title } });
    await waitFor(() => expect(items().length).toBeGreaterThan(0), T);
    await scrollOpenAndBack(paneOf(items()[0]), items()[0]);

    const filter = await screen.findByLabelText("Filter by title", undefined, T);
    expect(filter).toHaveValue(title);
    expect(screen.getByRole("tab", { name: /Weekly/ })).toHaveAttribute("data-state", "active");
    await waitFor(() => expect(paneOf(items()[0]).scrollTop).toBe(640), T);
  });

  it("Search: scroll offset, query and provider filter", async () => {
    render(<App />);
    await waitFor(() => expect(items().length).toBeGreaterThan(0), T);
    fireEvent.click(screen.getByRole("button", { name: /Search/ }));
    fireEvent.change(await screen.findByLabelText("Search manga"), { target: { value: "a" } });
    const chips = await screen.findByRole("group", { name: "Filter by provider" }, T);
    await waitFor(() => expect(items().length).toBeGreaterThan(0), T);
    const chip = [...chips.querySelectorAll("button")].find(
      (b) => !/^All/.test(b.textContent ?? ""),
    );
    if (chip) fireEvent.click(chip);
    await waitFor(() => expect(items().length).toBeGreaterThan(0), T);
    await scrollOpenAndBack(paneOf(items()[0]), items()[0]);

    expect(await screen.findByLabelText("Search manga", undefined, T)).toHaveValue("a");
    const name = chip?.textContent?.match(/^[A-Za-z ]+/)?.[0] ?? "All";
    const pressed = () =>
      [...document.querySelectorAll("fieldset button")].find(
        (b) => b.getAttribute("aria-pressed") === "true",
      );
    await waitFor(() => expect(pressed()?.textContent).toMatch(new RegExp(`^${name}`)), T);
    await waitFor(() => expect(paneOf(items()[0]).scrollTop).toBe(640), T);
  });

  it("a new search starts at the top", async () => {
    render(<App />);
    await waitFor(() => expect(items().length).toBeGreaterThan(0), T);
    fireEvent.click(screen.getByRole("button", { name: /Search/ }));
    const input = await screen.findByLabelText("Search manga");
    fireEvent.change(input, { target: { value: "a" } });
    await waitFor(() => expect(items().length).toBeGreaterThan(0), T);
    const pane = paneOf(items()[0]);
    pane.scrollTop = 640;
    fireEvent.scroll(pane);
    fireEvent.change(input, { target: { value: "e" } });
    await waitFor(() => expect(pane.scrollTop).toBe(0), T);
  });
});
