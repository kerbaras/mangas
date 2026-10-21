import { PlugZap } from "lucide-react";
import { useEffect, useId, useState } from "react";
import { toast } from "sonner";
import { Field } from "@/components/field";
import { SettingsGroup } from "@/components/settings-group";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { type ProxyMode, testProxy } from "@/lib/ipc";
import { notifyError } from "@/lib/notify";
import { proxyUrlError } from "@/lib/proxy-url";
import { useSettings } from "@/lib/settings";

/** The proxy test's one toast: a later result replaces an earlier failure. */
const PROXY_TOAST = "proxy-test";

const MODES: { id: ProxyMode; label: string }[] = [
  { id: "system", label: "Use the system proxy" },
  { id: "none", label: "No proxy" },
  { id: "custom", label: "Custom HTTP(S) proxy" },
];

/** Settings → Network: the proxy every provider request goes through, with a test. */
export function NetworkSettings() {
  const [settings, update] = useSettings();
  const name = useId();
  const [draft, setDraft] = useState(settings.proxyUrl);
  // A reset or another process changing the stored address replaces the draft.
  useEffect(() => setDraft(settings.proxyUrl), [settings.proxyUrl]);
  const [testing, setTesting] = useState(false);
  const custom = settings.proxyMode === "custom";
  const error = custom ? proxyUrlError(draft) : null;
  const save = () => {
    if (!error && draft.trim() !== settings.proxyUrl) update({ proxyUrl: draft.trim() });
  };
  const test = async () => {
    setTesting(true);
    try {
      const status = await testProxy(settings.proxyMode, draft.trim());
      if (status.state === "unreachable")
        notifyError("Proxy test failed", status.reason, { id: PROXY_TOAST });
      else
        toast.success("Proxy works", {
          id: PROXY_TOAST,
          action: null,
          description: "MangaDex answered through it.",
        });
    } catch (e: unknown) {
      notifyError("Proxy test failed", e, { id: PROXY_TOAST });
    } finally {
      setTesting(false);
    }
  };
  return (
    <SettingsGroup
      title="Network"
      resets={["proxyMode", "proxyUrl"]}
      resetHint="Provider requests use the system proxy again and the custom address is cleared."
    >
      <Field
        label="Proxy"
        hint="Searches, chapter lists, pages and covers go through it, in the app and the `mangas` command. `mangas --proxy` overrides it for one run. Through a proxy, the proxy looks up image hosts, so the app only refuses private addresses written in a link."
      >
        <fieldset className="flex flex-col gap-2">
          <legend className="sr-only">Proxy</legend>
          {MODES.map((m) => (
            <label key={m.id} className="flex items-center gap-2">
              <input
                type="radio"
                name={name}
                checked={settings.proxyMode === m.id}
                onChange={() => update({ proxyMode: m.id })}
                className="accent-accent"
              />
              {m.label}
            </label>
          ))}
        </fieldset>
      </Field>
      {custom && (
        <Field
          label="Proxy address"
          htmlFor="proxy-url"
          hint={error ?? "http:// or https://; user:password@ for a proxy that asks."}
        >
          <Input
            id="proxy-url"
            value={draft}
            placeholder="http://127.0.0.1:8888"
            spellCheck={false}
            aria-invalid={error !== null}
            onChange={(e) => setDraft(e.target.value)}
            onBlur={save}
            onKeyDown={(e) => e.key === "Enter" && save()}
            className="max-w-md aria-[invalid=true]:border-bad"
          />
        </Field>
      )}
      <Button onClick={test} disabled={testing || error !== null} className="self-start">
        <PlugZap /> {testing ? "Testing…" : "Test connection"}
      </Button>
    </SettingsGroup>
  );
}
