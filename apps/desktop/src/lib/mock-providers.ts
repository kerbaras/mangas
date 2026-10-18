import type { ProviderInfo } from "./ipc";

const capabilities: ProviderInfo["capabilities"] = ["search", "chapters", "download"];

/** Icons stay null so the browser mock never reaches the network; the UI shows a monogram. */
export const sampleProviders: ProviderInfo[] = [
  {
    id: "mangadex",
    name: "MangaDex",
    site: "https://mangadex.org",
    languages: ["en", "es", "fr"],
    capabilities,
    needs_browser_session: false,
    icon_url: null,
  },
  {
    id: "mangadot",
    name: "MangaDot",
    site: "https://mangadot.net",
    languages: ["en"],
    capabilities,
    needs_browser_session: false,
    icon_url: null,
  },
  {
    id: "weebcentral",
    name: "WeebCentral",
    site: "https://weebcentral.com",
    languages: ["en"],
    capabilities,
    needs_browser_session: false,
    icon_url: null,
  },
];
