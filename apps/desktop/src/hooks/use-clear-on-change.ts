import { useEffect, useRef } from "react";

/**
 * Calls `clear` whenever `choice` changes. A change made while `busy` clears once it is done,
 * unless `keepRunResult`: then that run's result stays (a delivery tab picked mid-delivery).
 */
export function useClearOnChange(
  choice: string,
  busy: boolean,
  clear: () => void,
  keepRunResult = false,
) {
  const last = useRef(choice);
  useEffect(() => {
    if (last.current === choice) return;
    if (busy) {
      if (keepRunResult) last.current = choice;
      return;
    }
    last.current = choice;
    clear();
  }, [choice, busy, clear, keepRunResult]);
}
