/** Browser mock of `search_all`: one group per enabled provider, same rules as the Rust side. */
import type { Manga, ProviderStatus, SearchGroup, Settings } from "./ipc";
import { sampleCatalog } from "./mock-data";
import { mockFlags } from "./mock-flags";
import { sampleProviders } from "./mock-providers";

const matches = (m: Manga, q: string) =>
  m.title.toLowerCase().includes(q) || m.description.toLowerCase().includes(q);

function statusOf(id: string, q: string, settings: Settings): ProviderStatus {
  const info = sampleProviders.find((p) => p.id === id);
  if (mockFlags.offline || q === "offline" || mockFlags.down.includes(id))
    return {
      state: "unreachable",
      reason: `network: error sending request for url (${info?.site ?? id}/search?q=${q})`,
    };
  if (info?.needs_browser_session && !settings.provider_sessions[id])
    return { state: "needs_session" };
  return { state: "ready" };
}

export function mockSearch(query: string, only: string[], settings: Settings): SearchGroup[] {
  const q = query.trim().toLowerCase();
  return sampleProviders
    .filter((p) => settings.providers_enabled?.[p.id] !== false)
    .filter((p) => only.length === 0 || only.includes(p.id))
    .map((p) => {
      const status: ProviderStatus = q ? statusOf(p.id, q, settings) : { state: "ready" };
      const items =
        q && status.state === "ready"
          ? sampleCatalog.filter((m) => m.source === p.id && matches(m, q))
          : [];
      return { provider: p.id, status, items };
    });
}
