import { useCallback, useState } from "react";
import { toast } from "sonner";
import { errorMessage } from "@/lib/format";
import { ejectKindle, type KindleVolume } from "@/lib/ipc.send";
import { notifyError } from "@/lib/notify";

export type EjectState =
  | { kind: "idle" }
  | { kind: "ejecting" }
  | { kind: "ejected" }
  | { kind: "error"; message: string };

/** The one toast of a Kindle's eject: each step replaces the last (#286). */
export const ejectToastId = (kindle: KindleVolume) => `eject-${kindle.root}`;

/** Flushes and ejects a mounted Kindle; a failure is retryable (close the program, try again). */
export function useEject(onEjected?: () => void) {
  const [state, setState] = useState<EjectState>({ kind: "idle" });
  const eject = useCallback(
    async function run(kindle: KindleVolume, files: string[]) {
      const id = ejectToastId(kindle);
      setState({ kind: "ejecting" });
      toast.loading(`Ejecting ${kindle.name}…`, { id, description: null, action: null });
      try {
        await ejectKindle(kindle.root, files);
        setState({ kind: "ejected" });
        toast.success(`${kindle.name} ejected`, {
          id,
          description: "It's safe to unplug it now.",
          action: null,
          duration: 6_000,
        });
        onEjected?.();
      } catch (e) {
        setState({ kind: "error", message: errorMessage(e) });
        notifyError(`Couldn't eject ${kindle.name}`, e, {
          id,
          retry: () => void run(kindle, files),
          retryReplaces: true,
          full: true,
        });
      }
    },
    [onEjected],
  );
  const reset = useCallback(() => setState({ kind: "idle" }), []);
  return { state, eject, reset };
}
