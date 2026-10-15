import type { ReactNode } from "react";

type Props = { title: string; children?: ReactNode; action?: ReactNode };

export function EmptyState({ title, children, action }: Props) {
  return (
    <div className="mx-auto mt-24 max-w-sm text-center">
      <p className="text-subtitle font-semibold">{title}</p>
      {children && <p className="mt-2 text-ink-2">{children}</p>}
      {action && <div className="mt-5 flex justify-center">{action}</div>}
    </div>
  );
}
