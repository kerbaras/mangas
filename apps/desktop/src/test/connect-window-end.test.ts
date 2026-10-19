import { afterEach, describe, expect, it, vi } from "vitest";
import { describeFailure } from "@/lib/errors";
import { type Backend, type ConnectEnd, setBackend } from "@/lib/ipc";
import { connectAndWait, connectNote } from "@/lib/providers";

const NO_CERTS =
  "no CA certificates found on this system, so secure connections can't be checked; install the ca-certificates package";

/** A backend whose Connect window ends with `end` (after saving a session when `saves`). */
function backend(
  end: { kind: "closed" } | { kind: "failed"; reason: string } | null,
  saves = false,
) {
  let saved: string | null = null;
  let listener: ((e: ConnectEnd) => void) | null = null;
  const unlisten = vi.fn();
  setBackend({
    getSettings: async () => ({ provider_sessions: saved ? { comick: { saved_at: saved } } : {} }),
    onProviderConnectEnd: async (cb: (e: ConnectEnd) => void) => {
      listener = cb;
      return unlisten;
    },
    connectProvider: async () => {
      setTimeout(() => {
        if (saves) saved = "2026-10-19T00:00:00Z";
        listener?.({ provider: "other", kind: "closed" });
        if (end) listener?.({ provider: "comick", ...end } as ConnectEnd);
      }, 10);
    },
  } as unknown as Backend);
  return { unlisten };
}

afterEach(() => setBackend(null));

describe("connectAndWait", () => {
  it("ends at once with cancelled when the window closes without a session", async () => {
    const b = backend({ kind: "closed" });
    const started = Date.now();
    await expect(connectAndWait("comick")).resolves.toBe("cancelled");
    expect(Date.now() - started).toBeLessThan(1000);
    expect(b.unlisten).toHaveBeenCalledOnce();
    expect(connectNote("cancelled", "Comick")).toBe("Connection cancelled");
  });

  it("is connected when the window closed itself after saving the session", async () => {
    backend({ kind: "closed" }, true);
    await expect(connectAndWait("comick")).resolves.toBe("connected");
    expect(connectNote("connected", "Comick")).toBeNull();
  });

  it("rejects with the load failure, which reads through describeFailure", async () => {
    backend({ kind: "failed", reason: NO_CERTS });
    const error = await connectAndWait("comick").catch((e: Error) => e);
    expect(String((error as Error).message)).toBe(NO_CERTS);
    expect(describeFailure(NO_CERTS).label).toMatch(/This system has no CA certificates/);
  });

  it("names a DNS failure as a network problem", async () => {
    const reason = "network: couldn't load https://comick.io/: Error resolving “comick.io”";
    backend({ kind: "failed", reason });
    await expect(connectAndWait("comick")).rejects.toThrow(reason);
    expect(describeFailure(reason).label).toBe(
      "Couldn't reach the site. Check your connection and try again.",
    );
  });

  it("still times out when nothing ends the wait", async () => {
    backend(null);
    await expect(connectAndWait("comick", 50)).resolves.toBe("timed_out");
  });
});

describe("connectAndWait after the window closed", () => {
  it("stops polling: no settings read once the wait is over", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    try {
      backend({ kind: "closed" });
      await expect(connectAndWait("comick")).resolves.toBe("cancelled");
      const reads = vi.fn(async () => {
        throw new Error("settings unreadable");
      });
      setBackend({ getSettings: reads } as unknown as Backend);
      await vi.advanceTimersByTimeAsync(5000); // vitest fails the run on an unhandled rejection
      expect(reads).not.toHaveBeenCalled();
    } finally {
      vi.useRealTimers();
    }
  });
});
