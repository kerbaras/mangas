import { BookOpen, Download, Search, Send } from "lucide-react";
import type { ReactNode } from "react";
import { LibraryScreen } from "@/screens/library";

const nav: { icon: ReactNode; label: string; active?: boolean }[] = [
  { icon: <BookOpen />, label: "Library", active: true },
  { icon: <Search />, label: "Search" },
  { icon: <Download />, label: "Downloads" },
  { icon: <Send />, label: "Send to Kindle" },
];

export function App() {
  return (
    <div className="flex h-full">
      <nav
        aria-label="Main"
        className="flex w-14 flex-col items-center gap-1 border-r border-stroke bg-surface py-3"
      >
        {nav.map((item) => (
          <button
            key={item.label}
            type="button"
            title={item.label}
            aria-current={item.active ? "page" : undefined}
            className="flex size-10 items-center justify-center rounded-md text-ink-2 hover:bg-surface-2 hover:text-ink aria-[current=page]:bg-accent-soft aria-[current=page]:text-accent [&_svg]:size-5"
          >
            {item.icon}
          </button>
        ))}
      </nav>
      <main className="flex-1 overflow-hidden">
        <LibraryScreen />
      </main>
    </div>
  );
}
