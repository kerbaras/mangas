import { fireEvent, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { describe, expect, it } from "vitest";
import { ChaptersTable } from "@/components/chapters-table";
import { useChapterSelection } from "@/hooks/use-chapter-selection";
import { selectableIds, selectionSummary } from "@/lib/chapters";
import type { Chapter } from "@/lib/ipc";

const PANE = 360;
const chapters: Chapter[] = Array.from({ length: 617 }, (_, i) => ({
  id: `c${i + 1}`,
  manga_id: "m",
  title: `Chapter ${i + 1}`,
  language: "en",
  volume: null,
  number: String(i + 1),
  local_path: null,
  group: null,
}));

/** jsdom has no layout: give the pane a height and an element `scrollTo`. */
const sized = (el: HTMLDivElement | null) => {
  if (!el) return;
  Object.defineProperty(el, "offsetHeight", { configurable: true, value: PANE });
  Object.defineProperty(el, "offsetWidth", { configurable: true, value: 800 });
  el.scrollTo = ((opts: ScrollToOptions) => {
    el.scrollTop = opts.top ?? el.scrollTop;
  }) as typeof el.scrollTo;
};

function Harness() {
  const pane = useRef<HTMLDivElement>(null);
  const ids = useMemo(() => selectableIds(chapters), []);
  const sel = useChapterSelection(ids);
  // Details mounts its pane first and the table once the chapters load.
  const [loaded, setLoaded] = useState(false);
  useLayoutEffect(() => sized(pane.current), []);
  useEffect(() => setLoaded(true), []);
  return (
    <>
      <p data-testid="summary">{selectionSummary(chapters, sel.selected)}</p>
      <div data-testid="pane" ref={pane} className="overflow-auto">
        {loaded && (
          <ChaptersTable
            chapters={chapters}
            selected={sel.selected}
            downloads={new Map()}
            onToggle={sel.onToggle}
            onToggleAll={sel.onToggleAll}
            scrollRef={pane}
          />
        )}
      </div>
    </>
  );
}

const rows = () => document.querySelectorAll("tr[data-nav-item]");
const scrollTo = (top: number) => {
  const pane = screen.getByTestId("pane");
  pane.scrollTop = top;
  fireEvent.scroll(pane);
};

describe("ChaptersTable window", () => {
  it("renders a screenful of rows with gaps standing in for the rest", () => {
    render(<Harness />);
    expect(rows().length).toBeGreaterThan(5);
    expect(rows().length).toBeLessThan(PANE / 36 + 2 * 12 + 2);
    expect(screen.getByRole("table")).toHaveAttribute("aria-rowcount", "618");
    expect(screen.getByText("Chapter 1").closest("tr")).toHaveAttribute("aria-rowindex", "2");
    const gap = document.querySelector<HTMLElement>("tr[aria-hidden] td");
    expect(gap?.style.height).toBe(`${(617 - rows().length) * 36}px`);
  });

  it("scrolling far down renders the rows there", () => {
    render(<Harness />);
    scrollTo(600 * 36);
    expect(screen.getByText("Chapter 610")).toBeTruthy();
    expect(screen.queryByText("Chapter 1")).toBeNull();
    expect(screen.getByText("Chapter 610").closest("tr")).toHaveAttribute("aria-rowindex", "611");
  });

  it("keeps the focused row mounted while it is scrolled away", () => {
    render(<Harness />);
    const row = screen.getByText("Chapter 3").closest("tr");
    row?.focus();
    expect(document.activeElement).toBe(row);
    scrollTo(300 * 36);
    expect(screen.getByText("Chapter 300")).toBeTruthy();
    expect(document.activeElement).toBe(row);
    expect(row?.isConnected).toBe(true);
  });

  it("select all reaches the rows that aren't rendered", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.click(screen.getByLabelText("Select all chapters"));
    expect(screen.getByTestId("summary")).toHaveTextContent("617 selected, 617 to download");
    scrollTo(500 * 36);
    expect(screen.getByLabelText("Select chapter 505")).toBeChecked();
  });
});
