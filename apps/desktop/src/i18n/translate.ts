/** Values a message can interpolate: `{name}` in the text takes `vars.name`. */
export type Vars = Record<string, string | number>;

/**
 * The text for `key` with each `{name}` replaced by `vars.name`. A `count` var picks the
 * `<key>_one` form when it is 1 and that form exists (i18next-style plurals); a missing key
 * falls back to the key itself so a gap shows up instead of a blank.
 */
export function translate(messages: Readonly<Record<string, string>>, key: string, vars?: Vars) {
  const plural = vars?.count === 1 ? messages[`${key}_one`] : undefined;
  const text = plural ?? messages[key] ?? key;
  if (!vars) return text;
  return text.replace(/\{(\w+)\}/g, (whole, name: string) =>
    name in vars ? String(vars[name]) : whole,
  );
}
