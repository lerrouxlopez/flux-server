import React from "react";
import ReactDOM from "react-dom/client";
import { RouterProvider } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import "./styles.css";
import { router } from "./router";
import { applyBrandingToDom, useBrandingStore } from "./state/branding";
import { useAuthStore } from "./state/auth";
import { LS_USER_THEME } from "./state/userTheme";
import { DEFAULT_THEME_ID } from "./branding/presets";

const queryClient = new QueryClient();

async function bootstrap() {
  // Branding must load before login renders.
  const host = window.location.host;
  await useBrandingStore.getState().loadBranding(host);
  const branding = useBrandingStore.getState().branding;
  if (branding?.app_name) {
    document.title = branding.app_name;
  }

  // Apply initial theme from frame 0.
  // Host-specific branding (white-label) takes precedence; otherwise use the user's personal theme.
  if (branding) {
    applyBrandingToDom(branding);
  } else {
    const userThemeId = localStorage.getItem(LS_USER_THEME) ?? DEFAULT_THEME_ID;
    applyBrandingToDom(null, { themeId: userThemeId });
  }

  // Restore session if tokens exist.
  useAuthStore.getState().hydrate();
  await useAuthStore.getState().loadMe().catch(() => {});

  ReactDOM.createRoot(document.getElementById("root")!).render(
    <React.StrictMode>
      <QueryClientProvider client={queryClient}>
        <RouterProvider router={router} />
      </QueryClientProvider>
    </React.StrictMode>,
  );
}

bootstrap();
