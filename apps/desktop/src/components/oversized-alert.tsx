import { Scissors } from "lucide-react";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import type { Oversized } from "@/lib/ipc.export";
import { PART_MB, SEND_TO_KINDLE_MB } from "@/lib/kindle-size";
import { fileName } from "@/lib/send-labels";

type Props = {
  /** Written files over the email limit, measured on disk after the export. */
  files: Oversized[];
  /** Email was picked: the books were written but not emailed. */
  email: boolean;
  /** Turns on splitting into parts; absent when the book was already split. */
  onSplit?: () => void;
};

const sized = ({ file, bytes }: Oversized) => `${fileName(file)} is ${(bytes / 1e6).toFixed(1)} MB`;

/** Books written too big to email, with the split that makes them fit. */
export function OversizedAlert({ files, email, onSplit }: Props) {
  if (files.length === 0) return null;
  const fix = onSplit
    ? `Split it into parts of ${PART_MB} MB and export again, or lower the JPEG quality.`
    : "Lower the part size or the JPEG quality and export again.";
  return (
    <Alert variant={email ? "destructive" : "default"}>
      <Scissors />
      <AlertTitle>
        {email ? "Not emailed: too big for Send to Kindle" : "Too big to email"}
      </AlertTitle>
      <AlertDescription>
        {files.map(sized).join(", ")}; Send to Kindle by email takes up to {SEND_TO_KINDLE_MB} MB
        per file. {fix}
        {onSplit && (
          <Button size="sm" onClick={onSplit} className="mt-1">
            <Scissors /> Split into {PART_MB} MB parts
          </Button>
        )}
      </AlertDescription>
    </Alert>
  );
}
