/** Shared classes for Radix dropdown and context menus so both read as the same native menu. */
export const menuContentClass =
  "z-50 min-w-40 rounded-md border border-stroke bg-surface p-1 shadow-lg";

export const menuItemClass =
  "relative flex h-7 cursor-default select-none items-center gap-2 rounded-sm px-2 text-body text-ink outline-none data-[highlighted]:bg-surface-2 data-[disabled]:pointer-events-none data-[disabled]:opacity-50 [&_svg]:size-4 [&_svg]:text-ink-2";

export const menuLabelClass = "px-2 py-1 text-caption text-ink-3";

export const menuSeparatorClass = "-mx-1 my-1 h-px bg-stroke";
