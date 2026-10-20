import { Minus, Square, X } from "lucide-react";
import { drawsWindowControls } from "@/lib/platform";

type Action = "minimize" | "toggleMaximize" | "close";

async function run(action: Action) {
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  await getCurrentWindow()[action]();
}

const base =
  "flex h-12 w-11 items-center justify-center text-ink-2 transition-colors [&_svg]:size-4";
/** Close keeps its own hover: next to `hover:bg-surface-2` the red fill lost the cascade. */
const plain = `${base} hover:bg-surface-2 hover:text-ink`;

/** Minimize / maximize / close for undecorated windows (Windows, Linux); nothing on macOS. */
export function WindowControls() {
  if (!drawsWindowControls()) return null;
  return (
    <div className="-mr-4 ml-2 flex shrink-0 self-stretch">
      <button type="button" aria-label="Minimize" className={plain} onClick={() => run("minimize")}>
        <Minus />
      </button>
      <button
        type="button"
        aria-label="Maximize"
        className={plain}
        onClick={() => run("toggleMaximize")}
      >
        <Square className="[&&]:size-3.5" />
      </button>
      <button
        type="button"
        aria-label="Close"
        className={`${base} hover:bg-accent-fill hover:text-white`}
        onClick={() => run("close")}
      >
        <X />
      </button>
    </div>
  );
}
