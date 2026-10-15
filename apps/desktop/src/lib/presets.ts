import type { Preset } from "./ipc";

export type PresetInfo = {
  id: Preset;
  label: string;
  /** One line, user-facing: what you trade for what. */
  description: string;
  /** Where the preset would cut a sample strip, as fractions of its height (for the diagram). */
  cuts: number[];
  /** How much a page may shrink to avoid a cut, as a fraction (0 = never). */
  shrink: number;
};

export const PRESETS: readonly PresetInfo[] = [
  {
    id: "balanced",
    label: "Balanced",
    description: "Shrinks a page up to 10% when that avoids cutting through art. Good default.",
    cuts: [0.27, 0.56, 0.84],
    shrink: 0.1,
  },
  {
    id: "fewest-cuts",
    label: "Fewest cuts",
    description: "Shrinks up to 25% so fewer pages are cut; text gets smaller on screen.",
    cuts: [0.34, 0.7],
    shrink: 0.25,
  },
  {
    id: "no-shrink",
    label: "No shrink",
    description: "Keeps artwork at native size and cuts at the nearest gutter; the most pages.",
    cuts: [0.22, 0.44, 0.66, 0.88],
    shrink: 0,
  },
];

export const DEFAULT_PRESET: Preset = "balanced";

export const presetInfo = (id: Preset): PresetInfo =>
  PRESETS.find((p) => p.id === id) ?? PRESETS[0];

export const isPreset = (v: unknown): v is Preset => PRESETS.some((p) => p.id === v);
