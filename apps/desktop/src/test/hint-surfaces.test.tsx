import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { Hint } from "@/components/hint";
import { ProviderRow } from "@/components/provider-row";
import { ProviderStatusBadge } from "@/components/provider-status-badge";
import { SearchGroupView } from "@/components/search-group";
import type { ProviderInfo } from "@/lib/ipc";
import { Description } from "@/lib/notify";

const REASON = "network: error sending request for url (https://api.example.org/manga?title=x)";
const unreachable = { state: "unreachable" as const, reason: REASON };
const provider: ProviderInfo = {
  id: "mangadex",
  name: "MangaDex",
  site: "https://mangadex.org",
  languages: ["en"],
  capabilities: ["search", "chapters", "download"],
  needs_browser_session: false,
  icon_url: null,
};
const noop = () => {};

const surfaces = {
  "the provider status badge": () => <ProviderStatusBadge status={unreachable} />,
  "a Search provider failure": () => (
    <SearchGroupView
      provider={provider}
      state={{ kind: "done", group: { provider: "mangadex", status: unreachable, items: [] } }}
      filtered={false}
      onRetry={noop}
      onShowAll={noop}
      followed={new Set()}
      busy={null}
      onFollow={noop}
      onOpen={noop}
    />
  ),
  "the toast detail line": () => <Description hint={null} detail={REASON} />,
};

describe("failure reasons use the shared Hint, not a native title", () => {
  it.each(Object.entries(surfaces))("%s shows on hover and on Tab", async (_, Surface) => {
    render(<Surface />);
    expect(document.querySelector("[title]")).toBeNull();
    const user = userEvent.setup();
    await user.tab();
    expect(await screen.findByRole("tooltip")).toHaveTextContent(REASON);
    await user.keyboard("{Escape}");
    await user.tab({ shift: true });
    const triggers = [...document.querySelectorAll<HTMLElement>('[tabindex="0"]')];
    const reason = triggers.find((t) => t.textContent?.includes(REASON)) ?? triggers[0];
    await user.hover(reason);
    expect(await screen.findByRole("tooltip")).toHaveTextContent(REASON);
  });

  it("the tooltip stacks above sonner's toaster (z-index 999999999)", async () => {
    render(<Description hint={null} detail={REASON} />);
    await userEvent.setup().tab();
    await screen.findByRole("tooltip");
    const z = document.querySelector("[data-side]")?.className.match(/z-\[(\d+)\]/)?.[1];
    expect(Number(z)).toBeGreaterThan(999_999_999);
  });

  it("the Settings provider row puts the reason in the badge's Hint and no title", async () => {
    render(
      <ProviderRow
        provider={provider}
        session={undefined}
        check={{ status: unreachable, checkedAt: Date.now() }}
        enabled
        connecting={false}
        now={Date.now()}
        onEnable={noop}
        onConnect={noop}
        onDisconnect={noop}
      />,
    );
    expect(document.querySelector("[title]")).toBeNull();
    await userEvent.setup().hover(screen.getByText("Unreachable"));
    expect(await screen.findByRole("tooltip")).toHaveTextContent(REASON);
  });
});

describe("truncated text inside a control", () => {
  it("opens when the control gets keyboard focus", async () => {
    const user = userEvent.setup();
    render(
      <button type="button">
        <Hint hint="A very long manga title" focusable={false} truncated>
          <p>A very long…</p>
        </Hint>
      </button>,
    );
    const p = screen.getByText("A very long…");
    Object.defineProperty(p, "scrollWidth", { configurable: true, value: 200 });
    Object.defineProperty(p, "clientWidth", { configurable: true, value: 100 });
    await user.tab();
    expect(await screen.findByRole("tooltip")).toHaveTextContent("A very long manga title");
  });
});
