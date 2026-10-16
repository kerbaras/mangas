import { connectProvider, getSettings } from "./ipc";

const POLL_MS = 2000;
const TIMEOUT_MS = 180_000;

const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

/** Resolves once a session is stored for `id`, or `false` after the timeout. */
export async function waitForSession(id: string, timeoutMs = TIMEOUT_MS): Promise<boolean> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    await sleep(POLL_MS);
    if ((await getSettings()).provider_sessions[id]) return true;
  }
  return false;
}

/** Opens the Connect window and waits for the verification to produce a session. */
export async function connectAndWait(id: string): Promise<boolean> {
  await connectProvider(id);
  return waitForSession(id);
}
