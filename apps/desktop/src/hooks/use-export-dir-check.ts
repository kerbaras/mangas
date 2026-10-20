import { useEffect, useState } from "react";
import { errorMessage } from "@/lib/format";
import { checkExportDir } from "@/lib/ipc.volumes";

/** How often a refused folder is checked again, so fixing its permissions re-enables Export. */
export const RECHECK_MS = 1500;

/**
 * Why a split or replacing export into `dir` would be refused (`couldn't list <dir>: …`), null
 * when it would not, undefined until the check for `key` settles. A refusal is checked again
 * every [`RECHECK_MS`] until it clears; a plain export, or one with nothing to write, is never.
 */
export function useExportDirCheck(
  dir: string,
  split: boolean,
  replace: boolean,
  any: boolean,
  key: string,
): string | null | undefined {
  const needed = (split || replace) && dir.trim() !== "" && any;
  const [checked, setChecked] = useState<{ key: string; message: string | null } | null>(null);
  // biome-ignore lint/correctness/useExhaustiveDependencies: key holds dir, split and replace
  useEffect(() => {
    if (!needed) return;
    let alive = true;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const check = () =>
      checkExportDir(dir, split, replace)
        .then(() => null, errorMessage)
        .then((message) => {
          if (!alive) return;
          setChecked({ key, message });
          if (message) timer = setTimeout(check, RECHECK_MS);
        });
    void check();
    return () => {
      alive = false;
      clearTimeout(timer);
    };
  }, [key, needed]);
  if (!needed) return null;
  return checked?.key === key ? checked.message : undefined;
}
