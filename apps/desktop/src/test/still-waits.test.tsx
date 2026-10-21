import { act, render, screen } from "@testing-library/react";
import { toast } from "sonner";
import { describe, expect, it, vi } from "vitest";
import { DescriptionSkeleton } from "@/components/description-skeleton";
import { ImportMenu } from "@/components/import-menu";
import { ProviderChips } from "@/components/provider-chips";
import { SearchGroupView } from "@/components/search-group";
import { Toaster } from "@/components/ui/sonner";
import type { ProviderInfo } from "@/lib/ipc";

// Waits of unknown length (an import, a search, a description, a loading toast) show still
// indicators: an endless spin or pulse keeps WebKitGTK repainting the window without a GPU.
const ANIMATED = ".animate-spin, .animate-pulse, .sonner-loading-bar";

const provider: ProviderInfo = {
  id: "mangadex",
  name: "MangaDex",
  site: "https://mangadex.org",
  languages: ["en"],
  capabilities: ["search"],
  needs_browser_session: false,
  icon_url: null,
};

describe("still indicators for open-ended waits", () => {
  it("labels a running import without spinning", () => {
    const { container } = render(<ImportMenu busy onPick={vi.fn()} />);
    const button = screen.getByRole("button", { name: "Importing…" });
    expect(button.getAttribute("aria-busy")).toBe("true");
    expect(container.querySelector(ANIMATED)).toBeNull();
  });

  it("marks a searching provider chip and group with a still hourglass", () => {
    const { container } = render(
      <>
        <ProviderChips
          providers={[provider]}
          groups={{ mangadex: { kind: "searching" } }}
          value="all"
          onChange={vi.fn()}
        />
        <SearchGroupView
          provider={provider}
          state={undefined}
          filtered={false}
          onRetry={vi.fn()}
          onShowAll={vi.fn()}
          followed={new Set()}
          busy={null}
          onFollow={vi.fn()}
          onOpen={vi.fn()}
        />
      </>,
    );
    expect(screen.getByLabelText("searching")).toBeTruthy();
    expect(screen.getByText("Searching…")).toBeTruthy();
    expect(container.querySelector(ANIMATED)).toBeNull();
  });

  it("keeps the description skeleton still", () => {
    const { container } = render(<DescriptionSkeleton />);
    expect(screen.getByRole("status", { name: "Loading details" })).toBeTruthy();
    expect(container.querySelector(ANIMATED)).toBeNull();
  });

  it("gives a loading toast a still hourglass", async () => {
    // jsdom has no matchMedia; the Toaster follows the OS colour scheme through it.
    const query = { matches: false, addEventListener: vi.fn(), removeEventListener: vi.fn() };
    vi.stubGlobal(
      "matchMedia",
      vi.fn(() => query),
    );
    render(<Toaster />);
    act(() => void toast.loading("Importing…"));
    expect(await screen.findByText("Importing…")).toBeTruthy();
    expect(document.querySelector(ANIMATED)).toBeNull();
    expect(document.querySelector("[data-sonner-toast] .lucide-hourglass")).not.toBeNull();
    act(() => toast.dismiss());
    vi.unstubAllGlobals();
  });
});
