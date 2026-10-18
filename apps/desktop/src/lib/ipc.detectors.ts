/** Detector support part of the IPC contract (`commands/detectors.rs`): ONNX Runtime + models. */
import type { Unlisten } from "./ipc";
import { backend } from "./ipc";

/** Mirrors `mangas_core::ml::RuntimeStatus`. */
export type RuntimeState =
  | { state: "unavailable" }
  | { state: "not_downloaded"; version: string }
  | { state: "downloaded"; version: string }
  | { state: "failed"; version: string; reason: string };

export type DetectorsStatus = {
  /** Built with detector support for this platform; the runtime and models download on demand. */
  available: boolean;
  /** The runtime and every model are on disk. */
  downloaded: boolean;
  provider: string;
  runtime: RuntimeState;
  /** Bytes a download would fetch now. */
  downloadBytes: number;
  /** Bytes the runtime and models take on disk. */
  diskBytes: number;
};

/** `item` is "onnxruntime" or a model id. */
export type DetectorsProgress = { item: string; done: number; total: number | null };

export type DetectorsBackend = {
  detectorsStatus(): Promise<DetectorsStatus>;
  /** Downloads the runtime and the models; progress via `onDetectorsProgress`. */
  downloadDetectors(): Promise<DetectorsStatus>;
  /** Stops a running download, which then rejects with "download cancelled". */
  cancelDetectorsDownload(): Promise<void>;
  /** Deletes the runtime and the models; resolves to the bytes freed. */
  removeDetectors(): Promise<number>;
  onDetectorsProgress(cb: (p: DetectorsProgress) => void): Promise<Unlisten>;
};

export const detectorsStatus = async () => (await backend()).detectorsStatus();
export const downloadDetectors = async () => (await backend()).downloadDetectors();
export const cancelDetectorsDownload = async () => (await backend()).cancelDetectorsDownload();
export const removeDetectors = async () => (await backend()).removeDetectors();
export const onDetectorsProgress = async (cb: (p: DetectorsProgress) => void) =>
  (await backend()).onDetectorsProgress(cb);
