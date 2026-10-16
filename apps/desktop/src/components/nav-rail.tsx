import { BookOpen, Search, Send, Settings } from "lucide-react";
import type { ReactNode } from "react";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { cn } from "@/lib/cn";
import { type Route, railSection, useNavigation } from "@/lib/navigation";
import { hasTrafficLights } from "@/lib/platform";

type Item = { icon: ReactNode; label: string; route: Route; badge?: number };

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
            <span className="absolute top-1 right-1 flex h-4 min-w-4 items-center justify-center rounded-full bg-accent px-1 text-[10px] font-semibold text-white">
              {item.badge}
            </span>
          ) : null}
        </button>
      </TooltipTrigger>
      <TooltipContent side="right">{item.label}</TooltipContent>
    </Tooltip>
  );
}

export function NavRail({ activeDownloads }: { activeDownloads: number }) {
  const { route } = useNavigation();
  const section = railSection(route);
  const top: Item[] = [
    { icon: <BookOpen />, label: "Library", route: { screen: "library" }, badge: activeDownloads },
    { icon: <Search />, label: "Search", route: { screen: "search" } },
    { icon: <Send />, label: "Send to Kindle", route: { screen: "kindle" } },
  ];
  const settings: Item = { icon: <Settings />, label: "Settings", route: { screen: "settings" } };
  return (
    <nav
      aria-label="Main"
      data-tauri-drag-region
      className={cn(
        "flex shrink-0 flex-col items-center gap-1 border-r border-stroke bg-surface py-3",
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
