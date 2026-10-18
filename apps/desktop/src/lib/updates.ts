/**
 * Update checks shared by the Library button and the background schedule: one run at a time,
 * observable so the button spins whatever started the run, and a counter screens reload on.
 */
import { useSyncExternalStore } from "react";
import { checkUpdates, type UpdateRun } from "./ipc.updates";

export type UpdatesState = {
  checking: boolean;
  /** Finished runs that found something; the library reloads when it moves. */
  found: number;
  last: UpdateRun | null;
};

let state: UpdatesState = { checking: false, found: 0, last: null };
let running: Promise<UpdateRun> | null = null;
const listeners = new Set<() => void>();

const set = (patch: Partial<UpdatesState>) => {
  state = { ...state, ...patch };
  for (const l of listeners) l();
};

/**
 * Runs a check, or joins the one in flight. `background` runs (launch, timer) let the backend
 * send the desktop notification; a manual run shows its own toast.
 */
export function runCheck(background: boolean): Promise<UpdateRun> {
  if (running) return running;
  set({ checking: true });
  running = checkUpdates(background)
    .then((run) => {
      set({ last: run, found: state.found + (run.updates.length > 0 ? 1 : 0) });
      return run;
    })
    .finally(() => {
      running = null;
      set({ checking: false });
    });
  return running;
}

export const readUpdates = (): UpdatesState => state;

const subscribe = (l: () => void) => {
  listeners.add(l);
  return () => listeners.delete(l);
};

export const useUpdates = (): UpdatesState =>
  useSyncExternalStore(subscribe, readUpdates, readUpdates);

/** Milliseconds between background checks; null when they are off. */
export const intervalMs = (hours: number): number | null =>
  Number.isFinite(hours) && hours > 0 ? hours * 3_600_000 : null;
