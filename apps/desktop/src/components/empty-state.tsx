import type { ReactNode } from "react";

type Props = { title: string; icon?: ReactNode; children?: ReactNode; action?: ReactNode };

export function EmptyState({ title, icon, children, action }: Props) {
  return (
    <div className="mx-auto mt-24 max-w-sm text-center">
      {icon && (
        <div className="mx-auto mb-4 flex size-12 items-center justify-center rounded-full bg-surface-2 text-ink-3 [&_svg]:size-6">
          {icon}
        </div>
      )}
      <p className="text-subtitle font-semibold">{title}</p>
      {children && <div className="mt-2 text-ink-2">{children}</div>}
      {action && <div className="mt-5 flex justify-center gap-2">{action}</div>}
    </div>
  );
}
