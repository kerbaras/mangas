import { useEffect, useState } from "react";

/** The current time, refreshed every `everyMs` so relative times ("2 min ago") stay true. */
export function useNow(everyMs: number) {
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), everyMs);
    return () => clearInterval(timer);
  }, [everyMs]);
  return now;
}
