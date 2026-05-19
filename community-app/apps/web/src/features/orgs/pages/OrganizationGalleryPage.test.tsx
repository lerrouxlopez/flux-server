import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { OrganizationGalleryPage } from "./OrganizationGalleryPage";
import { ExperienceContext, type ExperienceContextValue } from "../../experience/ExperienceProvider";
import { useAuthStore } from "../../../state/auth";

const apiFetchMock = vi.fn(async (_path: string) => ({}));
vi.mock("../../../api/client", () => {
  return {
    apiFetch: (path: string, init?: any) => apiFetchMock(path, init),
  };
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
          <OrganizationGalleryPage />
        </ExperienceContext.Provider>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("OrganizationGalleryPage", () => {
  it("renders tabs and mode label", async () => {
    useAuthStore.setState({
      user: { id: "u1", email: "u1@example.com", display_name: "U1", created_at: new Date().toISOString() },
    });
    apiFetchMock.mockImplementation(async (path: string) => {
      if (path === "/orgs") return { organizations: [] };
      return {};
    });

    renderPage({ label: "Game Mode", density: "compact", rawMode: "play" });

    expect(screen.getByText("Organizations")).toBeInTheDocument();
    expect(screen.getByText("Game Mode")).toBeInTheDocument();
    expect(screen.getByText("My Organizations")).toBeInTheDocument();
    expect(screen.getByText("Discover")).toBeInTheDocument();
    expect(screen.getByText("Requests")).toBeInTheDocument();
    expect(screen.getByText("Invites")).toBeInTheDocument();
  });

  it("opens JoinByInvite modal from header action", async () => {
    useAuthStore.setState({
      user: { id: "u1", email: "u1@example.com", display_name: "U1", created_at: new Date().toISOString() },
    });
    apiFetchMock.mockImplementation(async (path: string) => {
      if (path === "/orgs") return { organizations: [] };
      return {};
    });

    const user = userEvent.setup();
    renderPage();

    await user.click(screen.getByText("Enter invite code"));
    expect(screen.getByText("Join by invite code")).toBeInTheDocument();
    expect(screen.getByLabelText("Org slug")).toBeInTheDocument();
    expect(screen.getByLabelText("Invite code")).toBeInTheDocument();
  });
});

