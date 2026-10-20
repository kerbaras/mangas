import { describe, expect, it } from "vitest";
import { GROUPS } from "@/screens/settings-groups";

// #261: UI text that sends the user to "Settings → X" must name a group that exists.
const sources = import.meta.glob(["../**/*.ts", "../**/*.tsx", "!../test/**"], {
  query: "?raw",
  import: "default",
  eager: true,
});

/** Every `Settings → X` inside a string literal on a non-comment line. */
function pointers(src: string): string[] {
  return src
    .split("\n")
    .filter((line) => !/^\s*(\/\/|\/?\*)/.test(line))
    .flatMap((line) => [...line.matchAll(/["'`][^"'`]*Settings → (\w+)/g)].map((m) => m[1]));
}

describe("Settings locations in UI text", () => {
  it("name a real settings group", () => {
    const labels = GROUPS.map((g) => g.label);
    const found = Object.entries(sources).flatMap(([path, src]) =>
      pointers(String(src)).map((group) => ({ path: path.split("/").pop(), group })),
    );
    expect(found.length).toBeGreaterThan(0);
    expect(found.filter(({ group }) => !labels.includes(group))).toEqual([]);
  });

  it("send detector support to Settings → Detectors", () => {
    const storage = String(sources["../components/storage-panel.tsx"]);
    expect(pointers(storage)).toEqual(["Detectors", "Detectors"]);
  });
});
