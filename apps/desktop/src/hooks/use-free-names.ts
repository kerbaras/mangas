import { type Async, useAsync } from "@/hooks/use-async";
import { freeExportNames, type VolumePlan } from "@/lib/ipc.volumes";

/**
 * The planned books under the names the export will really write in `dir`: a name already taken
 * there gets ` (2)`, ` (3)`… as the writer does, unless `replace` writes over it. `named` is false
 * while the folder is being read; `epoch` rereads it (after an export filled it).
 */
export function useFreeNames(
  plans: Async<VolumePlan[]>,
  dir: string,
  epoch: number,
  replace: boolean,
) {
  const wanted = plans.kind === "ready" ? plans.data.map((p) => p.file_name) : [];
  const key = [dir, epoch, replace, ...wanted].join("\n");
  const free = useAsync(async () => {
    const names =
      !replace && dir.trim() && wanted.length > 0
        ? await freeExportNames(dir, wanted).catch(() => wanted)
        : wanted;
    return { key, names };
  }, [key]);
  const names =
    free.state.kind === "ready" && free.state.data.key === key ? free.state.data.names : null;
  if (plans.kind !== "ready" || !names) return { files: plans, named: names !== null };
  const data = plans.data.map((p, i) => ({ ...p, file_name: names[i] ?? p.file_name }));
  return { files: { kind: "ready", data } as Async<VolumePlan[]>, named: true };
}
