import { useEffect, useState } from "react";
import { proxyInUse } from "./ipc";
import { useSettings } from "./settings";

/** Whether downloads go through a proxy, asked again whenever the proxy setting changes. */
export function useProxyInUse(): boolean {
  const [{ proxyMode, proxyUrl }] = useSettings();
  const [proxied, setProxied] = useState(false);
  // biome-ignore lint/correctness/useExhaustiveDependencies: the backend reads the saved setting; these only say when to ask again.
  useEffect(() => {
    let live = true;
    proxyInUse().then(
      (yes) => live && setProxied(yes),
      () => live && setProxied(false),
    );
    return () => {
      live = false;
    };
  }, [proxyMode, proxyUrl]);
  return proxied;
}
