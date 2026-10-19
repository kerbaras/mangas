import type { Chapter } from "./ipc";

/** `path` without its last component; `/` and `\` both separate (Windows paths). */
export function parentFolder(path: string): string {
  const trimmed = path.replace(/[\\/]+$/, "");
  const cut = Math.max(trimmed.lastIndexOf("/"), trimmed.lastIndexOf("\\"));
  return cut > 0 ? trimmed.slice(0, cut) : trimmed;
}

/**
 * The folder of the first chapter on disk among `chapters`, or with `title` the folder that holds
 * it (the title's folder: chapters sit at `<data>/<source>/<title>/<chapter>`). Null when none
 * is on disk.
 */
export function chapterFolder(chapters: Chapter[], title: boolean): string | null {
  const path = chapters.find((c) => c.local_path)?.local_path;
  if (!path) return null;
  return title ? parentFolder(path) : path;
}
