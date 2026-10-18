import type { ReactNode } from "react";
import { DownloadRow } from "@/components/download-row";
import { pagesPerSecond, type QueueState } from "@/lib/download-queue";
import type { DownloadJob } from "@/lib/ipc.downloads";

type Props = {
  title: string;
  jobs: DownloadJob[];
  samples: QueueState["samples"];
  action?: ReactNode;
};

export function DownloadSection({ title, jobs, samples, action }: Props) {
  if (jobs.length === 0) return null;
  const id = `downloads-${title.toLowerCase()}`;
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
        {jobs.map((job) => (
          <DownloadRow key={job.id} job={job} speed={pagesPerSecond(samples.get(job.id))} />
        ))}
      </ul>
    </section>
  );
}
