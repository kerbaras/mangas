import { Wand2 } from "lucide-react";
import { Field } from "@/components/field";
import { Button } from "@/components/ui/button";
import { requestSetup } from "@/lib/setup";

/** Reopens the first-run setup sheet. */
export function RunSetupAgain() {
  return (
    <Field label="Setup" hint="Pick the data folder, e-reader and sources again, step by step.">
      <div>
        <Button onClick={requestSetup}>
          <Wand2 /> Run setup again
        </Button>
      </div>
    </Field>
  );
}
