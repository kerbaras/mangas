import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useMemo } from "react";
import { describe, expect, it } from "vitest";
import { ChaptersTable } from "@/components/chapters-table";
import { useChapterSelection } from "@/hooks/use-chapter-selection";
import { selectableIds, selectionSummary } from "@/lib/chapters";
import type { Chapter } from "@/lib/ipc";

const base: Chapter[] = [1, 2, 3, 4].map((n) => ({
  id: `c${n}`,
  manga_id: "m",
  title: `Chapter ${n}`,
  language: "en",
  volume: "1",
  number: String(n),
  local_path: n === 1 ? "/data/1" : null,
  group: n === 2 ? "Asura Scans" : null,
}));
const url = "https://mangaplus.shueisha.co.jp/viewer/1000000";
const unavailable: Chapter[] = [
  { ...base[3], id: "c5", number: "5", title: "Chapter 5", availability: { kind: "locked" } },
  {
    ...base[3],
    id: "c6",
    number: "6",
    title: "Chapter 6",
    availability: { kind: "external", url },
  },
];
let chapters = base;

function Harness() {
  const ids = useMemo(() => selectableIds(chapters), []);
  const sel = useChapterSelection(ids);
  return (
    <>
      <p data-testid="summary">{selectionSummary(chapters, sel.selected)}</p>
      <ChaptersTable
        chapters={chapters}
        selected={sel.selected}
        downloads={new Map()}
        onToggle={sel.onToggle}
        onToggleAll={sel.onToggleAll}
      />
    </>
  );
}

describe("ChaptersTable selection", () => {
  it("click selects, shift-click extends, header checkbox toggles all", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    const summary = screen.getByTestId("summary");
    expect(summary).toHaveTextContent("No chapters selected");

    await user.click(screen.getByText("Chapter 1"));
    expect(summary).toHaveTextContent("1 selected, all downloaded");

    await user.keyboard("{Shift>}");
    await user.click(screen.getByText("Chapter 3"));
    await user.keyboard("{/Shift}");
    expect(summary).toHaveTextContent("3 selected, 2 to download");

    await user.click(screen.getByLabelText("Select all chapters"));
    expect(summary).toHaveTextContent("4 selected, 3 to download");
    await user.click(screen.getByLabelText("Select all chapters"));
    expect(summary).toHaveTextContent("No chapters selected");
  });

  it("the row checkbox toggles without selecting a range", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.click(screen.getByLabelText("Select chapter 2"));
    expect(screen.getByLabelText("Select chapter 2")).toBeChecked();
    expect(screen.getByTestId("summary")).toHaveTextContent("1 selected, 1 to download");
  });
});

describe("ChaptersTable columns", () => {
  it("shows the scanlation group and language", () => {
    render(<Harness />);
    expect(screen.getByRole("columnheader", { name: "Group" })).toBeTruthy();
    expect(screen.getByText("Asura Scans")).toBeTruthy();
    expect(screen.getAllByText("English")).toHaveLength(4);
  });
});

describe("ChaptersTable availability", () => {
  it("locked and external rows can't be selected and select all skips them", async () => {
    chapters = [...base, ...unavailable];
    const user = userEvent.setup();
    render(<Harness />);
    const summary = screen.getByTestId("summary");
    expect(summary).toHaveTextContent("1 locked, 1 on another site skipped");
    expect(screen.getByLabelText("Select chapter 5")).toBeDisabled();
    expect(screen.getByLabelText("Select chapter 6")).toBeDisabled();
    await user.click(screen.getByText("Chapter 5"));
    expect(summary).toHaveTextContent("No chapters selected");
    await user.click(screen.getByLabelText("Select all chapters"));
    expect(summary).toHaveTextContent("4 selected, 3 to download");
    expect(screen.getByText("Locked")).toBeTruthy();
    expect(screen.getByRole("button", { name: /mangaplus.shueisha.co.jp/ })).toBeTruthy();
    chapters = base;
  });

  it("an external chapter without a link shows a plain badge", () => {
    const linkless: Chapter = { ...unavailable[1], availability: { kind: "external", url: null } };
    chapters = [...base, linkless];
    render(<Harness />);
    expect(screen.getByText("Another site")).toBeTruthy();
    expect(screen.queryByRole("button", { name: /Another site/ })).toBeNull();
    chapters = base;
  });
});
