import { RotateCw } from "lucide-react";
import { useEffect, useState } from "react";
import { ProviderRow } from "@/components/provider-row";
import { Button } from "@/components/ui/button";
import { useAsync } from "@/hooks/use-async";
import { useNow } from "@/hooks/use-now";
import { describeFailure } from "@/lib/errors";
import { errorMessage } from "@/lib/format";
import { disconnectProvider, getSettings, listProviders, type ProviderInfo } from "@/lib/ipc";
import { isEnabled, withEnabled } from "@/lib/provider-prefs";
import { checkStatus, useProviderChecks } from "@/lib/provider-status";
import { connectAndWait, connectNote } from "@/lib/providers";
import { useSettings } from "@/lib/settings";

/** A status older than this is checked again when Settings opens. */
const STALE_MS = 60_000;

/** Every provider with its status, session buttons, languages and an on/off switch for search. */
export function ProvidersList() {
  const providers = useAsync(listProviders, []);
  const sessions = useAsync(async () => (await getSettings()).provider_sessions, []);
  const checks = useProviderChecks();
  const [settings, update] = useSettings();
  const [connecting, setConnecting] = useState<string | null>(null);
  const [note, setNote] = useState<string | null>(null);
  const now = useNow(30_000);
  const list = providers.state.kind === "ready" ? providers.state.data : [];
  const ids = list.map((p) => p.id).join(",");

  // biome-ignore lint/correctness/useExhaustiveDependencies: probe once per provider list, not on every check
  useEffect(() => {
    for (const id of ids.split(",").filter(Boolean)) {
      const last = checks[id];
      if (!last || Date.now() - last.checkedAt > STALE_MS) void checkStatus(id);
    }
  }, [ids]);

  const connect = async (p: ProviderInfo) => {
    setConnecting(p.id);
    setNote(`Pass the verification in the ${p.name} window; it closes by itself when done.`);
    try {
      const outcome = await connectAndWait(p.id);
      setNote(connectNote(outcome, p.name));
      // Probe before the new session shows, so the row reads "Checking", never "expired".
      void checkStatus(p.id);
      sessions.reload();
    } catch (e: unknown) {
      setNote(describeFailure(errorMessage(e)).label);
    } finally {
      setConnecting(null);
    }
  };
  const disconnect = async (p: ProviderInfo) => {
    await disconnectProvider(p.id);
    setNote(null);
    void checkStatus(p.id);
    sessions.reload();
  };

  if (providers.state.kind !== "ready") return <p className="text-ink-3">Loading providers…</p>;
  const connected = sessions.state.kind === "ready" ? sessions.state.data : {};
  return (
    <div className="flex flex-col gap-2">
      <ul className="divide-y divide-stroke rounded-md border border-stroke">
        {list.map((p) => (
          <ProviderRow
            key={p.id}
            provider={p}
            session={connected[p.id]}
            check={checks[p.id]}
            enabled={isEnabled(settings, p.id)}
            connecting={connecting === p.id}
            now={now}
            onEnable={(on) => update({ providersEnabled: withEnabled(settings, p.id, on) })}
            onConnect={() => void connect(p)}
            onDisconnect={() => void disconnect(p)}
          />
        ))}
      </ul>
      <div className="flex items-center gap-3">
        <p className="min-w-0 flex-1 text-caption text-ink-3 [overflow-wrap:anywhere]">{note}</p>
        <Button
          size="sm"
          variant="ghost"
          onClick={() => list.forEach((p) => void checkStatus(p.id))}
        >
          <RotateCw /> Check again
        </Button>
      </div>
    </div>
  );
}
