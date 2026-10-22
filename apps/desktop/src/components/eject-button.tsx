import { ArrowUpFromLine, Check } from "lucide-react";
import { useEffect } from "react";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { useEject } from "@/hooks/use-eject";
import type { KindleVolume } from "@/lib/ipc.send";

type Props = {
  kindle: KindleVolume;
  /** Where the books just copied are on it: flushed before the eject. */
  files?: string[];
  /** Why Eject can't run now (a copy is under way); shown on hover and focus. */
  blocked?: string;
  onEjected?: () => void;
};

/** Eject for a mounted Kindle; the result goes to one toast that replaces the last. */
export function EjectButton({ kindle, files = [], blocked, onEjected }: Props) {
  const { state, eject, reset } = useEject(onEjected);
  const copying = blocked !== undefined;
  // A new copy remounted or rewrote the drive: it needs ejecting again.
  useEffect(() => {
    if (copying) reset();
  }, [copying, reset]);
  const ejecting = state.kind === "ejecting";
  const ejected = state.kind === "ejected";
  const button = (
    <Button
      size="sm"
      disabled={copying || ejecting || ejected}
      onClick={() => void eject(kindle, files)}
    >
      {ejected ? <Check /> : <ArrowUpFromLine />}
      {ejecting ? "Ejecting…" : ejected ? "Ejected" : "Eject"}
    </Button>
  );
  if (!copying) return button;
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        {/* biome-ignore lint/a11y/noNoninteractiveTabindex: focus shows why Eject is disabled */}
        <span tabIndex={0} className="relative rounded-md">
          {button}
          <span className="sr-only">{blocked}</span>
        </span>
      </TooltipTrigger>
      <TooltipContent className="max-w-64">{blocked}</TooltipContent>
    </Tooltip>
  );
}
