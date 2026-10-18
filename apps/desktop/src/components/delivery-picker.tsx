import { Field } from "@/components/field";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import type { Via } from "@/hooks/use-send";
import type { ExportFormat } from "@/lib/ipc";
import { useNavigation } from "@/lib/navigation";
import { emailReady, emailRefusal } from "@/lib/send-labels";
import { useSettings } from "@/lib/settings";

export type Delivery = "save" | Via;

/** Why email can't take this export (format, or no mail setup), or null when it can. */
export function useEmailBlock(format: ExportFormat) {
  const [settings] = useSettings();
  const refusal = emailRefusal(`book.${format}`);
  if (refusal) return { reason: refusal, setup: false };
  if (emailReady(settings.kindleEmail)) return null;
  return { reason: "Email needs your Kindle address and a mail server.", setup: true };
}

type Props = { value: Delivery; onChange: (d: Delivery) => void; format: ExportFormat };

/** After writing: keep the files in the folder, or also send them to the Kindle. */
export function DeliveryPicker({ value, onChange, format }: Props) {
  const nav = useNavigation();
  const block = useEmailBlock(format);
  const hint = block && (
    <>
      {block.reason}{" "}
      {block.setup && (
        <button
          type="button"
          className="text-accent underline-offset-2 hover:underline"
          onClick={() => nav.go({ screen: "settings" })}
        >
          Set up email
        </button>
      )}
    </>
  );
  return (
    <Field label="Delivery" hint={hint}>
      <Tabs value={value} onValueChange={(v) => onChange(v as Delivery)}>
        <TabsList aria-label="Delivery">
          <TabsTrigger value="save">Save to folder</TabsTrigger>
          <TabsTrigger value="usb">Kindle over USB</TabsTrigger>
          <TabsTrigger value="email" disabled={block !== null}>
            Email to Kindle
          </TabsTrigger>
        </TabsList>
      </Tabs>
    </Field>
  );
}
