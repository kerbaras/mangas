/** Mirrors `mangas_core::library::proxy_url`: a bare host:port gets `http://`. */
export function proxyUrlError(raw: string): string | null {
  const trimmed = raw.trim();
  if (!trimmed) return "Type the proxy's address, like http://127.0.0.1:8888.";
  const full = trimmed.includes("://") ? trimmed : `http://${trimmed}`;
  try {
    const url = new URL(full);
    if (url.protocol !== "http:" && url.protocol !== "https:") return "Use http:// or https://.";
    if (!url.hostname) return "The address needs a host.";
    return null;
  } catch {
    return "That isn't a valid address.";
  }
}
