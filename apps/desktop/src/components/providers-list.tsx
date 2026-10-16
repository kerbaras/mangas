import { Link2, Link2Off } from "lucide-react";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import { useAsync } from "@/hooks/use-async";
import { errorMessage } from "@/lib/format";
import { disconnectProvider, getSettings, listProviders, type ProviderInfo } from "@/lib/ipc";
import { connectAndWait } from "@/lib/providers";

/** Providers with their connection state; sites behind a verification get Connect/Disconnect. */
export function ProvidersList() {
  const providers = useAsync(listProviders, []);
  const sessions = useAsync(async () => (await getSettings()).provider_sessions, []);
  const [note, setNote] = useState<string | null>(null);

  const connect = async (p: ProviderInfo) => {
    setNote(`Pass the verification in the ${p.name} window; it closes by itself when done.`);
    try {
      const ok = await connectAndWait(p.id);
      setNote(ok ? null : `No session captured for ${p.name}; close its window and try again.`);
      sessions.reload();
    } catch (e: unknown) {
      setNote(errorMessage(e));
    }
  };
  const disconnect = async (p: ProviderInfo) => {
    await disconnectProvider(p.id);
    setNote(null);
    sessions.reload();
  };

  if (providers.state.kind !== "ready") return <p className="text-ink-3">Loading providers…</p>;
  const connected = sessions.state.kind === "ready" ? sessions.state.data : {};
  return (
    <div className="flex flex-col gap-2">
      <ul className="divide-y divide-stroke rounded-md border border-stroke">
        {providers.state.data.map((p) => {
          const session = connected[p.id];
          return (
            <li key={p.id} className="flex items-center gap-3 px-3 py-2">
              <div className="min-w-0 flex-1">
                <p className="text-body font-medium">{p.name}</p>
                <p className="truncate text-caption text-ink-3">
                  {p.site.replace(/^https?:\/\//, "")}
                  {p.needs_browser_session &&
                    (session
                      ? ` · connected${when(session.saved_at)}`
                      : " · needs a browser session")}
                </p>
              </div>
              {p.needs_browser_session &&
                (session ? (
                  <Button variant="ghost" size="sm" onClick={() => disconnect(p)}>
                    <Link2Off /> Disconnect
                  </Button>
                ) : (
                  <Button size="sm" onClick={() => connect(p)}>
                    <Link2 /> Connect
                  </Button>
                ))}
            </li>
          );
        })}
      </ul>
      {note && <p className="text-caption text-ink-3">{note}</p>}
    </div>
  );
}

const when = (iso: string | null) => (iso ? ` ${new Date(iso).toLocaleDateString()}` : "");
