import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, vi } from "vitest";
import { TooltipProvider } from "@/components/ui/tooltip";

// The app mounts one TooltipProvider at the root (app.tsx); every render gets it too.
vi.mock("@testing-library/react", async (importOriginal) => {
  const rtl = await importOriginal<typeof import("@testing-library/react")>();
  type Options = Parameters<typeof rtl.render>[1];
  const render = (ui: ReactNode, options?: Options) => {
    const Inner = options?.wrapper;
    const wrapper = ({ children }: { children: ReactNode }) => (
      <TooltipProvider delayDuration={0}>
        {Inner ? <Inner>{children}</Inner> : children}
      </TooltipProvider>
    );
    return rtl.render(ui, { ...options, wrapper } as Options);
  };
  return { ...rtl, render };
});

afterEach(() => cleanup());
