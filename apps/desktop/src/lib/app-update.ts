/**
 * App update state shared by the banner and Settings → About: the last check's result and the
 * install in progress. A silent check (at launch) never reports a failure.
 */
import { useSyncExternalStore } from "react";
import { errorMessage } from "./format";
import { backend } from "./ipc";
import type { AppUpdateChannel, AppUpdateStatus, InstallProgress } from "./ipc.app-update";

export type CheckState =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "done"; status: AppUpdateStatus }
  | { kind: "failed"; error: string };

export type InstallState =
  | { kind: "idle" }
  | { kind: "running"; progress: InstallProgress | null }
  | { kind: "failed"; error: string };

export type AppUpdateState = { check: CheckState; install: InstallState; dismissed: boolean };

const INITIAL: AppUpdateState = {
  check: { kind: "idle" },
  install: { kind: "idle" },
  dismissed: false,
};
let state = INITIAL;
/** Bumped by every check and reset: only the latest check's answer lands. */
let generation = 0;
const listeners = new Set<() => void>();

const set = (patch: Partial<AppUpdateState>) => {
  state = { ...state, ...patch };
  for (const l of listeners) l();
};

export const readAppUpdate = () => state;

/** The update a finished check found, if any. */
export const availableUpdate = (s: AppUpdateState) =>
  s.check.kind === "done" && s.check.status.kind === "available" ? s.check.status : null;

export async function checkForAppUpdate(channel: AppUpdateChannel, opts = { silent: false }) {
  if (state.install.kind === "running") return;
  const gen = ++generation;
  if (!opts.silent) set({ check: { kind: "checking" } });
  try {
    const status = await (await backend()).checkAppUpdate(channel);
    if (gen === generation) set({ check: { kind: "done", status }, install: INITIAL.install });
  } catch (e: unknown) {
    if (gen !== generation) return;
    if (opts.silent) console.warn("app update check failed", e);
    else set({ check: { kind: "failed", error: errorMessage(e) } });
  }
}

/** Downloads and installs the update found; on success the app relaunches. */
export async function installAppUpdate() {
  const update = availableUpdate(state);
  if (state.install.kind === "running" || !update) return;
  set({ install: { kind: "running", progress: null }, dismissed: false });
  try {
    await (await backend()).installAppUpdate(update.version, (progress) =>
      set({ install: { kind: "running", progress } }),
    );
  } catch (e: unknown) {
    set({ install: { kind: "failed", error: errorMessage(e) } });
  }
}

export const dismissAppUpdate = () => set({ dismissed: true });

/** Forgets the last check (the channel changed); an install already running carries on. */
export function resetAppUpdate({ force = false } = {}) {
  if (state.install.kind === "running" && !force) return;
  generation++;
  set(INITIAL);
}

const subscribe = (l: () => void) => {
  listeners.add(l);
  return () => listeners.delete(l);
};

export const useAppUpdate = () => useSyncExternalStore(subscribe, readAppUpdate, readAppUpdate);
