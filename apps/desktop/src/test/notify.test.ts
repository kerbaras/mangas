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
});
