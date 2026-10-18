/** In-browser presets and a fake preview: grey pages whose height follows the options. */
import type { CutPreviewBackend, ExportPreset } from "./ipc.cut-preview";

const page = (height: number, shade: number) =>
  `data:image/svg+xml;utf8,${encodeURIComponent(
    `<svg xmlns="http://www.w3.org/2000/svg" width="600" height="${height}"><rect width="600" height="${height}" fill="rgb(${shade},${shade},${shade})"/></svg>`,
  )}`;

export function mockCutPreview(): CutPreviewBackend {
  const presets = new Map<string, ExportPreset>();
  const key = (source: string, id: string) => `${source}:${id}`;
  return {
    async getExportPreset(source, mangaId) {
      return presets.get(key(source, mangaId)) ?? null;
    },
    async setExportPreset(source, mangaId, preset) {
      if (Object.keys(preset).length === 0) presets.delete(key(source, mangaId));
      else presets.set(key(source, mangaId), preset);
    },
    async clearExportPreset(source, mangaId) {
      return presets.delete(key(source, mangaId));
    },
    async cancelCutPreview() {},
    async previewCut(_dir, options) {
      await new Promise((r) => setTimeout(r, 300));
      const height = Math.round(800 * (options.page_height ?? 1));
      const count = Math.min(12, Math.ceil(8 / (options.page_height ?? 1)));
      const pages = Array.from({ length: count }, (_, i) => ({
        src: page(height, 200 + (i % 2) * 30),
        width: 600,
        height,
      }));
      return { pages, more: true, elapsed_ms: 300 };
    },
  };
}
