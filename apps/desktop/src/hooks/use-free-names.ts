import { type Async, useAsync } from "@/hooks/use-async";
import { errorMessage } from "@/lib/format";
import { checkExportDir, freeExportNames, type VolumePlan } from "@/lib/ipc.volumes";

/**
 * The planned books under the names the export will really write in `dir`: a name already taken
 * there gets ` (2)`, ` (3)`… as the writer does, unless `replace` writes over it. `named` is false
 * while the folder is being read; `epoch` rereads it (after an export filled it). `blocked` is the
 * `couldn't list` line when a split or replacing export into `dir` would be refused; a plain
 * export into an unlistable folder keeps its planned names (the writer's claim guards them).
 */
export function useFreeNames(
  plans: Async<VolumePlan[]>,
  dir: string,
  epoch: number,
  replace: boolean,
  splitMb: number | null,
) {
  const split = splitMb !== null;
  const wanted = plans.kind === "ready" ? plans.data.map((p) => p.file_name) : [];
  const key = [dir, epoch, replace, split, ...wanted].join("\n");
  const free = useAsync(async () => {
    const blocked =
      (split || replace) && dir.trim() && wanted.length > 0
        ? await checkExportDir(dir, split, replace).then(() => null, errorMessage)
        : null;
    const names =
      !replace && dir.trim() && wanted.length > 0
        ? await freeExportNames(dir, wanted).catch(() => wanted)
        : replace
          ? ownNames(wanted)
          : wanted;
    return { key, names, blocked };
  }, [key]);
  const done = free.state.kind === "ready" && free.state.data.key === key ? free.state.data : null;
  const [names, blocked] = [done?.names ?? null, done?.blocked ?? null];
  if (plans.kind !== "ready" || !names) return { files: plans, named: names !== null, blocked };
  const data = plans.data.map((p, i) => ({ ...p, file_name: names[i] ?? p.file_name }));
  return { files: { kind: "ready", data } as Async<VolumePlan[]>, named: true, blocked };
}

/** The names `volume::one_file_each` writes with replace on: a repeat gets ` (2)`, ` (3)`… */
export function ownNames(names: string[]): string[] {
  const used = new Set<string>();
  return names.map((name) => {
    for (let n = 1; ; n++) {
      const candidate = numbered(name, n);
      if (!used.has(candidate.toLowerCase())) {
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
