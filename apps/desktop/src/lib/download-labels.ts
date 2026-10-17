/** Chapter labels for download toasts: progress events only carry the chapter id. */
const labels = new Map<string, string>();

export function rememberLabels(entries: [id: string, label: string][]) {
  for (const [id, label] of entries) labels.set(id, label);
}

export const downloadLabel = (id: string) => labels.get(id) ?? "A chapter";
