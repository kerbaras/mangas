import { describeFailure } from "./errors";
import { connectProvider, getSettings, onProviderConnectEnd, type ProviderStatus } from "./ipc";

const POLL_MS = 2000;
const TIMEOUT_MS = 180_000;

const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

const savedAt = async (id: string) => (await getSettings()).provider_sessions[id]?.saved_at ?? null;

/**
 * Resolves once a session newer than `previous` (the `saved_at` seen before connecting) is
 * stored for `id`, or `false` after the timeout. An expired session left from an earlier
 * connection therefore does not count.
 */
export async function waitForSession(
  id: string,
  previous: string | null,
  timeoutMs = TIMEOUT_MS,
  signal?: AbortSignal,
): Promise<boolean> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline && !signal?.aborted) {
    await sleep(POLL_MS);
    if (await isFresh(id, previous)) return true;
  }
  return false;
}

const isFresh = async (id: string, previous: string | null) => {
  const current = await savedAt(id);
  return Boolean(current && current !== previous);
};

export type ConnectOutcome = "connected" | "cancelled" | "timed_out";

/**
 * Opens the Connect window and waits for the verification to produce a fresh session. Ends as
 * soon as the window closes ("cancelled" unless it saved a session first); rejects with the
 * reason when its page fails to load (TLS, DNS), for `describeFailure`.
 */
export async function connectAndWait(id: string, timeoutMs = TIMEOUT_MS): Promise<ConnectOutcome> {
  const previous = await savedAt(id);
  const stop = new AbortController();
  let settle: (o: Promise<ConnectOutcome>) => void = () => {};
  const ended = new Promise<ConnectOutcome>((resolve) => {
    settle = resolve;
  });
  const unlisten = await onProviderConnectEnd((end) => {
    if (end.provider !== id) return;
    settle(
      end.kind === "failed"
        ? Promise.reject(new Error(end.reason))
        : isFresh(id, previous).then((ok) => (ok ? "connected" : "cancelled")),
    );
  });
  try {
    await connectProvider(id);
    const polled = waitForSession(id, previous, timeoutMs, stop.signal).then(
      (ok): ConnectOutcome => (ok ? "connected" : "timed_out"),
    );
    return await Promise.race([polled, ended]);
  } finally {
    stop.abort();
    unlisten();
  }
}

/** The line under the providers list after Connect; null once connected. */
export function connectNote(outcome: ConnectOutcome, name: string): string | null {
  if (outcome === "connected") return null;
  if (outcome === "cancelled") return "Connection cancelled";
  return `No session captured for ${name}; close its window and try again.`;
}

type Detail = {
  site: string;
  needsSession: boolean;
  /** `saved_at` of the stored session; undefined when there is none. */
  savedAt: string | null | undefined;
  status: ProviderStatus | undefined;
  checkedAt: number | undefined;
  /** Formats `checkedAt` ("2 min ago"). */
  ago: (at: number) => string;
};

/** Second line of a provider row: host, session, when it was checked and why it failed. */
export function providerDetail(d: Detail): string {
  const parts = [d.site.replace(/^https?:\/\//, "")];
  if (d.needsSession)
    parts.push(
      d.savedAt === undefined
        ? "not connected"
        : `connected${d.savedAt ? ` ${new Date(d.savedAt).toLocaleDateString()}` : ""}`,
    );
  if (d.status?.state === "needs_session" && d.savedAt !== undefined) parts.push("session expired");
  if (d.status?.state === "unreachable") parts.push(describeFailure(d.status.reason).label);
  parts.push(d.checkedAt === undefined ? "checking…" : `checked ${d.ago(d.checkedAt)}`);
  return parts.join(" · ");
}

export type Tone = "ok" | "warn" | "bad" | "pending";

/** Badge for a provider: `undefined` while a check runs. */
export function statusBadge(status: ProviderStatus | undefined): { label: string; tone: Tone } {
  if (!status) return { label: "Checking", tone: "pending" };
  switch (status.state) {
    case "ready":
      return { label: "Ready", tone: "ok" };
    case "needs_session":
      return { label: "Needs connection", tone: "warn" };
    case "unreachable":
      return { label: "Unreachable", tone: "bad" };
  }
}
