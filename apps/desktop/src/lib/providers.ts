import { connectProvider, getSettings, type ProviderStatus } from "./ipc";

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
): Promise<boolean> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    await sleep(POLL_MS);
    const current = await savedAt(id);
    if (current && current !== previous) return true;
  }
  return false;
}

/** Opens the Connect window and waits for the verification to produce a fresh session. */
export async function connectAndWait(id: string): Promise<boolean> {
  const previous = await savedAt(id);
  await connectProvider(id);
  return waitForSession(id, previous);
}

/** Status line shown next to a provider's site; `status` is undefined while the probe runs. */
export function providerStatusText(
  needsSession: boolean,
  status: ProviderStatus | undefined,
  savedAt: string | null | undefined,
): string {
  const connected =
    savedAt !== undefined
      ? `connected${savedAt ? ` ${new Date(savedAt).toLocaleDateString()}` : ""}`
      : null;
  if (!status) return connected ? `${connected} · checking…` : "checking…";
  switch (status.state) {
    case "ready":
      return needsSession && connected ? connected : "ready";
    case "needs_session":
      return connected ? "session expired, connect again" : "needs a browser session";
    case "unreachable":
      return `unreachable (${status.reason})`;
  }
}
