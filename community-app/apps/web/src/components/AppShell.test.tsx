import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { AppShell } from "./AppShell";
import { useAuthStore } from "../state/auth";

vi.mock("../api/client", () => {
  return {
    apiFetch: vi.fn(async (path: string) => {
      if (path === "/orgs") {
        return {
          organizations: [{ id: "org-1", slug: "acme", name: "Acme", created_at: "2026-01-01T00:00:00Z" }],
        };
      }
      if (path === "/orgs/org-1/members") {
        return {
          members: [
            {
              user_id: "me-1",
              email: "me@example.com",
              display_name: "Me",
              role: "admin",
              joined_at: "2026-01-01T00:00:00Z",
            },
          ],
        };
      }
      if (path === "/orgs/org-1/branding") {
        return { ui_theme: "dark" };
      }
      if (path.startsWith("/experience/context?")) {
        return {
          mode: "work",
          source: "server",
          density: "comfortable",
          motion: "full",
          notification_profile: "all",
          media_defaults: {
            room_kind_preference: "meeting",
            join_intent: "video",
            auto_publish_audio: true,
            auto_publish_video: true,
            auto_publish_screen: false,
            auto_subscribe: true,
          },
          feature_flags: {},
        };
      }
      return {};
    }),
  };
});

beforeEach(() => {
  useAuthStore.setState({
    user: {
      id: "me-1",
      email: "me@example.com",
      display_name: "Me",
      created_at: "2026-01-01T00:00:00Z",
    },
    loadMe: vi.fn(async () => {}),
    logout: vi.fn(async () => {}),
  } as any);
});

function renderApp(initialPath: string) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter initialEntries={[initialPath]}>
        <Routes>
          <Route element={<AppShell />}>
            <Route path="/app/:org_slug/channels/:channel_id" element={<div>Channel</div>} />
            <Route path="/profile" element={<div>Profile</div>} />
          </Route>
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("AppShell user menu", () => {
  it("shows Admin menu item for org admins", async () => {
    renderApp("/app/acme/channels/ch-1");

    fireEvent.click(screen.getByText("Me"));

    await waitFor(() => {
      expect(screen.getByRole("menuitem", { name: "Admin" })).toBeInTheDocument();
    });
  });

  it("closes the menu when clicking outside", async () => {
    renderApp("/app/acme/channels/ch-1");

    fireEvent.click(screen.getByText("Me"));
    await waitFor(() => {
      expect(screen.getByLabelText("User menu")).toBeInTheDocument();
    });

    fireEvent.mouseDown(document.body);

    await waitFor(() => {
      expect(screen.queryByLabelText("User menu")).not.toBeInTheDocument();
    });
  });
});
