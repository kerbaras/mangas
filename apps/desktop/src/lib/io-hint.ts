/** The folders an io error can be in, so its hint names the one that failed. */
export type Folders = { data?: string | null; export?: string | null };

const IO = /(?:^|: )io: (.+)$/s;

/**
 * What follows the core `io: ` prefix (`<path>: <reason>`), also when the message wraps it
 * (`Book failed: io: …`); null when the message is not an io error.
 */
export const ioRest = (message: string): string | null => IO.exec(message)?.[1] ?? null;

const trimmed = (dir: string) => dir.replace(/(.)[\\/]+$/, "$1");

/** `dir` without its trailing separator when `rest` names it or a path inside it. */
function within(rest: string, dir: string | null | undefined): string | null {
  if (!dir?.trim()) return null;
  const folder = trimmed(dir);
  const seps = /[\\/]$/.test(folder) ? [""] : [":", "/", "\\"];
  return seps.some((sep) => rest.startsWith(folder + sep)) ? folder : null;
}

/** The hint for an io error's `rest`: the export folder first (it may sit in the data folder). */
export function ioHint(rest: string, folders: Folders): string | null {
  const exportDir = within(rest, folders.export);
  if (exportDir)
    return `The export folder ${exportDir} couldn't be written. Check its permissions or choose another folder.`;
  const dataDir = within(rest, folders.data);
  if (dataDir)
    return `A file in the data folder ${dataDir} couldn't be read or written. Check the data folder in Settings.`;
  return null;
}
