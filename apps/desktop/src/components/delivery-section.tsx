import { useCallback, useEffect, useState } from "react";
import { RecentExports } from "@/components/recent-exports";
import { SendPanel } from "@/components/send-panel";
import { type ExportRecord, listExports } from "@/lib/ipc.send";

const RECENT = 5;

/** Below the export: send the book just written, or one picked from the recent exports. */
export function DeliverySection({ exported }: { exported: string | null }) {
  const [picked, setPicked] = useState<string | null>(null);
  const [records, setRecords] = useState<ExportRecord[]>([]);
  const reload = useCallback(() => {
    listExports(RECENT)
      .then(setRecords)
      .catch((e: unknown) => console.warn("export history not loaded", e));
  }, []);

  useEffect(() => {
    if (exported) setPicked(exported);
    reload();
  }, [exported, reload]);

  return (
    <div className="flex flex-col gap-4">
      {picked && <SendPanel key={picked} file={picked} onSent={reload} />}
      <RecentExports records={records} current={picked} onPick={setPicked} />
    </div>
  );
}
