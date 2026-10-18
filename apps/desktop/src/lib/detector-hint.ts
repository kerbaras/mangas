import type { Async } from "@/hooks/use-async";
import type { DetectorsStatus } from "./ipc";

/** The line under the detectors switch, from what the backend says about detector support. */
export function detectorHint(state: Async<DetectorsStatus>): string {
  if (state.kind === "error") return `Couldn't check detector support: ${state.message}`;
  if (state.kind !== "ready") return "Checking detector support…";
  if (!state.data.available)
    return "This build has no detector support; cuts follow gutters and panel edges only.";
  return `Speech-bubble and face detectors (${state.data.provider}) keep cuts away from dialogue and faces. ${
    state.data.downloaded ? "Models are ready." : "Models download on first use."
  }`;
}
