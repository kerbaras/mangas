import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useMemo } from "react";
import { describe, expect, it } from "vitest";
import { ChaptersTable } from "@/components/chapters-table";
import { useChapterSelection } from "@/hooks/use-chapter-selection";
import { duplicateIds, filterByGroup, groupsOf, sourceRef, splitGroup } from "@/lib/chapter-groups";
import { exportTargets, pickSelected, selectableIds } from "@/lib/chapters";
import type { Chapter } from "@/lib/ipc";

const row = (id: string, number: string, group: string | null, onDisk = false): Chapter => ({
  id,
  manga_id: "2098",
  title: null,
  language: "en",
  volume: null,
  number,
  local_path: onDisk ? `/data/${id}` : null,
  group,
});

// The MangaDot shape from #160: chapter 1 once per group, plus the site's mirror.
const knight: Chapter[] = [
  row("u:1", "1", "K Manga"),
  row("u:2", "1", "Official", true),
  row("c:3", "1", "MangaDot mirror"),
  row("u:4", "2", "K Manga", true),
  row("u:5", "2", "Official"),
  row("u:6", "3", "K Manga"),
];

describe("chapter groups", () => {
  it("counts groups, most chapters first", () => {
    expect(groupsOf([...knight, row("x", "9", null)])).toEqual([
      { name: "K Manga", chapters: 3 },
      { name: "Official", chapters: 2 },
      { name: "MangaDot mirror", chapters: 1 },
    ]);
  });

  it("narrows to one row per chapter number", () => {
    const narrowed = filterByGroup(knight, "K Manga");
    expect(narrowed.map((c) => c.number)).toEqual(["1", "2", "3"]);
    expect(filterByGroup(knight, null)).toBe(knight);
  });

  it("acts only on the rows the group leaves", () => {
    const pool = filterByGroup(knight, "K Manga");
    const everything = new Set(knight.map((c) => c.id));
    expect(pickSelected(pool, everything).map((c) => c.id)).toEqual(["u:1", "u:4", "u:6"]);
    expect(exportTargets(pool, pool, new Set()).map((c) => c.id)).toEqual(["u:4"]);
  });

  it("flags twin rows and shortens their ids", () => {
    const twins = [row("u:1833857", "1", "K Manga"), row("u:1833858", "1", "K Manga")];
    expect([...duplicateIds([...twins, knight[1]])]).toEqual(["u:1833857", "u:1833858"]);
    expect(duplicateIds(knight).size).toBe(0);
    expect(sourceRef("u:1833857")).toBe("#1833857");
    expect(sourceRef("8f2c1a9e-0000-4000-8000-000000000000")).toBe("#8f2c1a9e");
  });
});

function Harness({ chapters }: { chapters: Chapter[] }) {
  const ids = useMemo(() => selectableIds(chapters), [chapters]);
  const sel = useChapterSelection(ids);
  return (
    <>
      <p data-testid="picked">
        {pickSelected(chapters, sel.selected)
          .map((c) => c.id)
          .join(",")}
      </p>
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

describe("chapters table with a group filter", () => {
  it("select all takes only the visible group", async () => {
    render(<Harness chapters={filterByGroup(knight, "Official")} />);
    expect(screen.getAllByRole("row")).toHaveLength(3);
    await userEvent.click(screen.getByLabelText("Select all chapters"));
    expect(screen.getByTestId("picked").textContent).toBe("u:2,u:5");
  });

  it("shows the source id only on twin rows", () => {
    render(
      <Harness chapters={[row("u:7", "4", "K Manga"), row("u:8", "4", "K Manga"), knight[1]]} />,
    );
    expect(screen.getByText("#7")).toBeTruthy();
    expect(screen.getByText("#8")).toBeTruthy();
    expect(screen.queryByText("#2")).toBeNull();
  });
});

describe("splitGroup", () => {
  it("keeps short names whole and always keeps the end of long ones", () => {
    expect(splitGroup("K Manga")).toEqual(["K Manga", ""]);
    expect(splitGroup("All For Manga Group, Wicked House Translation")).toEqual([
      "All For Manga Group, Wicked House ",
      "Translation",
    ]);
    expect(splitGroup("ThePervertedScanner")).toEqual(["ThePerver", "tedScanner"]);
  });
});
