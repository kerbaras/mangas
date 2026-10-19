import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ErrorAlert } from "@/components/error-alert";
import { SearchGroupView } from "@/components/search-group";
import type { ProviderInfo } from "@/lib/ipc";

const NO_CERTS =
  "network: no CA certificates found on this system, so secure connections can't be checked; install the ca-certificates package (e.g. `sudo apt install ca-certificates`)";
const UNREACHABLE = "network: error sending request: dns error: no such host";

const provider: ProviderInfo = {
  id: "mangadex",
  name: "MangaDex",
  site: "https://mangadex.org",
  languages: ["en"],
  capabilities: ["search"],
  needs_browser_session: false,
  icon_url: null,
};

function unreachable(reason: string) {
  render(
    <SearchGroupView
      provider={provider}
      state={{
        kind: "done",
        group: { provider: "mangadex", status: { state: "unreachable", reason }, items: [] },
      }}
      filtered={false}
      onRetry={vi.fn()}
      onShowAll={vi.fn()}
      followed={new Set()}
      busy={null}
      onFollow={vi.fn()}
      onOpen={vi.fn()}
    />,
  );
}

describe("search group failure", () => {
  it("shows the ca-certificates hint and no Try again without certificates", async () => {
    unreachable(NO_CERTS);
    const text = screen.getByText(/Install the ca-certificates package/);
    expect(text.textContent).not.toContain("no CA certificates found");
    expect(text).not.toHaveAttribute("title");
    await userEvent.setup().hover(text);
    expect(await screen.findByRole("tooltip")).toHaveTextContent(NO_CERTS);
    expect(text).not.toHaveClass("truncate");
    expect(screen.queryByRole("button", { name: /try again/i })).toBeNull();
  });

  it("keeps Try again for an unreachable host", () => {
    unreachable(UNREACHABLE);
    expect(screen.getByText(/Couldn't reach MangaDex: Couldn't reach the site/)).toBeTruthy();
    expect(screen.getByRole("button", { name: /try again/i })).toBeTruthy();
  });
});

describe("ErrorAlert retry", () => {
  it("offers Try again only when retrying can help", () => {
    const { rerender } = render(<ErrorAlert title="Failed" message={NO_CERTS} onRetry={vi.fn()} />);
    expect(screen.getByText(/Install the ca-certificates package/)).toBeTruthy();
    expect(screen.queryByRole("button", { name: /try again/i })).toBeNull();
    rerender(<ErrorAlert title="Failed" message={UNREACHABLE} onRetry={vi.fn()} />);
    expect(screen.getByRole("button", { name: /try again/i })).toBeTruthy();
  });
});
