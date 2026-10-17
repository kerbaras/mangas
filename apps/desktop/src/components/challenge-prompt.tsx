import { Link2 } from "lucide-react";
import { EmptyState } from "@/components/empty-state";
import { Button } from "@/components/ui/button";
import { connectThen } from "@/lib/notify";

type Props = { source: string; onConnected: () => void };

/** Inline offer to pass a site's browser check, shown where the blocked content would be. */
export function ChallengePrompt({ source, onConnected }: Props) {
  return (
    <EmptyState
      icon={<Link2 />}
      title="This site asks for a browser check"
      action={
        <Button variant="primary" onClick={() => void connectThen(source, onConnected)}>
          <Link2 /> Connect {source}
        </Button>
      }
    >
      Pass it once in its own window; the app keeps the session and retries.
    </EmptyState>
  );
}
