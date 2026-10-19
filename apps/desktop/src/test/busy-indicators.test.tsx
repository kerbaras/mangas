import { act, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { CheckUpdatesButton } from "@/components/check-updates-button";
import { ProviderStatusBadge } from "@/components/provider-status-badge";
import { type Backend, setBackend } from "@/lib/ipc";
import type { UpdateRun } from "@/lib/ipc.updates";
import { runCheck } from "@/lib/updates";

// Open-ended waits (a library-wide check, a browser sign-in) show still icons: an endless spin
// keeps WebKitGTK repainting at ~100 % CPU without a GPU for as long as the wait lasts.
describe("busy indicators for long waits", () => {
  afterEach(() => setBackend(null));

  it("labels a running library check without spinning", async () => {
    let finish: (run: UpdateRun) => void = () => {};
    const checkUpdates = vi.fn(() => new Promise<UpdateRun>((r) => (finish = r)));
    setBackend({ checkUpdates } as unknown as Backend);
    const { container } = render(<CheckUpdatesButton />);
    let run: Promise<UpdateRun> = Promise.resolve({} as UpdateRun);
    act(() => {
      run = runCheck(true);
    });
    const button = screen.getByRole("button", { name: "Checking…" });
    expect(button.getAttribute("aria-busy")).toBe("true");
    expect(container.querySelector(".animate-spin")).toBeNull();
    await vi.waitFor(() => expect(checkUpdates).toHaveBeenCalled());
    await act(async () => {
      finish({ updates: [], errors: [] } as unknown as UpdateRun);
      await run;
    });
    expect(screen.getByRole("button", { name: "Check for updates" })).toBeTruthy();
  });

  it("shows a still dot while a provider is checking or connecting", () => {
    const { container } = render(<ProviderStatusBadge status={undefined} label="Connecting" />);
    expect(screen.getByText("Connecting")).toBeTruthy();
    expect(container.querySelector(".animate-spin")).toBeNull();
    expect(container.querySelector(".bg-ink-3")).not.toBeNull();
  });
});
