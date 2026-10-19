import { render, screen } from "@testing-library/react";
import type { ReactElement } from "react";
import { describe, expect, it } from "vitest";
import { DevicesPending } from "@/components/devices-pending";
import { ExportProgressPanel } from "@/components/export-progress";
import { SendStatus } from "@/components/send-status";

const TOKEN = "h".repeat(200);

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
];

describe("plain lines with an unbroken token", () => {
  it.each(lines)("wraps %s anywhere instead of running off the window", (_, line) => {
    render(line);
    expect(screen.getByText(new RegExp(TOKEN)).className).toContain("[overflow-wrap:anywhere]");
  });
});
