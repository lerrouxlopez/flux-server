import { describe, expect, it, vi } from "vitest";
import { render, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { AdminPage } from "./AdminPage";
import { ExperienceContext, type ExperienceContextValue } from "../features/experience/ExperienceProvider";

const apiFetchMock = vi.fn(async (_path: string, _init?: any) => ({}));
vi.mock("../api/client", () => {
  return {
    apiFetch: (path: string, init?: any) => apiFetchMock(path, init),
  };
});

vi.mock("react-router-dom", async () => {
  const actual: any = await vi.importActual("react-router-dom");
  return { ...actual, useParams: () => ({ org_slug: "acme" }) };
});

function renderPage(ctx: Partial<ExperienceContextValue> = {}) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  const base: ExperienceContextValue = {
    rawMode: "work",
    label: "Work Mode",
    colorScheme: "dark",
    density: "comfortable",
    motion: "full",
    source: "test",
    notificationProfile: "all",
    mediaDefaults: {
      room_kind_preference: "meeting",
      join_intent: "video",
      auto_publish_audio: true,
      auto_publish_video: true,
      auto_publish_screen: false,
      auto_subscribe: true,
    },
    featureFlags: {},
    themeId: "teams",
    isOrgContext: true,
    userThemeId: "teams",
    setUserTheme: () => {},
    previewBranding: () => {},
    isLoading: false,
    error: null,
    setMode: () => {},
    clearModePreference: () => {},
    refetch: () => {},
  };

  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter>
        <ExperienceContext.Provider value={{ ...base, ...ctx }}>
          <AdminPage />
        </ExperienceContext.Provider>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("AdminPage theming", () => {
  it("does not force DEFAULT_THEME_ID on first branding load", async () => {
    apiFetchMock.mockImplementation(async (path: string) => {
      if (path === "/orgs") return { organizations: [{ id: "org-1", slug: "acme", name: "Acme", created_at: "" }] };
      if (path === "/orgs/org-1/branding")
        return {
          organization_id: "org-1",
          app_name: "Acme",
          theme: "dark",
          ui_mode: "work",
          ui_theme: "midnight",
          logo_url: null,
          primary_color: null,
          secondary_color: null,
          bg_color: null,
          surface_color: null,
          text_color: null,
          muted_color: null,
          border_color: null,
          selection_bg: null,
          selection_text: null,
          dropdown_bg: null,
          dropdown_text: null,
          chat_bubble_me_bg: null,
          chat_bubble_me_text: null,
          chat_bubble_other_bg: null,
          chat_bubble_other_text: null,
          updated_at: "2026-01-01T00:00:00Z",
        };
      if (path === "/orgs/org-1/members") return { members: [] };
      if (path === "/orgs/org-1/roles") return { roles: [] };
      return {};
    });

    const previewBranding = vi.fn();
    renderPage({ previewBranding });

    await waitFor(() => {
      expect(previewBranding).toHaveBeenCalledWith(expect.objectContaining({ ui_theme: "midnight" }));
    });
  });
});

