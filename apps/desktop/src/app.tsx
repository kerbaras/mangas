import { useCallback, useMemo, useState } from "react";
import { NavRail } from "@/components/nav-rail";
import { TooltipProvider } from "@/components/ui/tooltip";
import { activeCount, DownloadsProvider, useDownloads } from "@/hooks/use-downloads";
import { type Navigation, NavigationContext, type Route } from "@/lib/navigation";
import { KindleScreen } from "@/screens/kindle";
import { LibraryScreen } from "@/screens/library";
import { MangaDetailsScreen } from "@/screens/manga-details";
import { SearchScreen } from "@/screens/search";
import { SettingsScreen } from "@/screens/settings";

function Screen({ route }: { route: Route }) {
  switch (route.screen) {
    case "library":
      return <LibraryScreen />;
    case "search":
      return <SearchScreen />;
    case "manga":
      return (
        <MangaDetailsScreen
          key={`${route.source}:${route.id}`}
          source={route.source}
          id={route.id}
        />
      );
    case "kindle":
      return <KindleScreen key={route.chapterIds?.join(",") ?? ""} chapterIds={route.chapterIds} />;
    case "settings":
      return <SettingsScreen />;
  }
}

function Shell({ nav }: { nav: Navigation }) {
  const downloads = useDownloads();
  return (
    <NavigationContext.Provider value={nav}>
      <div className="flex h-full">
        <NavRail activeDownloads={activeCount(downloads)} />
        <main className="flex-1 overflow-hidden">
          <Screen route={nav.route} />
        </main>
      </div>
    </NavigationContext.Provider>
  );
}

/** Routing is a history stack in state: rail clicks replace the stack, drill-downs push onto it. */
export function App() {
  const [stack, setStack] = useState<Route[]>([{ screen: "library" }]);

  const go = useCallback((route: Route) => {
    setStack((s) => (route.screen === "manga" ? [...s, route] : [route]));
  }, []);
  const back = useCallback(() => {
    setStack((s) => (s.length > 1 ? s.slice(0, -1) : [{ screen: "library" }]));
  }, []);

  const nav = useMemo<Navigation>(
    () => ({ route: stack[stack.length - 1], go, back, canGoBack: stack.length > 1 }),
    [stack, go, back],
  );

  return (
    <TooltipProvider delayDuration={400}>
      <DownloadsProvider>
        <Shell nav={nav} />
      </DownloadsProvider>
    </TooltipProvider>
  );
}
