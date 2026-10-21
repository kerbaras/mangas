/** Provider sessions, status probes and the proxy test for the browser mock. */
import type { Backend, ProviderInfo, Settings } from "./ipc";

type Store = { get: () => Settings; set: (next: Settings) => void };
type ProvidersPart = Pick<
  Backend,
  | "listProviders"
  | "connectProvider"
  | "disconnectProvider"
  | "onProviderConnectEnd"
  | "providerStatus"
  | "testProxy"
  | "proxyInUse"
>;

const wait = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

/** The provider calls of `mockBackend`, reading and writing sessions through `store`. */
export function mockProviderSessions(providers: ProviderInfo[], store: Store): ProvidersPart {
  return {
    listProviders: async () => providers,
    async connectProvider(id) {
      await wait(800);
      store.set({
        ...store.get(),
        provider_sessions: {
          ...store.get().provider_sessions,
          [id]: { saved_at: new Date().toISOString() },
        },
      });
    },
    async disconnectProvider(id) {
      const { [id]: _, ...rest } = store.get().provider_sessions;
      store.set({ ...store.get(), provider_sessions: rest });
    },
    onProviderConnectEnd: async () => () => {},
    async providerStatus(id) {
      await wait(300);
      const info = providers.find((p) => p.id === id);
      if (!info) return { state: "unreachable", reason: `not found: source '${id}'` };
      if (info.needs_browser_session && !store.get().provider_sessions[id])
        return { state: "needs_session" };
      return { state: "ready" };
    },
    async testProxy(mode, url) {
      await wait(400);
      if (mode === "custom" && !/^https?:\/\/[^/]+/.test(url))
        return { state: "unreachable", reason: `invalid proxy URL "${url}"` };
      if (url.includes(":9"))
        return {
          state: "unreachable",
          reason: `network: couldn't connect through the proxy ${url}: connection refused`,
        };
      return { state: "ready" };
    },
    proxyInUse: async (mode, url) => mode === "custom" && !!url.trim(),
  };
}
