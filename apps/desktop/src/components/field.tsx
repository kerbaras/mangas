import type { ReactNode } from "react";

type Props = { label: string; htmlFor?: string; hint?: ReactNode; children: ReactNode };

/** Label above control, hint below: the settings/form rhythm used on Kindle and Settings. */
export function Field({ label, htmlFor, hint, children }: Props) {
  return (
    <div className="flex flex-col gap-1.5">
      <label htmlFor={htmlFor} className="text-body font-medium">
        {label}
      </label>
      {children}
      {hint && <p className="text-caption text-ink-3">{hint}</p>}
    </div>
  );
}
