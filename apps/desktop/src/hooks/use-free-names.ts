import { type Async, useAsync } from "@/hooks/use-async";
import { useExportDirCheck } from "@/hooks/use-export-dir-check";
import { freeExportNames, type KeptBook, type VolumePlan } from "@/lib/ipc.volumes";

/**
 * The planned books under the names the export will really write in `dir`: a name already taken
 * there gets ` (2)`, ` (3)`… as the writer does, unless `replace` writes over it. `named` is false
 * while the folder is being read; `epoch` rereads it (after an export filled it). `blocked` is the
 * `couldn't list` line when a split or replacing export into `dir` would be refused (checked again
 * until it clears); a plain export into an unlistable folder is named by looking each name up
 * there (`free_names_in`), so `Book (2).epub` shows when `Book.epub` is already in it.
 */
export function useFreeNames(
  plans: Async<VolumePlan[]>,
  dir: string,
  epoch: number,
  replace: boolean,
  splitMb: number | null,
  kept: KeptBook[] = [],
) {
  const split = splitMb !== null;
  // A book a stopped run kept stays under its own file; Try again names only the others
  // (`volume::retry_plans`), so the preview names those alone, around the kept files.
  const keptAt = plans.kind === "ready" ? plans.data.map((p) => keptFile(p, kept)) : [];
  const wanted =
    plans.kind === "ready" ? plans.data.filter((_, i) => !keptAt[i]).map((p) => p.file_name) : [];
  const key = [dir, epoch, replace, splitMb, ...wanted, "", ...keptAt].join("\n");
  const check = useExportDirCheck(dir, split, replace, wanted.length > 0, key);
  // A folder that lists again is read again: its names may now be taken.
  const listKey = `${key}\n${check === null}`;
  const free = useAsync(async () => {
    const fresh =
      !replace && dir.trim() && wanted.length > 0
        ? await freeExportNames(dir, wanted).catch(() => wanted)
        : replace
          ? ownNames(
              wanted,
              kept.map((k) => k.file),
            )
          : wanted;
    let next = 0;
    return { listKey, names: keptAt.map((file) => file ?? fresh[next++] ?? "") };
  }, [listKey]);
  const done =
    free.state.kind === "ready" && free.state.data.listKey === listKey ? free.state.data : null;
  const [names, blocked] = [check === undefined ? null : (done?.names ?? null), check ?? null];
  if (plans.kind !== "ready" || !names) return { files: plans, named: names !== null, blocked };
  const data = plans.data.map((p, i) => ({ ...p, file_name: names[i] || p.file_name }));
  return { files: { kind: "ready", data } as Async<VolumePlan[]>, named: true, blocked };
}

/**
 * The names `volume::one_file_each` writes with replace on: a repeat gets ` (2)`, ` (3)`…, and,
 * as `volume::retry_plans`, never a `taken` (kept) file nor a name whose first part is one.
 */
export function ownNames(names: string[], taken: string[] = []): string[] {
  const kept = new Set(taken.map((t) => t.toLowerCase()));
  const isKept = (c: string) => kept.has(c.toLowerCase()) || kept.has(firstPart(c).toLowerCase());
  const used = new Set<string>();
  return names.map((name) => {
    for (let n = 1; ; n++) {
      const candidate = numbered(name, n);
      if (!isKept(candidate) && !used.has(candidate.toLowerCase())) {
        used.add(candidate.toLowerCase());
        return candidate;
      }
    }
  });
}

function numbered(name: string, n: number): string {
  if (n === 1) return name;
  const dot = name.lastIndexOf(".");
  return dot > 0 ? `${name.slice(0, dot)} (${n})${name.slice(dot)}` : `${name} (${n})`;
}

/** `kindle::part_path(name, 1)`: `Book.epub` → `Book (part 1).epub`. */
function firstPart(name: string): string {
  const dot = name.lastIndexOf(".");
  return dot > 0 ? `${name.slice(0, dot)} (part 1)${name.slice(dot)}` : `${name} (part 1)`;
}

/** The kept book's file when `plan` is one (the same chapter ids), as `volume::skip_written` matches. */
function keptFile(plan: VolumePlan | undefined, kept: KeptBook[]): string | undefined {
  if (!plan) return undefined;
  const ids = plan.chapters
    .map((c) => c.id)
    .sort()
    .join("\n");
  return kept.find((k) => k.chapters.length && [...k.chapters].sort().join("\n") === ids)?.file;
}
