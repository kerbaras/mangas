import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AppUpdateBanner } from "@/components/app-update-banner";
import { AppUpdateSettings } from "@/components/app-update-settings";
import type { AppUpdateState } from "@/lib/app-update";
import { parseReleaseNotes } from "@/lib/release-notes";

// What scripts/release-notes.sh prints for a stable tag (CHANGELOG.md section, CRLF-safe).
const NOTES = [
  "## [0.6.0] - 2026-10-22",
  "",
  "### Added",
  "",
  "- **export:** Warn over the Send-to-Kindle limit (#287)",
  "- Rename the product from Mangas to mangas (#294)",
  "",
  "### Fixed",
  "",
  "- **cli,desktop:** Update checks name the network cause (#348)",
].join("\r\n");

const store = vi.hoisted(() => ({ state: null as unknown }));
vi.mock("@/lib/app-update", async (original) => ({
  ...(await original<typeof import("@/lib/app-update")>()),
  useAppUpdate: () => store.state,
}));
vi.mock("@/lib/settings", () => ({
  useSettings: () => [{ appUpdateChannel: "stable" }, vi.fn()],
}));
vi.mock("@/lib/app-version", () => ({ appVersion: async () => "0.5.0" }));

const withNotes = (notes: string | null): AppUpdateState => ({
  check: {
    kind: "done",
    status: {
      kind: "available",
      current: "0.5.0",
      version: "0.6.0",
      notes,
      date: null,
      channel: "stable",
    },
  },
  install: { kind: "idle" },
  dismissed: false,
});

describe("parseReleaseNotes", () => {
  it("drops the version heading and keeps groups, items and bold scopes", () => {
    const blocks = parseReleaseNotes(NOTES);
    expect(blocks.map((b) => b.kind)).toEqual(["heading", "list", "heading", "list"]);
    expect(blocks[0]).toMatchObject({ text: "Added" });
    const list = blocks[1];
    if (list?.kind !== "list") throw new Error("expected a list");
    expect(list.items).toHaveLength(2);
    expect(list.items[0]?.spans.map((s) => [s.text, s.bold])).toEqual([
      ["export:", true],
      [" Warn over the Send-to-Kindle limit (#287)", false],
    ]);
  });

  it("has nothing to show for no notes or only the heading", () => {
    expect(parseReleaseNotes(null)).toEqual([]);
    expect(parseReleaseNotes("## [0.6.0] - 2026-10-22")).toEqual([]);
  });
});

describe("What's new for an available update", () => {
  beforeEach(() => {
    store.state = null;
  });

  for (const [where, ui] of [
    ["the banner", <AppUpdateBanner key="b" />],
    ["Settings → About", <AppUpdateSettings key="s" />],
  ] as const) {
    it(`${where} lists every group and entry`, async () => {
      store.state = withNotes(NOTES);
      render(ui);
      await userEvent.setup().click(screen.getByText("What's new in v0.6.0"));
      expect(screen.getByRole("heading", { name: "Added" })).toBeVisible();
      expect(screen.getByRole("heading", { name: "Fixed" })).toBeVisible();
      const items = screen.getAllByRole("listitem");
      expect(items.map((li) => li.textContent)).toEqual([
        "export: Warn over the Send-to-Kindle limit (#287)",
        "Rename the product from Mangas to mangas (#294)",
        "cli,desktop: Update checks name the network cause (#348)",
      ]);
      expect(within(items[0] as HTMLElement).getByText("export:").tagName).toBe("STRONG");
      expect(screen.queryByText(/## \[0\.6\.0\]/)).toBeNull();
    });

    it(`${where} shows no What's new for heading-only notes`, () => {
      store.state = withNotes("## [0.6.0] - 2026-10-22");
      render(ui);
      expect(screen.getByText(/v0\.6\.0 is available/)).toBeInTheDocument();
      expect(screen.queryByText(/What's new/)).toBeNull();
    });
  }
});
