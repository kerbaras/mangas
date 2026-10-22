import { Usb } from "lucide-react";
import { EjectButton } from "@/components/eject-button";
import { Hint } from "@/components/hint";
import { type KindleVolume, MTP_NO_EJECT } from "@/lib/ipc.send";

/** Shown on Eject while a copy runs. */
export const COPYING_HINT = "Wait for the copy to finish before ejecting";

type Props = {
  /** The Kindle the last USB delivery found, mounted. */
  kindle: KindleVolume | null;
  /** A Kindle reached over MTP only (nothing mounted). */
  mtp?: string;
  /** A delivery is running: Eject waits for it. */
  busy: boolean;
  /** Where the books the last copy wrote are on the Kindle. */
  copied?: string[];
};

/** The Kindle a delivery reached, with its Eject (mounted drives only; MTP has nothing to eject). */
export function KindleRow({ kindle, mtp, busy, copied }: Props) {
  const name = kindle?.name ?? mtp;
  if (!name) return null;
  const ejectable = kindle?.ejectable === true;
  return (
    <div className="flex items-center justify-between gap-3 rounded-md bg-surface-2 px-2 py-1.5">
      <div className="flex min-w-0 items-center gap-2">
        <Usb className="size-4 shrink-0 text-ink-3" aria-hidden />
        <div className="min-w-0">
          <p className="truncate text-caption font-medium text-ink">{name}</p>
          {kindle && ejectable ? (
            <Hint hint={kindle.root}>
              <p className="truncate text-caption text-ink-3">{kindle.root}</p>
            </Hint>
          ) : (
            <p className="text-caption text-ink-3">{`${name} ${MTP_NO_EJECT}.`}</p>
          )}
        </div>
      </div>
      {kindle && ejectable && (
        <EjectButton
          key={kindle.root}
          kindle={kindle}
          files={copied}
          blocked={busy ? COPYING_HINT : undefined}
        />
      )}
    </div>
  );
}
