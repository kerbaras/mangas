/** Opening folders in the file manager (`commands/folders.rs`). */
import { backend } from "./ipc";

export type FoldersBackend = {
  /** Opens `path` if it is a folder of the app's (data, logs, downloads) or an export folder. */
  openFolder(path: string): Promise<void>;
};

export const openFolder = async (path: string) => (await backend()).openFolder(path);
