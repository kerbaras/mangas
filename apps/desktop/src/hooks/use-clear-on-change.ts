import { useEffect, useRef } from "react";

/** Calls `clear` whenever `choice` changes; a change made while `busy` clears once it is done. */
export function useClearOnChange(choice: string, busy: boolean, clear: () => void) {
  const last = useRef(choice);
  useEffect(() => {
    if (busy || last.current === choice) return;
    last.current = choice;
    clear();
  }, [choice, busy, clear]);
}
