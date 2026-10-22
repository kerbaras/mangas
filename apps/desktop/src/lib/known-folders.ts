/** The data folder `settings.ts` last loaded or saved, for hints that name it without importing it. */
let dataFolder = "";

export const knownDataFolder = (): string => dataFolder;

export function rememberDataFolder(dir: string) {
  dataFolder = dir;
}
