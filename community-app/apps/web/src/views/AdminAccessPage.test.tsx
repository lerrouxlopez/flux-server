import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { AdminAccessPage } from "./AdminAccessPage";
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
    colorScheme: "light",
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
    previewBranding: () => {},
    themeId: "default",
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
          <AdminAccessPage />
        </ExperienceContext.Provider>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("AdminAccessPage", () => {
  it("renders discovery settings form", async () => {
    apiFetchMock.mockImplementation(async (path: string) => {
      if (path === "/orgs") return { organizations: [{ id: "org-1", slug: "acme", name: "Acme", created_at: "" }] };
      if (path === "/orgs/org-1/discovery-settings")
        return {
          discoverable: false,
          join_policy: "invite_only",
          description: null,
          avatar_url: null,
          banner_url: null,
          member_count_visible: true,
          online_count_visible: false,
          category: null,
          tags: [],
        };
      if (path === "/orgs/org-1/join-requests") return { requests: [] };
      return {};
    });

    renderPage();

    expect(await screen.findByText("Discovery settings")).toBeInTheDocument();
    expect(await screen.findByLabelText("Join policy")).toBeInTheDocument();
    expect(screen.getByText("Public gallery preview")).toBeInTheDocument();
    expect(screen.getByText("Join requests")).toBeInTheDocument();
  });

  it("calls PATCH discovery-settings on save", async () => {
    apiFetchMock.mockImplementation(async (path: string, init?: any) => {
      if (path === "/orgs") return { organizations: [{ id: "org-1", slug: "acme", name: "Acme", created_at: "" }] };
      if (path === "/orgs/org-1/discovery-settings" && (!init || init.method === "GET"))
        return {
          discoverable: false,
          join_policy: "invite_only",
          description: null,
          avatar_url: null,
          banner_url: null,
          member_count_visible: true,
          online_count_visible: false,
          category: null,
          tags: [],
        };
      if (path === "/orgs/org-1/join-requests") return { requests: [] };
      if (path === "/orgs/org-1/discovery-settings" && init?.method === "PATCH") return { status: "ok" };
      return {};
    });

    const user = userEvent.setup();
    renderPage();

    await screen.findByText("Discovery settings");
    await screen.findByText("Discoverable in public gallery");
    await user.click(screen.getByText("Discoverable in public gallery"));
    await user.click(screen.getByText("Save changes"));

    expect(apiFetchMock).toHaveBeenCalledWith(
      "/orgs/org-1/discovery-settings",
      expect.objectContaining({ method: "PATCH" }),
    );
  });
});
