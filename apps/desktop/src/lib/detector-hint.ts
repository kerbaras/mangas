import type { Async } from "@/hooks/use-async";
import type { DetectorsProgress, DetectorsStatus } from "./ipc.detectors";
import { formatBytes } from "./storage";

/** The line under the detectors switch, from what the backend says about detector support. */
export function detectorHint(state: Async<DetectorsStatus>): string {
  if (state.kind === "error") return `Couldn't check detector support: ${state.message}`;
  if (state.kind !== "ready") return "Checking detector support…";
  if (!state.data.available)
    return "This build has no detector support; cuts follow gutters and panel edges only.";
  const what = "Speech-bubble and face detectors keep cuts away from dialogue and faces.";
  if (runtimeFailure(state.data)) return `${what} Off: the ONNX Runtime didn't load.`;
  return state.data.downloaded
    ? `${what} Running on ${state.data.provider}.`
    : `${what} Download them to turn this on.`;
}

/** Why loading the runtime failed, if the backend says so. */
export const runtimeFailure = (s: DetectorsStatus): string | null =>
  s.runtime.state === "failed" ? s.runtime.reason : null;

/** After a partial remove; the CLI prints the same words (`pending_line`). */
export const pendingRemoval = (s: DetectorsStatus): string | null =>
  s.runtime.state === "remove_pending"
    ? `Detector models removed; the runtime (${formatBytes(s.runtime.bytes)}) is still in use — restart Mangas and remove again.`
    : null;

export const downloadLabel = (bytes: number) =>
  `Download detector support (~${formatBytes(bytes)})`;

/** "ONNX Runtime: 4.20 MB of 9.13 MB". */
export function progressLabel(p: DetectorsProgress | null): string {
  if (!p) return "Starting download…";
  const what = p.item === "onnxruntime" ? "ONNX Runtime" : `Model ${p.item}`;
  const total = p.total ? ` of ${formatBytes(p.total)}` : "";
  return `${what}: ${formatBytes(p.done)}${total}`;
}

/** "ONNX Runtime 1.28.3 and 2 models, 190 MB on disk". */
export function installedLabel(s: DetectorsStatus): string {
  const version = s.runtime.state === "downloaded" ? ` ${s.runtime.version}` : "";
  return `ONNX Runtime${version} and 2 models, ${formatBytes(s.diskBytes)} on disk`;
}
