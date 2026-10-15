import { useCallback, useEffect, useRef, useState } from "react";
import { errorMessage } from "@/lib/format";

export type Async<T> =
  | { kind: "loading" }
  | { kind: "ready"; data: T }
  | { kind: "error"; message: string };

/** Runs `load` whenever `deps` change; `reload` reruns it without flashing the loading state. */
export function useAsync<T>(load: () => Promise<T>, deps: unknown[]) {
  const [state, setState] = useState<Async<T>>({ kind: "loading" });
  const [tick, setTick] = useState(0);
  const loadRef = useRef(load);
  loadRef.current = load;

  // biome-ignore lint/correctness/useExhaustiveDependencies: deps are the caller's contract; tick forces a rerun
  useEffect(() => {
    let alive = true;
    loadRef
      .current()
      .then((data) => alive && setState({ kind: "ready", data }))
      .catch((e: unknown) => alive && setState({ kind: "error", message: errorMessage(e) }));
    return () => {
      alive = false;
    };
  }, [...deps, tick]);

  const reload = useCallback(() => setTick((t) => t + 1), []);
  const patch = useCallback(
    (fn: (data: T) => T) =>
      setState((s) => (s.kind === "ready" ? { kind: "ready", data: fn(s.data) } : s)),
    [],
  );
  return { state, reload, patch };
}
