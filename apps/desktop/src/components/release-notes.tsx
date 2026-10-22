import { type NotesSpan, parseReleaseNotes } from "@/lib/release-notes";

function Spans({ spans }: { spans: NotesSpan[] }) {
  return spans.map((s) =>
    s.bold ? (
      <strong key={s.key} className="font-semibold text-ink">
        {s.text}
      </strong>
    ) : (
      <span key={s.key}>{s.text}</span>
    ),
  );
}

/** "What's new in vX": the update's release notes, collapsed; nothing when it has none. */
export function ReleaseNotes({ version, notes }: { version: string; notes: string | null }) {
  const blocks = parseReleaseNotes(notes);
  if (blocks.length === 0) return null;
  return (
    <details className="w-full max-w-2xl text-caption text-ink-2">
      <summary className="cursor-default select-none text-body text-ink">
        What's new in v{version}
      </summary>
      <div className="mt-2 grid max-h-64 gap-2 overflow-y-auto pr-2 [overflow-wrap:anywhere]">
        {blocks.map((b) =>
          b.kind === "heading" ? (
            <h3 key={b.key} className="font-semibold text-ink">
              {b.text}
            </h3>
          ) : b.kind === "list" ? (
            <ul key={b.key} className="grid list-disc gap-1 pl-5">
              {b.items.map((item) => (
                <li key={item.key}>
                  <Spans spans={item.spans} />
                </li>
              ))}
            </ul>
          ) : (
            <p key={b.key}>
              <Spans spans={b.spans} />
            </p>
          ),
        )}
      </div>
    </details>
  );
}
