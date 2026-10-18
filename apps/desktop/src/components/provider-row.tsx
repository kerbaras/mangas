import { Link2, Link2Off, Loader2 } from "lucide-react";
import { ProviderIcon } from "@/components/provider-icon";
import { ProviderLanguages } from "@/components/provider-languages";
import { ProviderStatusBadge } from "@/components/provider-status-badge";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { cn } from "@/lib/cn";
import type { ProviderInfo, ProviderSession } from "@/lib/ipc";
import type { Checked } from "@/lib/provider-status";
import { providerDetail } from "@/lib/providers";
import { ago } from "@/lib/relative-time";

type Props = {
  provider: ProviderInfo;
  session: ProviderSession | undefined;
  check: Checked | undefined;
  enabled: boolean;
  connecting: boolean;
  now: number;
  onEnable: (on: boolean) => void;
  onConnect: () => void;
  onDisconnect: () => void;
};

/** Name, status and last check; languages, session buttons and the search switch on the right. */
export function ProviderRow({
  provider: p,
  session,
  check,
  enabled,
  connecting,
  now,
  ...on
}: Props) {
  const status = connecting ? undefined : check?.status;
  const needsConnect = !session || status?.state === "needs_session";
  const detail = providerDetail({
    site: p.site,
    needsSession: p.needs_browser_session,
    savedAt: session ? session.saved_at : undefined,
    status,
    checkedAt: connecting ? undefined : check?.checkedAt,
    ago: (at) => ago(at, now),
  });
  return (
    <li className="flex items-center gap-3 px-3 py-2.5">
      <ProviderIcon provider={p} className={cn("size-5", !enabled && "opacity-50")} />
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <p className={cn("text-body font-medium", !enabled && "text-ink-3")}>{p.name}</p>
          <ProviderStatusBadge status={status} label={connecting ? "Connecting" : undefined} />
        </div>
        <p className="truncate text-caption text-ink-3" title={detail}>
          {detail}
        </p>
      </div>
      {p.languages.length > 1 && <ProviderLanguages provider={p} />}
      {p.needs_browser_session && session && (
        <Button size="sm" variant="ghost" onClick={on.onDisconnect} disabled={connecting}>
          <Link2Off /> Disconnect
        </Button>
      )}
      {p.needs_browser_session && needsConnect && (
        <Button size="sm" variant="primary" onClick={on.onConnect} disabled={connecting}>
          {connecting ? <Loader2 className="animate-spin" /> : <Link2 />} Connect
        </Button>
      )}
      <Switch checked={enabled} onCheckedChange={on.onEnable} aria-label={`Search ${p.name}`} />
    </li>
  );
}
