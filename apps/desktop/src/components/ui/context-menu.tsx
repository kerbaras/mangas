import * as Menu from "@radix-ui/react-context-menu";
import type { ComponentProps } from "react";
import { cn } from "@/lib/cn";
import { menuContentClass, menuItemClass, menuSeparatorClass } from "./menu-styles";

export const ContextMenu = Menu.Root;
export const ContextMenuTrigger = Menu.Trigger;

export function ContextMenuContent({ className, ...props }: ComponentProps<typeof Menu.Content>) {
  return (
    <Menu.Portal>
      <Menu.Content className={cn(menuContentClass, className)} {...props} />
    </Menu.Portal>
  );
}

export function ContextMenuItem({ className, ...props }: ComponentProps<typeof Menu.Item>) {
  return <Menu.Item className={cn(menuItemClass, className)} {...props} />;
}

export function ContextMenuSeparator({
  className,
  ...props
}: ComponentProps<typeof Menu.Separator>) {
  return <Menu.Separator className={cn(menuSeparatorClass, className)} {...props} />;
}
