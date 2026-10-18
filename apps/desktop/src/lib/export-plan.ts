/** Choices of the export dialog turned into planner input. Pure. */
import type { Async } from "@/hooks/use-async";
import type { Chapter, DetectorsStatus, ExportFormat } from "./ipc";
import type { Grouping } from "./ipc.volumes";

/** How the chapters become files: one per chapter, or bundled like the volume planner does. */
export type BundleMode = "per_chapter" | Grouping["kind"];

/** One file per chapter is the planner's "every 1": each file still gets cover, TOC and metadata. */
export const groupingOf = (mode: BundleMode, size: number): Grouping => {
  if (mode === "per_chapter") return { kind: "every", size: 1 };
  if (mode === "every") return { kind: "every", size };
  return { kind: mode };
};

const volumes = (chapters: Chapter[]) => new Set(chapters.map((c) => c.volume?.trim() || null));

/** Opens on "By volume" when the chapters span several provider volumes, else one book. */
export const initialMode = (chapters: Chapter[]): BundleMode =>
  volumes(chapters).size > 1 ? "by_volume" : "single";

/** `name` with the extension of `format`, as the exporter writes it. */
export const withExtension = (name: string, format: ExportFormat): string =>
  `${name.replace(/\.(epub|cbz|pdf)$/i, "")}.${format}`;

/** The switch only works when the build was compiled with detector support (#130). */
export const detectorsUsable = (state: Async<DetectorsStatus>): boolean =>
  state.kind === "ready" && state.data.available;

/** On by default once the models are on disk; first use would otherwise start a download. */
export const detectorsDefault = (state: Async<DetectorsStatus>): boolean =>
  detectorsUsable(state) && state.kind === "ready" && state.data.downloaded;

/** "Export 3 books and copy": the dialog's main button. */
export function exportAction(books: number, delivery: "save" | "usb" | "email"): string {
  const what = books > 1 ? `${books} books` : "book";
  const then = { save: "", usb: " and copy", email: " and email" }[delivery];
  return `Export ${what}${then}`;
}

/** "3 downloaded chapters of Solo" or "5 downloaded chapters from 2 series". */
export function pickSummary(picks: { manga: { title: string }; chapters: unknown[] }[]): string {
  const n = picks.reduce((sum, p) => sum + p.chapters.length, 0);
  const chapters = `${n} downloaded ${n === 1 ? "chapter" : "chapters"}`;
  if (picks.length === 1) return `${chapters} of ${picks[0].manga.title}`;
  return `${chapters} from ${picks.length} series`;
}
