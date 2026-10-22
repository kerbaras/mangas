import { Scissors } from "lucide-react";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import type { Oversized } from "@/lib/ipc.export";
import type { Levers } from "@/lib/ipc.volumes";
import { PART_MB, SEND_TO_KINDLE_MB, WEB_MB, WEB_PART_MB } from "@/lib/kindle-size";
import { fileName } from "@/lib/send-labels";

type Props = {
  /** Written files over the delivery's budget, measured on disk after the export. */
  files: Oversized[];
  /** Email was picked: the books were written but not emailed. */
  email: boolean;
  /** Turns on splitting into parts; absent when the book was already split. */
  onSplit?: () => void;
  /** The plan's levers for these books (`kindle::book_levers`); null while unknown. */
  levers: Levers | null;
};

const sized = ({ file, bytes }: Oversized) => `${fileName(file)} is ${(bytes / 1e6).toFixed(1)} MB`;

/** Books written over their delivery's budget, with the split that makes them fit. */
export function OversizedAlert({ files, email, onSplit, levers }: Props) {
  if (files.length === 0) return null;
  const [limit, part] = email ? [SEND_TO_KINDLE_MB, PART_MB] : [WEB_MB, WEB_PART_MB];
  const fix = `To fit, ${ways(onSplit ? `split it into parts of ${part} MB` : "lower the part size", levers)}, then export again.`;
  return (
    <Alert variant={email ? "destructive" : "default"}>
      <Scissors />
      <AlertTitle>
        {email ? "Not emailed: too big for Send to Kindle" : "Too big for Send to Kindle"}
      </AlertTitle>
      <AlertDescription>
        {files.map(sized).join(", ")}; Send to Kindle {email ? "by email" : "on the web"} takes up
        to {limit} MB per file. {fix}
        {onSplit && (
          <Button size="sm" onClick={onSplit} className="mt-1">
            <Scissors /> Split into {part} MB parts
          </Button>
        )}
      </AlertDescription>
    </Alert>
  );
}

/** The levers as the plan offers them: only a quality that fits, only when one does. */
function ways(split: string, levers: Levers | null) {
  const n = levers?.chapters_per_book;
  const q = levers?.jpeg_quality;
  const all = [split, n && `put ${n} chapters in each book`, q && `lower the JPEG quality to ${q}`];
  return all.filter(Boolean).join(", or ");
}
