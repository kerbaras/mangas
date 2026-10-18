import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useMemo } from "react";
import { describe, expect, it } from "vitest";
import { ChaptersTable } from "@/components/chapters-table";
import { useChapterSelection } from "@/hooks/use-chapter-selection";
import { selectionSummary } from "@/lib/chapters";
import type { Chapter } from "@/lib/ipc";

const chapters: Chapter[] = [1, 2, 3, 4].map((n) => ({
  id: `c${n}`,
  manga_id: "m",
  title: `Chapter ${n}`,
  language: "en",
  volume: "1",
  number: String(n),
  local_path: n === 1 ? "/data/1" : null,
  group: n === 2 ? "Asura Scans" : null,
}));

function Harness() {
  const ids = useMemo(() => chapters.map((c) => c.id), []);
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
