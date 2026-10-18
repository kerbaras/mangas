import { useCallback, useEffect, useMemo, useState } from "react";
import { NavRail } from "@/components/nav-rail";
import { Toaster } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";
import { activeCount, DownloadsProvider, useDownloads } from "@/hooks/use-downloads";
import { useUpdateSchedule } from "@/hooks/use-update-schedule";
import { onAppFocus, watchAppFocus } from "@/lib/app-focus";
import { mangaKey } from "@/lib/format";
import { type Navigation, NavigationContext, type Route } from "@/lib/navigation";
import { loadSettings } from "@/lib/settings";
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
      return <MangaDetailsScreen key={mangaKey(route.manga)} manga={route.manga} />;
    case "kindle":
      return <KindleScreen key={route.chapterIds?.join(",") ?? ""} chapterIds={route.chapterIds} />;
    case "settings":
      return <SettingsScreen />;
  }
}

function Shell({ nav }: { nav: Navigation }) {
  const downloads = useDownloads();
  useUpdateSchedule();
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
  useEffect(() => {
    void loadSettings();
    const stop = watchAppFocus();
    const off = onAppFocus(() => void loadSettings());
    return () => {
      off();
      stop();
    };
  }, []);

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
      <Toaster />
    </TooltipProvider>
  );
}
