import { afterEach, describe, expect, it, vi } from "vitest";
import { notifyError, TOAST_EXIT_MS } from "@/lib/notify";

type Action = { label: string; onClick: () => void };
const error = vi.hoisted(() => vi.fn((_title: string, _opts: { action?: Action }) => {}));
vi.mock("sonner", () => ({ toast: { error, warning: vi.fn() } }));

afterEach(() => {
  vi.useRealTimers();
  vi.clearAllMocks();
});

describe("notifyError", () => {
  it("retries only after the dismissed toast has left", () => {
    vi.useFakeTimers();
    const retry = vi.fn();
    notifyError("Search failed", new Error("network: offline"), { retry, id: "search" });

    const action = error.mock.calls[0]?.[1].action;
    expect(action?.label).toBe("Retry");
    action?.onClick();
    expect(retry).not.toHaveBeenCalled();
    vi.advanceTimersByTime(TOAST_EXIT_MS);
    expect(retry).toHaveBeenCalledOnce();
  });

  it("offers no Retry for a failure a retry cannot fix, and keeps it for an unreachable host", () => {
    const retry = vi.fn();
    notifyError(
      "Download failed",
      new Error(
        "network: no CA certificates found on this system, so secure connections can't be checked; install the ca-certificates package (e.g. `sudo apt install ca-certificates`)",
      ),
      { retry, id: "d" },
    );
    notifyError("Download failed", new Error("network: dns error: no such host"), { retry });
    expect(error.mock.calls[0]?.[1].action).toBeNull();
    expect(error.mock.calls[1]?.[1].action?.label).toBe("Retry");
  });
});
