import { createContext, useContext } from "react";
import type { Manga } from "@/lib/ipc";

export type Route =
  | { screen: "library" }
  | { screen: "search" }
  | { screen: "manga"; manga: Manga }
  | { screen: "kindle"; chapterIds?: string[] }
  | { screen: "settings" };

export type Navigation = {
  route: Route;
  go: (route: Route) => void;
  back: () => void;
  canGoBack: boolean;
};

export const NavigationContext = createContext<Navigation>({
  route: { screen: "library" },
  go: () => {},
  back: () => {},
  canGoBack: false,
});

export const useNavigation = () => useContext(NavigationContext);

/** Rail items highlight their whole section, so the details screen lights up "Library". */
export const railSection = (route: Route): "library" | "search" | "kindle" | "settings" =>
  route.screen === "manga" ? "library" : route.screen;
