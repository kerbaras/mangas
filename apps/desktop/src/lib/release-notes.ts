/**
 * Release notes as the update manifest carries them: the release's CHANGELOG.md section
 * (`scripts/release-notes.sh`), in Keep a Changelog markdown. Only that subset is read:
 * `### Group` headings, `- item` lists, `**bold**`, and plain lines. Keys are line positions.
 */
export type NotesSpan = { key: string; text: string; bold: boolean };
export type NotesItem = { key: string; spans: NotesSpan[] };
export type NotesBlock =
  | { kind: "heading"; key: string; text: string }
  | { kind: "list"; key: string; items: NotesItem[] }
  | { kind: "text"; key: string; spans: NotesSpan[] };

export const spans = (line: string, key = ""): NotesSpan[] =>
  line
    .split(/\*\*(.+?)\*\*/)
    .map((text, i) => ({ key: `${key}.${i}`, text, bold: i % 2 === 1 }))
    .filter((s) => s.text !== "");

/** Blocks to show; the `## [x.y.z] - date` heading is dropped (the title names the version). */
export function parseReleaseNotes(notes: string | null): NotesBlock[] {
  const blocks: NotesBlock[] = [];
  (notes ?? "").split(/\r?\n/).forEach((raw, n) => {
    const line = raw.trim();
    const key = String(n);
    const last = blocks.at(-1);
    if (line === "" || /^##\s+\[/.test(line)) return;
    if (line.startsWith("#"))
      blocks.push({ kind: "heading", key, text: line.replace(/^#+\s*/, "") });
    else if (/^[-*]\s+/.test(line)) {
      const item = { key, spans: spans(line.replace(/^[-*]\s+/, ""), key) };
      if (last?.kind === "list") last.items.push(item);
      else blocks.push({ kind: "list", key, items: [item] });
    } else blocks.push({ kind: "text", key, spans: spans(line, key) });
  });
  return blocks;
}
