import { act, render, screen } from "@testing-library/react";
import { Toaster } from "sonner";
import { describe, expect, it } from "vitest";
import type { UpdateRun } from "@/lib/ipc.updates";
import { toastRun } from "@/lib/update-toast";

const NO_CERTS =
  "network: no CA certificates found on this system, so secure connections can't be checked; install the ca-certificates package (e.g. `sudo apt install ca-certificates`)";
const UNREACHABLE = "network: error sending request for url (https://mangadot.net/api)";

const run = (error: string): UpdateRun => ({
  updates: [],
  failures: [{ source: "mangadot", manga_id: "2098", title: "2098", error, challenge: false }],
  checked: 1,
  skipped: 0,
  summary: { chapters: 0, titles: 0, unreachable: 1 },
  queued: 0,
});

describe("update check toast", () => {
  it("shows the ca-certificates hint when certificates are missing", async () => {
    render(<Toaster />);
    act(() => toastRun(run(NO_CERTS)));
    const text = await screen.findByText(/Not checked: 2098 \(mangadot\)/);
    expect(text.textContent).toContain("Install the ca-certificates package");
    expect(text.textContent).not.toContain("no CA certificates found");
  });

  it("shows the plain network reason for an unreachable host", async () => {
    render(<Toaster />);
    act(() => toastRun(run(UNREACHABLE)));
    const text = await screen.findByText(/Not checked: 2098 \(mangadot\)/);
    expect(text.textContent).toContain("Couldn't reach the site");
    expect(text.textContent).not.toContain("ca-certificates");
  });
});
