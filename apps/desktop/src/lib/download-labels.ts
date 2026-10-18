/**
 * What each download was for: progress events only carry the chapter id, but the toast needs a
 * label and the provider (for Connect), and the log needs the provider and manga ids.
 */
export type DownloadContext = { label: string; source: string; mangaId: string };

const contexts = new Map<string, DownloadContext>();

export function rememberDownloads(entries: [id: string, context: DownloadContext][]) {
  for (const [id, context] of entries) contexts.set(id, context);
}

export const downloadContext = (id: string): DownloadContext | undefined => contexts.get(id);

export const downloadLabel = (id: string) => contexts.get(id)?.label ?? "A chapter";

/** The warning logged for one failed chapter; ids as key=value so the log can be grepped. */
export const failureLog = (id: string, error: string, context?: DownloadContext) =>
  `chapter download failed: provider=${context?.source ?? "unknown"} manga=${context?.mangaId ?? "unknown"} chapter=${id}: ${error}`;
