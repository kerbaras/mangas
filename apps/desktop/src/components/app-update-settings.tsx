import { CheckCircle2, Info, Loader2, RefreshCw } from "lucide-react";
import { Field } from "@/components/field";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { useAsync } from "@/hooks/use-async";
import {
  type CheckState,
  checkForAppUpdate,
  installAppUpdate,
  resetAppUpdate,
  useAppUpdate,
} from "@/lib/app-update";
import { checkFailedLine, statusLine } from "@/lib/app-update-labels";
import { appVersion } from "@/lib/app-version";
import { describeFailure } from "@/lib/errors";
import type { AppUpdateChannel } from "@/lib/ipc.app-update";
import { useSettings } from "@/lib/settings";

function CheckResult({ check }: { check: CheckState }) {
  const { install } = useAppUpdate();
  if (check.kind === "checking") return <p className="text-caption text-ink-2">Checking…</p>;
  if (check.kind === "failed")
    return (
      <p
        role="alert"
        className="text-caption text-bad"
        title={describeFailure(check.error).tooltip}
      >
        {checkFailedLine(check.error)}
      </p>
    );
  if (check.kind !== "done") return null;
  const { status } = check;
  if (status.kind === "invalid_signature")
    return (
      <p role="alert" className="text-caption text-bad">
        {statusLine(status)}
      </p>
    );
  if (status.kind !== "available")
    return (
      <p className="flex items-center gap-1.5 text-caption text-ink-2">
        {status.kind === "older" ? (
          <Info className="size-4" />
        ) : (
          <CheckCircle2 className="size-4 text-ok" />
        )}{" "}
        {statusLine(status)}
      </p>
    );
  return (
    <div className="flex items-center gap-3">
      <p className="text-caption text-ink-2">{statusLine(status)}</p>
      <Button
        size="sm"
        variant="primary"
        disabled={install.kind === "running"}
        onClick={() => void installAppUpdate()}
      >
        Restart to update
      </Button>
    </div>
  );
}

/** Settings → About: app version, update check, release channel and launch checks. */
export function AppUpdateSettings() {
  const [settings, update] = useSettings();
  const loaded = useAsync(appVersion, []);
  const version = loaded.state.kind === "ready" ? loaded.state.data : null;
  const { check } = useAppUpdate();
  const checking = check.kind === "checking";
  const setChannel = (channel: AppUpdateChannel) => {
    update({ appUpdateChannel: channel });
    resetAppUpdate();
  };
  return (
    <>
      <Field label="Version">
        <div className="flex items-center gap-3">
          <span className="text-body tabular-nums">{version ?? "…"}</span>
          <Button
            size="sm"
            disabled={checking}
            onClick={() => void checkForAppUpdate(settings.appUpdateChannel)}
          >
            {checking ? <Loader2 className="animate-spin" /> : <RefreshCw />} Check for updates
          </Button>
        </div>
        <CheckResult check={check} />
      </Field>
      <Field
        label="Update channel"
        htmlFor="app-update-channel"
        hint="Release candidates come out before each release, to try what's next. They may have rough edges."
      >
        <Select
          value={settings.appUpdateChannel}
          onValueChange={(v) => setChannel(v === "rc" ? "rc" : "stable")}
        >
          <SelectTrigger id="app-update-channel" className="w-48">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="stable">Stable</SelectItem>
            <SelectItem value="rc">Release candidates</SelectItem>
          </SelectContent>
        </Select>
      </Field>
      <Field
        label="Automatic checks"
        hint="Only looks for a new version; nothing downloads until you choose Restart to update."
      >
        <label htmlFor="app-update-checks" className="flex items-center gap-2 text-body">
          <Switch
            id="app-update-checks"
            checked={settings.appUpdateChecks}
            onCheckedChange={(on) => update({ appUpdateChecks: on })}
          />
          Check for updates at launch
        </label>
      </Field>
    </>
  );
}
