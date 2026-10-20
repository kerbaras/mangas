/** A volume export that failed part-way (`mangas_core::export::volume::VolumeOutcome`). */
import type { ExportResult } from "./ipc";
import type { VolumeOutcome } from "./ipc.wire";
import { toExportResult } from "./ipc.wire";

export type StoppedBook = { title: string; output: string };

/**
 * The run stopped at `failed`: the books in `results` were written before it and are complete,
 * nothing of `failed` is on disk and the books in `notWritten` were not started.
 */
export class ExportStopped extends Error {
  readonly results: ExportResult[];
  readonly failed: StoppedBook & { reason: string };
  readonly notWritten: StoppedBook[];

  constructor(outcome: VolumeOutcome & { failed: NonNullable<VolumeOutcome["failed"]> }) {
    super(`${outcome.failed.title} failed: ${outcome.failed.reason}`);
    this.name = "ExportStopped";
    this.results = outcome.written.map(toExportResult);
    this.failed = outcome.failed;
    this.notWritten = outcome.not_written;
  }
}

/** The written books of an outcome, or an `ExportStopped` error when one of them failed. */
export function outcomeResults(outcome: VolumeOutcome): ExportResult[] {
  const { failed } = outcome;
  if (failed) throw new ExportStopped({ ...outcome, failed });
  return outcome.written.map(toExportResult);
}
