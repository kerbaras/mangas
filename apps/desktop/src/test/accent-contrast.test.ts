import { describe, expect, it } from "vitest";
import tokens from "../styles/tokens.css?raw";

// #206: red text must pass WCAG AA (4.5:1) on every surface it can sit on, in both schemes.
const BACKGROUNDS = ["canvas", "surface", "surface-2", "accent-soft"];

function block(css: string, start: string): Record<string, string> {
  const at = css.indexOf(start);
  const body = css.slice(css.indexOf("{", at) + 1, css.indexOf("}", at));
  const vars: Record<string, string> = {};
  for (const [, name, value] of body.matchAll(/--color-([\w-]+):\s*(#[0-9a-f]{6});/gi)) {
    vars[name] = value;
  }
  return vars;
}

function luminance(hex: string): number {
  const [r, g, b] = [1, 3, 5].map((i) => {
    const c = Number.parseInt(hex.slice(i, i + 2), 16) / 255;
    return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function contrast(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

const light = block(tokens, "@theme");
const dark = { ...light, ...block(tokens, ":root.dark") };
const osDark = { ...light, ...block(tokens, ":root:not(.light)") };

describe("accent text contrast", () => {
  it.each([
    ["light", light],
    ["dark", dark],
  ])("passes 4.5:1 on every surface in %s mode", (_, palette) => {
    for (const bg of BACKGROUNDS) {
      expect(contrast(palette["accent-text"], palette[bg]), bg).toBeGreaterThanOrEqual(4.5);
    }
  });

  it("small labels on an accent-text fill pass 4.5:1 with canvas text", () => {
    for (const palette of [light, dark]) {
      expect(contrast(palette.canvas, palette["accent-text"])).toBeGreaterThanOrEqual(4.5);
    }
  });

  it("the OS-dark fallback matches the dark palette", () => {
    expect(osDark).toEqual(dark);
  });

  it("the dark accent itself is what fails, so the token is needed", () => {
    expect(contrast(dark.accent, dark.canvas)).toBeLessThan(4.5);
  });
});

// #236: every text colour token passes AA on every surface; white text passes on its fills.
const TEXT = ["ink", "ink-2", "ink-3", "accent-text", "ok", "warn", "bad"];

describe("text token contrast", () => {
  it.each([
    ["light", light],
    ["dark", dark],
  ])("passes 4.5:1 on every surface in %s mode", (_, palette) => {
    for (const fg of TEXT)
      for (const bg of BACKGROUNDS) {
        expect(contrast(palette[fg], palette[bg]), `${fg} on ${bg}`).toBeGreaterThanOrEqual(4.5);
      }
  });

  it("white text passes 4.5:1 on the accent fill and its hover in both modes", () => {
    for (const palette of [light, dark])
      for (const fill of ["accent-fill", "accent-fill-hover"]) {
        expect(contrast("#ffffff", palette[fill]), fill).toBeGreaterThanOrEqual(4.5);
      }
  });

  it("the muted nav badge (canvas on ink-3) passes 4.5:1 in both modes", () => {
    for (const palette of [light, dark]) {
      expect(contrast(palette.canvas, palette["ink-3"])).toBeGreaterThanOrEqual(4.5);
    }
  });

  it("the dark accent stays the brand red for strokes, but white text sits on the fill", () => {
    expect(contrast("#ffffff", dark.accent)).toBeLessThan(4.5);
  });
});

// White text belongs on the fill tokens: on `bg-accent` it fails in dark mode.
describe("white text on accent", () => {
  it("uses bg-accent-fill", () => {
    const offenders = Object.entries(sources)
      .filter(([, src]) =>
        /bg-accent(?![\w-])[^"]*text-white|!bg-accent(?![\w-])/.test(String(src)),
      )
      .map(([path]) => path.split("/").pop() ?? path);
    expect(offenders).toEqual([]);
  });
});

// Text classes may only use the readable token; `text-accent` is left for icons.
const sources = import.meta.glob("../**/*.tsx", { query: "?raw", import: "default", eager: true });
const ICONS_ONLY = ["error-boundary.tsx", "sonner.tsx", "nav-rail.tsx"];

describe("text-accent usage", () => {
  it("is limited to icon-only elements", () => {
    const offenders = Object.entries(sources)
      .filter(([, src]) => /(?<![\w-])text-accent(?![\w-])/.test(String(src)))
      .map(([path]) => path.split("/").pop() ?? path)
      .filter((name) => !ICONS_ONLY.includes(name));
    expect(offenders).toEqual([]);
  });
});
