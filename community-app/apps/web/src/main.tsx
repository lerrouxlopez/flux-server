import React from "react";
import ReactDOM from "react-dom/client";
import { RouterProvider } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import "./styles.css";
import { router } from "./router";
import { useBrandingStore } from "./state/branding";

const queryClient = new QueryClient();

async function bootstrap() {
  // Branding must load before login renders.
  const host = window.location.host;
  await useBrandingStore.getState().loadBranding(host);

  ReactDOM.createRoot(document.getElementById("root")!).render(
    <React.StrictMode>
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
        <ReactQueryDevtools initialIsOpen={false} />
      </QueryClientProvider>
    </React.StrictMode>,
  );
}

bootstrap();

