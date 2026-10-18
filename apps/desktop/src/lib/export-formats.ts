import type { Device, ExportFormat } from "./ipc";

export type FormatInfo = { id: ExportFormat; label: string; description: string };

export const FORMATS: FormatInfo[] = [
  { id: "epub", label: "EPUB", description: "Fixed-layout book for Kindle and Apple Books." },
  { id: "cbz", label: "CBZ", description: "Comic archive for Kobo, Komga and tablet readers." },
  { id: "pdf", label: "PDF", description: "One image per page, sized to the screen." },
];

export const isFormat = (v: unknown): v is ExportFormat => FORMATS.some((f) => f.id === v);

export const formatLabel = (id: ExportFormat): string =>
  FORMATS.find((f) => f.id === id)?.label ?? id.toUpperCase();

/** The format picked last, else what the device reads best, else EPUB. */
export const effectiveFormat = (
  picked: ExportFormat | null,
  devices: Device[],
  deviceId: string,
): ExportFormat => picked ?? devices.find((d) => d.id === deviceId)?.format ?? "epub";
