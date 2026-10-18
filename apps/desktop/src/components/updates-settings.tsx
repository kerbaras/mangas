import { Field } from "@/components/field";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { useSettings } from "@/lib/settings";

const INTERVALS = [
  { hours: 0, label: "Never" },
  { hours: 1, label: "Every hour" },
  { hours: 3, label: "Every 3 hours" },
  { hours: 6, label: "Every 6 hours" },
  { hours: 12, label: "Every 12 hours" },
  { hours: 24, label: "Once a day" },
];

/** How often followed titles are checked for new chapters, and whether to notify. */
export function UpdatesSettings() {
  const [settings, update] = useSettings();
  const known = INTERVALS.some((i) => i.hours === settings.updateCheckHours);
  const options = known
    ? INTERVALS
    : [
        ...INTERVALS,
        { hours: settings.updateCheckHours, label: `Every ${settings.updateCheckHours} hours` },
      ];
  return (
    <>
      <Field
        label="Check for new chapters"
        htmlFor="update-interval"
        hint="At launch and then on this schedule, while the app is open. Completed and dropped titles are skipped."
      >
        <Select
          value={String(settings.updateCheckHours)}
          onValueChange={(v) => update({ updateCheckHours: Number(v) })}
        >
          <SelectTrigger id="update-interval" className="w-48">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {options.map((i) => (
              <SelectItem key={i.hours} value={String(i.hours)}>
                {i.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </Field>
      <Field label="Notifications" hint="One desktop notification per check that finds something.">
        <label htmlFor="update-notifications" className="flex items-center gap-2 text-body">
          <Switch
            id="update-notifications"
            checked={settings.updateNotifications}
            onCheckedChange={(on) => update({ updateNotifications: on })}
          />
          Notify me about new chapters
        </label>
      </Field>
    </>
  );
}
