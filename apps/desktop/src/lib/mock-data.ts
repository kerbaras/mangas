import type { Chapter, Manga } from "./ipc";

export { sampleDevices } from "./mock-devices";

/** A flat two-tone cover with the title set small at the foot, so the grid reads as a shelf. */
export function coverDataUrl(title: string, hue: number): string {
  const safe = title.replace(/&/g, "&amp;").replace(/</g, "&lt;");
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="200" height="300" viewBox="0 0 200 300">
<rect width="200" height="300" fill="hsl(${hue} 22% 28%)"/>
<rect y="206" width="200" height="94" fill="hsl(${hue} 18% 18%)"/>
<rect x="18" y="26" width="164" height="160" rx="2" fill="hsl(${hue} 26% 36%)"/>
<text x="18" y="238" font-family="system-ui, sans-serif" font-size="13" font-weight="600" fill="#f2f2f0">${safe.slice(0, 20)}</text>
<text x="18" y="256" font-family="system-ui, sans-serif" font-size="13" font-weight="600" fill="#f2f2f0">${safe.slice(20, 40)}</text>
</svg>`;
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`;
}

type Seed = [id: string, title: string, status: Manga["status"], hue: number, blurb: string];

const seeds: Seed[] = [
  [
    "801513ba",
    "Berserk",
    "hiatus",
    210,
    "Guts, a lone mercenary branded for sacrifice, hunts the demon who took everything from him.",
  ],
  [
    "d1a9fdeb",
    "Vagabond",
    "hiatus",
    28,
    "A retelling of the life of Miyamoto Musashi, from reckless brawler to the sword saint.",
  ],
  [
    "32d76d19",
    "Solo Leveling",
    "completed",
    258,
    "The weakest hunter alive is handed a system that lets him grow without limit.",
  ],
  [
    "a96676e5",
    "Tower of God",
    "ongoing",
    190,
    "Bam climbs a tower where each floor is a test, chasing the only friend he has ever had.",
  ],
  [
    "a77742b1",
    "Chainsaw Man",
    "ongoing",
    8,
    "Denji trades a dog's life for a devil's heart and starts hunting devils for Public Safety.",
  ],
  [
    "5e4d1f2c",
    "Vinland Saga",
    "ongoing",
    160,
    "Thorfinn grows up among the Vikings who killed his father, then has to decide what a true warrior is.",
  ],
  [
    "c0f9b4a1",
    "Omniscient Reader's Viewpoint",
    "ongoing",
    240,
    "The only reader of a web novel watches it come true and knows how every scene ends.",
  ],
  [
    "7b5c3d9e",
    "Blue Period",
    "ongoing",
    200,
    "A model student discovers painting and gambles everything on getting into Tokyo's art university.",
  ],
  [
    "e3a5f6c2",
    "Lone Wolf and Cub",
    "completed",
    40,
    "A disgraced executioner wanders Edo Japan as an assassin, his infant son in a cart beside him.",
  ],
];

const extras: Seed[] = [
  [
    "1f8e2c7a",
    "Dorohedoro",
    "completed",
    110,
    "A man with a lizard's head searches a sorcerer-ridden city for the one who cursed him.",
  ],
  [
    "9c4b7e21",
    "Dungeon Meshi",
    "completed",
    70,
    "A party eats its way through a dungeon to rescue a friend before she is digested.",
  ],
  [
    "4d2a8b6f",
    "The Horizon",
    "completed",
    215,
    "Two children walk through a war-torn country, finding kindness in the ruins.",
  ],
  [
    "b7e6a1d3",
    "Witch Hat Atelier",
    "ongoing",
    300,
    "A tailor's daughter learns that magic is drawn, not spoken, and sets out to draw her own.",
  ],
];

const toManga = ([id, title, status, hue, description]: Seed): Manga => ({
  id,
  source: "mangadex",
  title,
  description,
  cover_url: coverDataUrl(title, hue),
  status,
});

export const sampleLibrary: Manga[] = seeds.map(toManga);
/** WeebCentral lists a few of the same series under its own ids. */
const mirrors = seeds
  .slice(0, 4)
  .map((s) => ({ ...toManga(s), source: "weebcentral", id: `wc-${s[0]}` }));
export const sampleCatalog: Manga[] = [...[...seeds, ...extras].map(toManga), ...mirrors];

const titles = [
  "The Black Swordsman",
  "The Brand",
  "Guardians of Desire",
  "The Golden Age",
  "Casca",
  "Zodd the Immortal",
  "Griffith",
  "Assassin",
  "Precious Thing",
  "Tombstone of Flame",
  "Sparks from a Sword Tip",
  "Night of Miracles",
];

const groups = ["Asura Scans", "Flame Comics", "Reaper Scans, Hive Toons"];
const url = "https://mangaplus.shueisha.co.jp/viewer/1000000";

/** Deterministic chapter list per manga: 24 chapters, 3 volumes, first few downloaded, a Spanish dup. */
export function sampleChapters(manga: Manga, dataDir: string): Chapter[] {
  const seed = manga.id.charCodeAt(0) + manga.id.charCodeAt(1);
  const chapters: Chapter[] = [];
  for (let n = 1; n <= 24; n++) {
    const downloaded = n <= 3 + (seed % 5);
    chapters.push({
      id: `${manga.id}-c${n}`,
      manga_id: manga.id,
      title: n % 4 === 2 ? null : titles[(n + seed) % titles.length],
      language: "en",
      volume: String(Math.ceil(n / 8)),
      number: n === 12 ? "12.5" : String(n),
      local_path: downloaded
        ? `${dataDir}/${manga.source}/${manga.id}/${String(n).padStart(3, "0")}`
        : null,
      group: groups[(n + seed) % (n > 20 ? 1 : groups.length)],
      availability: n < 23 ? undefined : n === 23 ? { kind: "locked" } : { kind: "external", url },
    });
    if (n === 1)
      chapters.push({
        ...chapters[0],
        id: `${manga.id}-c1-es`,
        language: "es",
        local_path: null,
        group: "Lector Scans",
      });
  }
  return chapters;
}

export { sampleProviders } from "./mock-providers";
