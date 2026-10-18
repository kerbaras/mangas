import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { LibraryScreen } from "@/screens/library";

vi.mock("@/lib/navigation", () => ({ useNavigation: () => ({ go: vi.fn() }) }));
vi.mock("@/hooks/use-import", () => ({ useImport: () => ({ busy: false, start: vi.fn() }) }));
vi.mock("@/hooks/use-shelf", () => ({
  useShelf: () => ({
    state: { kind: "ready", data: { entries: [], categories: [] } },
    reload: vi.fn(),
    addCategory: vi.fn(),
    renameCategory: vi.fn(),
    removeCategory: vi.fn(),
  }),
}));

describe("empty library", () => {
  it("still lets you set up categories before following anything", () => {
    render(<LibraryScreen />);
    expect(screen.getByText("Nothing on the shelf yet")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "Edit categories" }));
    expect(screen.getByRole("dialog")).toBeTruthy();
  });
});
