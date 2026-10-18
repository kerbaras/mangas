import { useEffect, useRef } from "react";

/** Calls `clear` whenever `choice` changes, unless `busy`: a result never outlives its options. */
export function useClearOnChange(choice: string, busy: boolean, clear: () => void) {
  const last = useRef(choice);
  useEffect(() => {
    if (last.current === choice) return;
    last.current = choice;
    if (!busy) clear();
  }, [choice, busy, clear]);
}
