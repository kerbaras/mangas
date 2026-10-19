import { useCallback, useEffect, useMemo, useState } from "react";
import { AppUpdateBanner } from "@/components/app-update-banner";
import { NavRail } from "@/components/nav-rail";
import { Toaster } from "@/components/ui/sonner";
import { TooltipProvider } from "@/components/ui/tooltip";
import { DownloadsProvider, useQueue } from "@/hooks/use-downloads";
import { useLaunchAppUpdate } from "@/hooks/use-launch-app-update";
import { useTheme } from "@/hooks/use-theme";
import { useUpdateSchedule } from "@/hooks/use-update-schedule";
import { onAppFocus, watchAppFocus } from "@/lib/app-focus";
import { pendingCount } from "@/lib/download-sections";
import { mangaKey } from "@/lib/format";
import { type Navigation, NavigationContext, type Route } from "@/lib/navigation";
import { loadSettings } from "@/lib/settings";
import { DownloadsScreen } from "@/screens/downloads";
import { ExportScreen } from "@/screens/export";
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
    case "downloads":
      return <DownloadsScreen />;
    case "manga":
      return <MangaDetailsScreen key={mangaKey(route.manga)} manga={route.manga} />;
    case "export":
      return <ExportScreen key={route.chapterIds?.join(",") ?? ""} chapterIds={route.chapterIds} />;
    case "settings":
      return <SettingsScreen />;
  }
}

function Shell({ nav }: { nav: Navigation }) {
  const { jobs, gate } = useQueue();
  const pending = useMemo(() => pendingCount(jobs.values()), [jobs]);
  useUpdateSchedule();
  useTheme();
  useLaunchAppUpdate();
  return (
    <NavigationContext.Provider value={nav}>
      <div className="flex h-full overflow-hidden">
        <NavRail downloads={{ pending, held: gate.paused || gate.held }} />
        <main className="flex min-w-0 flex-1 flex-col overflow-hidden">
          <AppUpdateBanner />
          <div className="min-h-0 flex-1">
            <Screen route={nav.route} />
          </div>
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
