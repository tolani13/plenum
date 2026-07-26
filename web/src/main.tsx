import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { QueryClientProvider } from "@tanstack/react-query";
import { BrowserRouter } from "react-router";

// Fonts ship via npm — zero runtime CDN (demo-room network risk, resolution 11).
import "@fontsource/barlow-condensed/500.css";
import "@fontsource/barlow-condensed/600.css";
import "@fontsource/inter/400.css";
import "@fontsource/inter/500.css";
import "@fontsource/inter/600.css";
import "./styles/tokens.css";

import { queryClient } from "./lib/queryClient";
import { ErrorBoundary, describeRootError } from "./components/ErrorBoundary";
import { RenderErrorProbe } from "./components/RenderErrorProbe";
import { ROOT_RENDER_ERROR_EVENT } from "./lib/events";
import { App } from "./App";

const root = document.getElementById("root");
if (!root) throw new Error("#root missing");

// D-3 (2026-07-26): the outermost door. The screen boundary inside the Shell
// catches everything that renders under the nav; this one catches what is
// ABOVE it — the Shell itself, RequireAuth, Login, the router, the providers.
// Nothing renders outside it, so an empty document is no longer reachable.
createRoot(root).render(
  <StrictMode>
    <ErrorBoundary
      region="app root"
      className="min-h-screen bg-bg p-6 text-text"
      describe={describeRootError}
    >
      <RenderErrorProbe event={ROOT_RENDER_ERROR_EVENT} />
      <QueryClientProvider client={queryClient}>
        <BrowserRouter>
          <App />
        </BrowserRouter>
      </QueryClientProvider>
    </ErrorBoundary>
  </StrictMode>,
);
