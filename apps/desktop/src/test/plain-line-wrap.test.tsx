import { render, screen } from "@testing-library/react";
import type { ReactElement } from "react";
import { describe, expect, it, vi } from "vitest";
import { AppUpdateSettings } from "@/components/app-update-settings";
import { ConfirmDialog } from "@/components/confirm-dialog";
import { DevicesPending } from "@/components/devices-pending";
import { ExportProgressPanel } from "@/components/export-progress";
import { SendStatus } from "@/components/send-status";
import type { AppUpdateState } from "@/lib/app-update";

const TOKEN = "h".repeat(200);
const WRAP = "[overflow-wrap:anywhere]";

vi.mock("@/lib/app-update", async (original) => ({
  ...(await original<typeof import("@/lib/app-update")>()),
  useAppUpdate: (): AppUpdateState => ({
    check: { kind: "failed", error: `https://updates.example/${TOKEN}/latest.json` },
    install: { kind: "idle" },
    dismissed: false,
  }),
}));
vi.mock("@/lib/settings", () => ({
  useSettings: () => [{ appUpdateChannel: "stable" }, vi.fn()],
}));
vi.mock("@/lib/app-version", () => ({ appVersion: async () => "0.5.0" }));

const lines: [string, ReactElement][] = [
  ["a device list error", <DevicesPending key="d" state={{ kind: "error", message: TOKEN }} />],
  [
    "an export error",
    <ExportProgressPanel key="e" state={{ kind: "error", message: TOKEN }} onReset={() => {}} />,
  ],
  [
    "a delivery report",
    <SendStatus key="s" state={{ kind: "done", via: "email", message: TOKEN }} />,
  ],
  [
    "a dialog description with a path",
    <ConfirmDialog
      key="c"
      open
      title="Move the data folder?"
      description={`Chapters are copied to /${TOKEN} and checked.`}
      confirm="Move"
      onConfirm={() => {}}
      onCancel={() => {}}
    />,
  ],
];

describe("plain lines with an unbroken token", () => {
  it.each(lines)("wraps %s anywhere instead of running off the window", (_, line) => {
    render(line);
    expect(screen.getByText(new RegExp(TOKEN)).className).toContain(WRAP);
  });

  it("wraps the update check failure with a long feed URL", () => {
    render(<AppUpdateSettings />);
    const line = screen.getByRole("alert");
    expect(line).toHaveTextContent(TOKEN);
    expect(line.className).toContain(WRAP);
  });
});
