import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AppUpdateBanner } from "@/components/app-update-banner";
import { AppUpdateSettings } from "@/components/app-update-settings";
import type { AppUpdateState } from "@/lib/app-update";

const REASON =
  "network: error sending request for url (https://github.com/kerbaras/mangas/latest.json)";
const store = vi.hoisted(() => ({ state: null as unknown }));

vi.mock("@/lib/app-update", async (original) => ({
  ...(await original<typeof import("@/lib/app-update")>()),
  useAppUpdate: () => store.state,
}));
vi.mock("@/lib/settings", () => ({
  useSettings: () => [{ appUpdateChannel: "stable" }, vi.fn()],
}));
vi.mock("@/lib/app-version", () => ({ appVersion: async () => "0.5.0" }));

const available = {
  kind: "done" as const,
  status: {
    kind: "available" as const,
    current: "0.4.0",
    version: "0.5.0",
    notes: null,
    date: null,
    channel: "stable" as const,
  },
};
const failed = { kind: "failed" as const, error: REASON };
const idle = { kind: "idle" as const };

describe("app update failures use the shared Hint, not a native title", () => {
  beforeEach(() => {
    store.state = null;
  });

  it("the banner's install failure", async () => {
    store.state = { check: available, install: failed, dismissed: false } satisfies AppUpdateState;
    render(<AppUpdateBanner />);
    const line = screen.getByText(/Update failed/);
    expect(line).not.toHaveAttribute("title");
    await userEvent.setup().hover(line);
    expect(await screen.findByRole("tooltip")).toHaveTextContent(REASON);
  });

  it("the Settings check failure", async () => {
    store.state = { check: failed, install: idle, dismissed: false } satisfies AppUpdateState;
    render(<AppUpdateSettings />);
    const line = screen.getByRole("alert");
    expect(line).not.toHaveAttribute("title");
    await userEvent.setup().hover(line);
    expect(await screen.findByRole("tooltip")).toHaveTextContent(REASON);
  });
});
