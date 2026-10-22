import { Field } from "@/components/field";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import type { Via } from "@/hooks/use-send";
import { useNavigation } from "@/lib/navigation";
import { emailReady } from "@/lib/send-labels";
import { useSettings } from "@/lib/settings";

export type Delivery = "save" | Via;

/**
 * Why email can't send yet (no mail setup), or null when it can. Not the format: the export
 * writes EPUB for Email when the picked format isn't one Send to Kindle takes.
 */
export function useEmailBlock() {
  const [settings] = useSettings();
  if (emailReady(settings.kindleEmail)) return null;
  return { reason: "Email needs your Kindle address and a mail server." };
}

type Props = { value: Delivery; onChange: (d: Delivery) => void };

/** After writing: keep the files in the folder, or also send them to the Kindle. */
export function DeliveryPicker({ value, onChange }: Props) {
  const nav = useNavigation();
  const block = useEmailBlock();
  const hint = block && (
    <>
      {block.reason}{" "}
      <button
        type="button"
        className="text-accent-text underline-offset-2 hover:underline"
        onClick={() => nav.go({ screen: "settings" })}
      >
        Set up email
      </button>
    </>
  );
  // A blocked Email stays a focusable tab (aria-disabled, not disabled) inside the tablist, so
  // hover and keyboard focus show why; picking it changes nothing.
  const blocked = block !== null;
  const email = (
    <TabsTrigger
      value="email"
      aria-disabled={blocked || undefined}
      className={blocked ? "cursor-default text-ink-3 opacity-60 hover:text-ink-3" : undefined}
    >
      Email to Kindle
    </TabsTrigger>
  );
  const pick = (v: string) => {
    if (v !== "email" || !blocked) onChange(v as Delivery);
  };
  return (
    <Field label="Delivery" hint={hint}>
      <Tabs value={value} onValueChange={pick}>
        <TabsList aria-label="Delivery">
          <TabsTrigger value="save">Save to folder</TabsTrigger>
          <TabsTrigger value="usb">Kindle over USB</TabsTrigger>
          {block ? (
            <Tooltip>
              <TooltipTrigger asChild>{email}</TooltipTrigger>
              <TooltipContent className="max-w-64">{block.reason}</TooltipContent>
            </Tooltip>
          ) : (
            email
          )}
        </TabsList>
      </Tabs>
    </Field>
  );
}
