import { useEffect, useState } from "react";
import { emailRefusal } from "@/lib/ipc.send";

/**
 * Why Send to Kindle by email won't take `file`, as core answers it (`email_refusal`), or null
 * when it will. Null until core answers, or if asking fails: sending checks the format again.
 */
export function useEmailRefusal(file: string): string | null {
  const [answer, setAnswer] = useState<{ file: string; refusal: string | null } | null>(null);
  useEffect(() => {
    let live = true;
    emailRefusal(file).then(
      (refusal) => live && setAnswer({ file, refusal }),
      () => live && setAnswer({ file, refusal: null }),
    );
    return () => {
      live = false;
    };
  }, [file]);
  return answer?.file === file ? answer.refusal : null;
}
