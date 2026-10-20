import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { AppUpdateSettings } from "@/components/app-update-settings";

vi.mock("@/lib/app-update", async (original) => ({
  ...(await original<typeof import("@/lib/app-update")>()),
  useAppUpdate: () => ({ check: { kind: "checking" }, install: { kind: "idle" } }),
}));
vi.mock("@/lib/settings", () => ({
  useSettings: () => [{ appUpdateChannel: "stable" }, vi.fn()],
}));
vi.mock("@/lib/app-version", () => ({ appVersion: async () => "0.5.0" }));

// A spin here kept WebKitGTK at ~240 % CPU without a GPU for up to the 20 s request timeout.
describe("About → Check for updates", () => {
  it("shows a still icon and Checking… while the check runs", () => {
    const { container } = render(<AppUpdateSettings />);
    const button = screen.getByRole("button", { name: "Check for updates" });
    expect(button).toBeDisabled();
    expect(button.getAttribute("aria-busy")).toBe("true");
    expect(screen.getByText("Checking…")).toBeTruthy();
    expect(container.querySelector(".animate-spin")).toBeNull();
  });
});
