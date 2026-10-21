import { useEffect, useState } from "react";
import { proxyInUse } from "./ipc";
import { useSettings } from "./settings";

/**
 * Whether downloads go through a proxy, for the setting as edited (not as last saved, so a save
 * still in flight can't leave a stale answer).
 */
export function useProxyInUse(): boolean {
  const [{ proxyMode, proxyUrl }] = useSettings();
  const [proxied, setProxied] = useState(false);
  useEffect(() => {
    let live = true;
    proxyInUse(proxyMode, proxyUrl).then(
      (yes) => live && setProxied(yes),
      () => live && setProxied(false),
    );
    return () => {
      live = false;
    };
  }, [proxyMode, proxyUrl]);
  return proxied;
}
