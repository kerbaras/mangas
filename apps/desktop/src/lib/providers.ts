import { connectProvider, getSettings } from "./ipc";

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
