import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./app";
import { ErrorBoundary } from "./components/error-boundary";
import { installGlobalErrorLogging } from "./lib/report";
import "./styles/app.css";

const root = document.getElementById("root");
if (root) {
  installGlobalErrorLogging();
  createRoot(root).render(
    <StrictMode>
      <ErrorBoundary>
        <App />
      </ErrorBoundary>
    </StrictMode>,
  );
}
