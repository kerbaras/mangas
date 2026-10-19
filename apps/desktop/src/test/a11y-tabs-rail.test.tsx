import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { LIBRARY_RESULTS, LibraryToolbar } from "@/components/library-toolbar";
import { downloadsName } from "@/components/nav-rail";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { DEFAULT_VIEW } from "@/lib/library-view";

describe("tabs without panels", () => {
  it("set no aria-controls unless given one", () => {
    render(
      <Tabs value="a">
        <TabsList>
          <TabsTrigger value="a">A</TabsTrigger>
          <TabsTrigger value="b" aria-controls="pane">
            B
          </TabsTrigger>
        </TabsList>
      </Tabs>,
    );
    expect(screen.getByRole("tab", { name: "A" })).not.toHaveAttribute("aria-controls");
    expect(screen.getByRole("tab", { name: "B" })).toHaveAttribute("aria-controls", "pane");
  });

  it("point the library category tabs at the results pane", () => {
    const categories = [{ id: 1, name: "Reading" }];
    render(
      <LibraryToolbar
        view={DEFAULT_VIEW}
        entries={[]}
        categories={categories}
        onChange={() => {}}
        onEditCategories={() => {}}
      />,
    );
    for (const tab of screen.getAllByRole("tab"))
      expect(tab).toHaveAttribute("aria-controls", LIBRARY_RESULTS);
  });
});

describe("Downloads rail name", () => {
  it("includes the badge count", () => {
    expect(downloadsName({ pending: 3, held: false })).toBe("Downloads, 3 pending");
    expect(downloadsName({ pending: 2, held: true })).toBe("Downloads (paused), 2 waiting");
    expect(downloadsName({ pending: 0, held: false })).toBe("Downloads");
    expect(downloadsName({ pending: 0, held: true })).toBe("Downloads (paused)");
  });
});
