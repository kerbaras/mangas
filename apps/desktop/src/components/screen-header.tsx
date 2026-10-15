import { ArrowLeft } from "lucide-react";
import type { ReactNode } from "react";
import { Button } from "@/components/ui/button";
import { useNavigation } from "@/lib/navigation";

type Props = { title: ReactNode; subtitle?: ReactNode; back?: boolean; children?: ReactNode };

/** 48px bar shared by every screen: title left, actions right, optional back arrow. */
export function ScreenHeader({ title, subtitle, back, children }: Props) {
  const nav = useNavigation();
  return (
    <header className="flex h-12 shrink-0 items-center gap-3 border-b border-stroke px-4">
      {back && (
        <Button variant="ghost" size="icon" aria-label="Back" onClick={nav.back} className="-ml-2">
          <ArrowLeft />
        </Button>
      )}
      <div className="flex min-w-0 items-baseline gap-2">
        <h1 className="truncate text-body-lg font-semibold">{title}</h1>
        {subtitle && <span className="truncate text-caption text-ink-3">{subtitle}</span>}
      </div>
      <div className="ml-auto flex items-center gap-2">{children}</div>
    </header>
  );
}
