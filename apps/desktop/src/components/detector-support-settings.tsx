import { DetectorSupportActions } from "@/components/detector-support";
import { Field } from "@/components/field";
import { useDetectorSupport } from "@/hooks/use-detector-support";
import { detectorHint } from "@/lib/detector-hint";

/** Settings → Detectors: detector support status, download and remove. */
export function DetectorSupportSettings() {
  const support = useDetectorSupport();
  return (
    <Field label="Detector support" hint={detectorHint(support.status)}>
      <DetectorSupportActions support={support} manage />
    </Field>
  );
}
