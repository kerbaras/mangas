import { useState } from "react";
import { cn } from "@/lib/cn";
import type { ProviderInfo } from "@/lib/ipc";

type Props = { provider: Pick<ProviderInfo, "name" | "icon_url">; className?: string };

/** The site's favicon, or its initial when there is none or it fails to load. */
export function ProviderIcon({ provider, className }: Props) {
  const [failed, setFailed] = useState(false);
  const box = cn("size-4 shrink-0 rounded-sm", className);
  if (provider.icon_url && !failed)
    return <img src={provider.icon_url} alt="" className={box} onError={() => setFailed(true)} />;
  return (
    <span
      aria-hidden
      className={cn(box, "inline-flex items-center justify-center bg-surface-2 text-ink-2")}
    >
      <span className="text-[10px] leading-none font-semibold">{provider.name.charAt(0)}</span>
    </span>
  );
}
