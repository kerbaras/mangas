import { type ReactNode, useState } from "react";
import { DownloadRow } from "@/components/download-row";
import { Button } from "@/components/ui/button";
import { pagesPerSecond, type QueueState } from "@/lib/download-queue";
import type { DownloadJob } from "@/lib/ipc.downloads";

type Props = {
  title: string;
  jobs: DownloadJob[];
  samples: QueueState["samples"];
  action?: ReactNode;
  /** Rows shown at first and added by "Show more"; every row when unset. */
  pageSize?: number;
};

/** Nothing while empty, so a cleared section starts again from one page. */
export function DownloadSection(props: Props) {
  return props.jobs.length === 0 ? null : <PagedSection {...props} />;
}

function PagedSection({ title, jobs, samples, action, pageSize }: Props) {
  const [limit, setLimit] = useState(pageSize ?? Number.POSITIVE_INFINITY);
  const id = `downloads-${title.toLowerCase()}`;
  const shown = jobs.slice(0, limit);
  const hidden = jobs.length - shown.length;
  return (
    <section aria-labelledby={id} className="mb-6">
      <div className="flex h-9 items-center gap-2 px-4">
        <h2 id={id} className="text-body font-semibold">
          {title}
        </h2>
        <span className="text-caption text-ink-3 tabular-nums">{jobs.length}</span>
        <div className="ml-auto">{action}</div>
      </div>
      <ul className="border-t border-stroke/60">
        {shown.map((job) => (
          <DownloadRow key={job.id} job={job} speed={pagesPerSecond(samples.get(job.id))} />
        ))}
      </ul>
      {hidden > 0 && pageSize && (
        <div className="flex h-11 items-center gap-2 px-4">
          <Button variant="ghost" size="sm" onClick={() => setLimit(shown.length + pageSize)}>
            Show {Math.min(hidden, pageSize)} more
          </Button>
          <span className="text-caption text-ink-3 tabular-nums">{hidden} not shown</span>
        </div>
      )}
    </section>
  );
}
