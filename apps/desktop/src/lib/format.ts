import type { Manga } from "./ipc";

export const statusLabel: Record<Manga["status"], string> = {
  ongoing: "Ongoing",
  completed: "Completed",
  hiatus: "On hiatus",
  cancelled: "Cancelled",
  unknown: "Unknown",
};

export const languageLabel = (code: string) => {
  try {
    return new Intl.DisplayNames(["en"], { type: "language" }).of(code) ?? code;
  } catch {
    return code;
  }
};

export const mangaKey = (m: { source: string; id: string }) => `${m.source}:${m.id}`;

export const errorMessage = (e: unknown) => (e instanceof Error ? e.message : String(e));
