import { AlertTriangle, Copy, RotateCw } from "lucide-react";
import { Component, type ErrorInfo, type ReactNode, useState } from "react";
import { Button } from "@/components/ui/button";
import { copyText } from "@/lib/clipboard";
import { crashReport } from "@/lib/errors";
import { logUiError } from "@/lib/report";

type State = { error: Error | null; report: string };

/** App-root safety net: a render crash shows what happened instead of a blank window. */
export class ErrorBoundary extends Component<{ children: ReactNode }, State> {
  state: State = { error: null, report: "" };

  static getDerivedStateFromError(error: Error): Partial<State> {
    return { error, report: crashReport(error) };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    const report = crashReport(error, info.componentStack);
    this.setState({ report });
    logUiError(report);
  }

  render() {
    const { error, report } = this.state;
    return error ? <CrashScreen error={error} report={report} /> : this.props.children;
  }
}

function CrashScreen({ error, report }: { error: Error; report: string }) {
  const [copied, setCopied] = useState<"idle" | "done" | "failed">("idle");
  const copy = () =>
    copyText(report).then(
      () => setCopied("done"),
      () => setCopied("failed"),
    );
  return (
    <div role="alert" className="flex h-full items-center justify-center p-8">
      <div className="flex w-full max-w-lg flex-col gap-4">
        <AlertTriangle className="size-8 text-accent" />
        <div>
          <h1 className="text-subtitle font-semibold">Something went wrong</h1>
          <p className="mt-1 text-ink-2">
            The window hit an error it couldn't recover from. Copy the error into a bug report; it
            is in the log file too.
          </p>
        </div>
        <pre className="max-h-48 overflow-auto rounded-md border border-stroke bg-surface p-3 font-mono text-caption whitespace-pre-wrap text-ink-2">
          {error.message || error.name}
        </pre>
        <div className="flex gap-2">
          <Button variant="primary" onClick={copy}>
            <Copy /> {copied === "done" ? "Copied" : "Copy error"}
          </Button>
          <Button onClick={() => window.location.reload()}>
            <RotateCw /> Reload
          </Button>
        </div>
        {copied === "failed" && (
          <p className="text-caption text-accent">The clipboard is not available.</p>
        )}
      </div>
    </div>
  );
}
