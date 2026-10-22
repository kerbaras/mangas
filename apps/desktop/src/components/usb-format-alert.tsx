import { Ban, Mail, TriangleAlert, Usb } from "lucide-react";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import type { UsbVerdict } from "@/lib/ipc.send";

type Props = {
  verdict: UsbVerdict;
  /** Copies the book even so; offered for a format the Kindle may not open. */
  onCopyAnyway?: () => void;
  /** Sends by email instead; offered for an EPUB, whose route is Email. */
  onEmail?: () => void;
};

/** Why a USB copy didn't start: the Kindle doesn't open the format (no copy) or may not (Copy anyway). */
export function UsbFormatAlert({ verdict, onCopyAnyway, onEmail }: Props) {
  const warn = verdict.decision === "warn";
  const title = warn
    ? `${verdict.device} may not open ${verdict.format || "this file"}`
    : `${verdict.device} doesn't open ${verdict.format}`;
  return (
    <Alert variant={warn ? "default" : "destructive"}>
      {warn ? <TriangleAlert /> : <Ban />}
      <AlertTitle>{title}</AlertTitle>
      <AlertDescription>
        {verdict.reason}
        {warn && (
          <div className="mt-1 flex gap-2">
            {onEmail && verdict.format === "EPUB" && (
              <Button size="sm" onClick={onEmail}>
                <Mail /> Use Email
              </Button>
            )}
            {onCopyAnyway && (
              <Button size="sm" onClick={onCopyAnyway}>
                <Usb /> Copy anyway
              </Button>
            )}
          </div>
        )}
      </AlertDescription>
    </Alert>
  );
}
