import React from "react";
import ReactDOM from "react-dom/client";
import { RouterProvider } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import "./styles.css";
import { router } from "./router";
import { useBrandingStore } from "./state/branding";
import { useAuthStore } from "./state/auth";

const queryClient = new QueryClient();

const Devtools = import.meta.env.DEV
  ? React.lazy(async () => {
      const mod = await import("@tanstack/react-query-devtools");
      return { default: mod.ReactQueryDevtools };
    })
  : null;

async function bootstrap() {
  // Branding must load before login renders.
  const host = window.location.host;
  await useBrandingStore.getState().loadBranding(host);
  const branding = useBrandingStore.getState().branding;
  if (branding?.app_name) {
    document.title = branding.app_name;
  }

  // Restore session if tokens exist.
  useAuthStore.getState().hydrate();
  await useAuthStore.getState().loadMe().catch(() => {});

  ReactDOM.createRoot(document.getElementById("root")!).render(
    <React.StrictMode>
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
        {Devtools ? (
          <React.Suspense fallback={null}>
            <Devtools initialIsOpen={false} />
          </React.Suspense>
        ) : null}
      </QueryClientProvider>
    </React.StrictMode>,
  );
}

bootstrap();
