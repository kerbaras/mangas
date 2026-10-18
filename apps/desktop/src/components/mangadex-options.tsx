import { useEffect, useState } from "react";
import { Field } from "@/components/field";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Switch } from "@/components/ui/switch";
import {
  CONTENT_RATINGS,
  editMangaDexOptions,
  MANGADEX,
  type MangaDexOptions,
  parseGroups,
  parseMangaDexOptions,
  toggleRating,
} from "@/lib/mangadex-options";
import { useSettings } from "@/lib/settings";

const ratingLabel = (r: string) => r[0].toUpperCase() + r.slice(1);

/** Settings > Providers > MangaDex: search filters, image quality and group preference. */
export function MangaDexOptionsForm() {
  const [settings, update] = useSettings();
  const stored = settings.providerOptions[MANGADEX];
  const options = parseMangaDexOptions(stored);
  const save = (patch: Partial<MangaDexOptions>) =>
    update({
      providerOptions: {
        ...settings.providerOptions,
        [MANGADEX]: editMangaDexOptions(stored, patch),
      },
    });

  const joined = options.preferred_groups.join(", ");
  const [groups, setGroups] = useState(joined);
  useEffect(() => setGroups(joined), [joined]);

  return (
    <div className="flex flex-col gap-4 rounded-md border border-stroke p-3">
      <Field label="Content rating" hint="Search only shows titles with these ratings.">
        <div className="flex flex-wrap gap-x-4 gap-y-2">
          {CONTENT_RATINGS.map((r) => (
            <label key={r} htmlFor={`rating-${r}`} className="flex items-center gap-2 text-body">
              <Checkbox
                id={`rating-${r}`}
                checked={options.content_ratings.includes(r)}
                onChange={() => save({ content_ratings: toggleRating(options.content_ratings, r) })}
              />
              {ratingLabel(r)}
            </label>
          ))}
        </div>
      </Field>
      <div className="flex items-start justify-between gap-4">
        <div className="flex flex-col gap-0.5">
          <label htmlFor="mangadex-data-saver" className="text-body font-medium">
            Data saver
          </label>
          <p className="text-caption text-ink-3">
            Downloads compressed JPEG pages: smaller and faster, lower quality.
          </p>
        </div>
        <Switch
          id="mangadex-data-saver"
          checked={options.data_saver}
          onCheckedChange={(on) => save({ data_saver: on })}
        />
      </div>
      <Field
        label="Preferred groups"
        htmlFor="mangadex-groups"
        hint="When several groups upload the same chapter, the first match in this list wins; otherwise the newest upload is kept. Comma-separated."
      >
        <Input
          id="mangadex-groups"
          value={groups}
          placeholder="e.g. Asura Scans, Flame Comics"
          onChange={(e) => setGroups(e.target.value)}
          onBlur={() => save({ preferred_groups: parseGroups(groups) })}
        />
      </Field>
    </div>
  );
}
