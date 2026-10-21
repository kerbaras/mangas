/** Still bars while the description loads: a pulse repaints the window on every frame. */
export function DescriptionSkeleton() {
  return (
    <div role="status" aria-label="Loading details" className="mt-2 flex max-w-2xl flex-col gap-2">
      <span className="h-3 w-[28rem] max-w-full rounded-sm bg-surface-2" />
      <span className="h-3 w-[22rem] max-w-full rounded-sm bg-surface-2" />
    </div>
  );
}
