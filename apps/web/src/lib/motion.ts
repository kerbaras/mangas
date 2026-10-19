// "How it works": which step is active. The scrubbed motion itself is CSS (motion.css); in
// browsers without scroll timelines this script only marks reveals as they come into view.

export type Hit = { index: number; visible: boolean };

/** The step crossing the middle of the viewport; the latest one wins, none keeps the current. */
export function nextActive(current: number, hits: Hit[]): number {
  const visible = hits.filter((h) => h.visible && h.index >= 0).map((h) => h.index);
  return visible.length > 0 ? Math.max(...visible) : current;
}

/** Marks every reveal once it enters the viewport (fallback for `animation-timeline: view()`). */
function watchReveals(targets: Element[]) {
  const io = new IntersectionObserver(
    (entries) => {
      for (const e of entries) {
        if (!e.isIntersecting) continue;
        e.target.setAttribute("data-in", "");
        io.unobserve(e.target);
      }
    },
    { rootMargin: "0px 0px -10% 0px" },
  );
  for (const t of targets) io.observe(t);
}

/** Swaps the sticky window's screenshot to the step crossing the middle of the viewport. */
function watchSteps(root: Element) {
  const steps = [...root.querySelectorAll("[data-step]")];
  const frames = [...root.querySelectorAll("[data-frame]")];
  let active = 0;
  const show = (index: number) => {
    active = index;
    for (const [i, el] of steps.entries()) el.toggleAttribute("data-current", i === index);
    for (const [i, el] of frames.entries()) {
      el.toggleAttribute("data-current", i === index);
      el.setAttribute("aria-hidden", String(i !== index));
    }
  };
  const io = new IntersectionObserver(
    (entries) => {
      const hits = entries.map((e) => ({
        index: steps.indexOf(e.target),
        visible: e.isIntersecting,
      }));
      show(nextActive(active, hits));
    },
    { rootMargin: "-50% 0px -50% 0px" },
  );
  for (const s of steps) io.observe(s);
  show(0);
}

export function start(doc: Document) {
  const mode = doc.documentElement.dataset.motion;
  if (mode !== "css" && mode !== "io") return;
  if (mode === "io") watchReveals([...doc.querySelectorAll("[data-reveal]")]);
  const how = doc.querySelector("[data-steps]");
  if (how) watchSteps(how);
}
