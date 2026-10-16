import { isTauri } from "./ipc";

const isMacLike = () =>
  typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform);

/** macOS keeps its native traffic lights (overlay title bar); the header leaves room for them. */
export const hasTrafficLights = () => isTauri() && isMacLike();

/** Windows and Linux run undecorated, so the header draws its own window controls. */
export const drawsWindowControls = () => isTauri() && !isMacLike();
