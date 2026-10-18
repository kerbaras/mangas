import { act, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { TooltipProvider } from "@/components/ui/tooltip";
import { DownloadsProvider } from "@/hooks/use-downloads";
import { type Backend, setBackend } from "@/lib/ipc";
import { type DownloadJob, type QueueEvent, UNLISTED } from "@/lib/ipc.downloads";
import { DownloadsScreen } from "@/screens/downloads";
import { makeJob } from "./job-fixture";

vi.mock("@/lib/notify", () => ({ notifyError: vi.fn() }));
afterEach(() => setBackend(null));

const jobs: DownloadJob[] = [
  makeJob({ id: 1, state: "running", pages_done: 5, pages_total: 20, number: "1" }),
  makeJob({ id: 2, state: "queued", number: "2" }),
  makeJob({ id: 3, state: "failed", error: "HTTP 429 from source", number: "3" }),
  makeJob({ id: 4, state: "failed", error: UNLISTED, number: "4" }),
  makeJob({ id: 5, state: "cancelled", number: "5" }),
  makeJob({ id: 6, state: "done", local_path: "/d/6", pages_total: 12, number: "6" }),
];

function setup(paused = false) {
  let emit: (e: QueueEvent) => void = () => {};
  const backend = {
    listDownloads: async () => jobs,
    queueStatus: async () => ({ paused, held: false }),
    onQueueEvent: async (cb: (e: QueueEvent) => void) => {
      emit = cb;
      return () => {};
    },
    resumeDownload: vi.fn(async () => []),
    dismissDownload: vi.fn(async () => {}),
    clearDownloads: vi.fn(async () => 2),
    reportError: async () => {},
  };
  setBackend(backend as unknown as Backend);
  render(
    <TooltipProvider>
      <DownloadsProvider>
        <DownloadsScreen />
      </DownloadsProvider>
    </TooltipProvider>,
  );
  return { backend, emit: (e: QueueEvent) => emit(e) };
}

const section = (name: string) => screen.getByRole("region", { name: new RegExp(`^${name}`) });

describe("Downloads screen", () => {
  it("shows every section with a state label on each row", async () => {
    setup();
    expect(await screen.findByText("Kaiju · Chapter 1")).toBeInTheDocument();
    expect(within(section("Active")).getByText("Downloading")).toBeInTheDocument();
    expect(within(section("Active")).getByText("5/20 pages")).toBeInTheDocument();
    expect(within(section("Queued")).getByRole("listitem")).toHaveTextContent("Queued");
    const failed = within(section("Failed"));
    expect(failed.getByText("HTTP 429 from source")).toBeInTheDocument();
    expect(failed.getByText("No longer listed")).toBeInTheDocument();
    expect(within(section("Completed")).getByText("Cancelled")).toBeInTheDocument();
    expect(within(section("Completed")).getByText("Done")).toBeInTheDocument();
    expect(screen.getByText("1 active · 1 queued · 2 failed")).toBeInTheDocument();
  });

  it("clears an unlisted chapter and completed jobs", async () => {
    const { backend } = setup();
    const row = (await screen.findByText("Kaiju · Chapter 4")).closest("li") as HTMLElement;
    await userEvent.click(within(row).getByRole("button", { name: "Clear" }));
    expect(backend.dismissDownload).toHaveBeenCalledWith(4);
    await userEvent.click(screen.getByRole("button", { name: /Clear completed/ }));
    expect(backend.clearDownloads).toHaveBeenCalled();
  });

  it("shows the paused banner with Resume, and follows gate events", async () => {
    const { backend, emit } = setup(true);
    expect(await screen.findByText("Queue paused")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Resume" }));
    expect(backend.resumeDownload).toHaveBeenCalledWith(null);
    await act(async () => {
      emit({ kind: "gate", paused: false, held: true });
      await new Promise((r) => setTimeout(r, 150));
    });
    expect(screen.queryByText("Queue paused")).toBeNull();
    expect(screen.getByText("Queue held while the data folder moves")).toBeInTheDocument();
  });
});
