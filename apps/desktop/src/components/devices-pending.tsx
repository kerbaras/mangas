import type { Async } from "@/hooks/use-async";
import type { Device } from "@/lib/ipc";

/** Placeholder for a device picker whose list is still loading or failed to load. */
export function DevicesPending({ state }: { state: Exclude<Async<Device[]>, { kind: "ready" }> }) {
  return state.kind === "error" ? (
    <p className="text-caption text-accent-text [overflow-wrap:anywhere]">
      Couldn't load devices: {state.message}
    </p>
  ) : (
    <p className="text-ink-3">Loading devices…</p>
  );
}
