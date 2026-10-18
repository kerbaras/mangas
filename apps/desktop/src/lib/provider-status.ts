/**
 * Last known status of each provider and when it was checked, shared by Settings and Search.
 * A search answer counts as a check too. `undefined` means a check is running or due.
 */
import { useSyncExternalStore } from "react";
import { errorMessage } from "./format";
import { type ProviderStatus, providerStatus } from "./ipc";

export type Checked = { status: ProviderStatus; checkedAt: number };

let checks: Record<string, Checked> = {};
const listeners = new Set<() => void>();
const notify = () => {
  for (const l of listeners) l();
};

export function recordStatus(id: string, status: ProviderStatus, at = Date.now()) {
  checks = { ...checks, [id]: { status, checkedAt: at } };
  notify();
}

/** Forgets `id`'s status so it reads as checking (right after connecting, say). */
export function forgetStatus(id: string) {
  const { [id]: _, ...rest } = checks;
  checks = rest;
  notify();
}

/** Probes `id` and records the answer; a failed probe counts as unreachable. */
export async function checkStatus(id: string): Promise<ProviderStatus> {
  forgetStatus(id);
  const status = await providerStatus(id).catch(
    (e: unknown): ProviderStatus => ({ state: "unreachable", reason: errorMessage(e) }),
  );
  recordStatus(id, status);
  return status;
}

const subscribe = (l: () => void) => {
  listeners.add(l);
  return () => listeners.delete(l);
};
const read = () => checks;

export const useProviderChecks = () => useSyncExternalStore(subscribe, read, read);
