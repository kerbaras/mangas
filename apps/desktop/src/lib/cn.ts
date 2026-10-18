import { type ClassValue, clsx } from "clsx";
import { extendTailwindMerge } from "tailwind-merge";

// The Fluent type ramp (`text-caption`, `text-body`, …) sets font sizes, not colours.
const twMerge = extendTailwindMerge({
  extend: {
    theme: { text: ["caption", "body", "body-lg", "subtitle", "title"] },
  },
});

export const cn = (...inputs: ClassValue[]) => twMerge(clsx(inputs));
