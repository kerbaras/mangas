import type { PresetInfo } from "@/lib/presets";

/** The panels of a sample strip, as [top, height] fractions. Shared so every preset cuts the same art. */
const panels: [number, number][] = [
  [0.02, 0.17],
  [0.21, 0.12],
  [0.35, 0.19],
  [0.56, 0.1],
  [0.68, 0.16],
  [0.86, 0.12],
];

const W = 36;
const H = 120;

/** A tall webtoon strip with the preset's cut lines drawn across it in hanko red. */
export function CutStrip({ preset, active }: { preset: PresetInfo; active: boolean }) {
  const pages = preset.cuts.length + 1;
  return (
    <svg
      width={W}
      height={H}
      viewBox={`0 0 ${W} ${H}`}
      aria-label={`${pages} pages, ${preset.cuts.length} cuts`}
      className="shrink-0"
    >
      <title>
        {pages} pages, {preset.cuts.length} cuts
      </title>
      <rect
        x="0.5"
        y="0.5"
        width={W - 1}
        height={H - 1}
        rx="2"
        className="fill-surface stroke-stroke"
      />
      {panels.map(([top, height]) => (
        <rect
          key={top}
          x="5"
          y={top * H}
          width={W - 10}
          height={height * H}
          rx="1"
          className={active ? "fill-ink-3/60" : "fill-ink-3/35"}
        />
      ))}
      {preset.cuts.map((cut) => (
        <line
          key={cut}
          x1="-2"
          x2={W + 2}
          y1={cut * H}
          y2={cut * H}
          strokeDasharray="3 2"
          strokeWidth="1.25"
          className={active ? "stroke-accent" : "stroke-ink-3"}
        />
      ))}
    </svg>
  );
}
