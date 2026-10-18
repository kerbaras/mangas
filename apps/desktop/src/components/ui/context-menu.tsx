import * as Menu from "@radix-ui/react-context-menu";
import { Check, ChevronRight } from "lucide-react";
import type { ComponentProps } from "react";
import { cn } from "@/lib/cn";
import { menuContentClass, menuItemClass, menuLabelClass, menuSeparatorClass } from "./menu-styles";

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

export const ContextMenuSub = Menu.Sub;
export const ContextMenuRadioGroup = Menu.RadioGroup;

export function ContextMenuSubTrigger({
  className,
  children,
  ...props
}: ComponentProps<typeof Menu.SubTrigger>) {
  return (
    <Menu.SubTrigger
      className={cn(menuItemClass, "data-[state=open]:bg-surface-2", className)}
      {...props}
    >
      {children}
      <ChevronRight className="ml-auto" />
    </Menu.SubTrigger>
  );
}

export function ContextMenuSubContent({
  className,
  ...props
}: ComponentProps<typeof Menu.SubContent>) {
  return (
    <Menu.Portal>
      <Menu.SubContent className={cn(menuContentClass, className)} {...props} />
    </Menu.Portal>
  );
}

export function ContextMenuRadioItem({
  className,
  children,
  ...props
}: ComponentProps<typeof Menu.RadioItem>) {
  return (
    <Menu.RadioItem className={cn(menuItemClass, "pl-7", className)} {...props}>
      <Menu.ItemIndicator className="absolute left-2 [&_svg]:size-3.5">
        <Check />
      </Menu.ItemIndicator>
      {children}
    </Menu.RadioItem>
  );
}

export function ContextMenuCheckboxItem({
  className,
  children,
  ...props
}: ComponentProps<typeof Menu.CheckboxItem>) {
  return (
    <Menu.CheckboxItem className={cn(menuItemClass, "pl-7", className)} {...props}>
      <Menu.ItemIndicator className="absolute left-2 [&_svg]:size-3.5">
        <Check />
      </Menu.ItemIndicator>
      {children}
    </Menu.CheckboxItem>
  );
}

export function ContextMenuLabel({ className, ...props }: ComponentProps<typeof Menu.Label>) {
  return <Menu.Label className={cn(menuLabelClass, className)} {...props} />;
}
