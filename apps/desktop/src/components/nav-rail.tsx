import { BookOpen, Download, Search, Send, Settings } from "lucide-react";
import type { ReactNode } from "react";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { cn } from "@/lib/cn";
import { type Route, railSection, useNavigation } from "@/lib/navigation";
import { hasTrafficLights } from "@/lib/platform";

type Item = { icon: ReactNode; label: string; route: Route; badge?: number; muted?: boolean };

function RailButton({ item, active }: { item: Item; active: boolean }) {
  const nav = useNavigation();
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <button
          type="button"
          aria-label={item.label}
          aria-current={active ? "page" : undefined}
          onClick={() => nav.go(item.route)}
          className="relative flex size-10 items-center justify-center rounded-md text-ink-2 transition-colors hover:bg-surface-2 hover:text-ink aria-[current=page]:bg-accent-soft aria-[current=page]:text-accent [&_svg]:size-5"
        >
          {item.icon}
          {item.badge ? (
            <span
              className={cn(
                "absolute top-1 right-1 flex h-4 min-w-4 items-center justify-center rounded-full px-1 text-[10px] font-semibold",
                item.muted ? "bg-ink-3 text-white" : "bg-accent-text text-canvas",
              )}
            >
              {item.badge}
            </span>
          ) : null}
        </button>
      </TooltipTrigger>
      <TooltipContent side="right">{item.label}</TooltipContent>
    </Tooltip>
  );
}

/** `pending`: running and queued jobs; `held`: the queue is paused or held, so they wait. */
type Props = { downloads: { pending: number; held: boolean } };

export function NavRail({ downloads }: Props) {
  const { route } = useNavigation();
  const section = railSection(route);
  const top: Item[] = [
    { icon: <BookOpen />, label: "Library", route: { screen: "library" } },
    { icon: <Search />, label: "Search", route: { screen: "search" } },
    {
      icon: <Download />,
      label: downloads.held ? "Downloads (paused)" : "Downloads",
      route: { screen: "downloads" },
      badge: downloads.pending,
      muted: downloads.held,
    },
    { icon: <Send />, label: "Export", route: { screen: "export" } },
  ];
  const settings: Item = { icon: <Settings />, label: "Settings", route: { screen: "settings" } };
  return (
    <nav
      aria-label="Main"
      data-tauri-drag-region
      className={cn(
        "flex shrink-0 flex-col items-center gap-1 overflow-y-auto overscroll-contain border-r border-stroke bg-surface py-3",
        hasTrafficLights() ? "w-[72px] pt-[60px]" : "w-14",
      )}
    >
      {top.map((item) => (
        <RailButton key={item.label} item={item} active={section === item.route.screen} />
      ))}
      <div data-tauri-drag-region className="flex-1" />
      <RailButton item={settings} active={section === "settings"} />
    </nav>
  );
}
