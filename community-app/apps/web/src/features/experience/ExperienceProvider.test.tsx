import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ExperienceProvider } from "./ExperienceProvider";
import { useExperience } from "./useExperience";
import { useAuthStore } from "../../state/auth";

const apiFetchMock = vi.fn(async (_path: string) => ({}));
vi.mock("../../api/client", () => {
  return {
    apiFetch: (path: string, init?: any) => apiFetchMock(path, init),
  };
});

function ShowExperience() {
  const e = useExperience();
  return (
    <div>
      <div data-testid="label">{e.label}</div>
      <div data-testid="density">{e.density}</div>
      <div data-testid="scheme">{e.colorScheme}</div>
      <button onClick={() => e.setMode("play")} type="button">
        set-play
      </button>
    </div>
  );
}

function renderWithProviders(ui: React.ReactNode) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(<QueryClientProvider client={qc}>{ui}</QueryClientProvider>);
}

beforeEach(() => {
  apiFetchMock.mockReset();
  localStorage.clear();
  useAuthStore.setState({ user: null, accessToken: null, refreshToken: null });
});

describe("ExperienceProvider", () => {
  it("maps backend mode=play to Game Mode label", async () => {
    useAuthStore.setState({
      user: { id: "u1", email: "u1@example.com", display_name: "U1", created_at: new Date().toISOString() },
    });

    apiFetchMock.mockImplementation(async (path: string) => {
      if (path.startsWith("/experience/context")) {
        return {
          mode: "play",
          source: "user_preference",
          density: "compact",
          motion: "reduced",
          notification_profile: "minimal",
          media_defaults: {
            room_kind_preference: "voice",
            join_intent: "voice_only",
            auto_publish_audio: true,
            auto_publish_video: false,
            auto_publish_screen: false,
            auto_subscribe: true,
          },
          feature_flags: {},
        };
      }
      return {};
    });

    const { container } = renderWithProviders(
      <ExperienceProvider orgId="org-1" channelId="ch-1">
        <ShowExperience />
      </ExperienceProvider>,
    );

    await waitFor(() => expect(screen.getByTestId("label")).toHaveTextContent("Game Mode"));
    expect(screen.getByTestId("density")).toHaveTextContent("compact");
    expect(screen.getByTestId("scheme")).toHaveTextContent("dark");
    expect(container).toMatchSnapshot();
  });

  it("uses local preference when no orgId is active", async () => {
    localStorage.setItem("flux_experience_mode_preference", "play");

    renderWithProviders(
      <ExperienceProvider orgId={null} channelId={null}>
        <ShowExperience />
      </ExperienceProvider>,
    );

    expect(screen.getByTestId("label")).toHaveTextContent("Game Mode");
    expect(screen.getByTestId("scheme")).toHaveTextContent("dark");
    expect(apiFetchMock).not.toHaveBeenCalled();
  });

  it("persists user preference when setMode is used", async () => {
    useAuthStore.setState({
      user: { id: "u1", email: "u1@example.com", display_name: "U1", created_at: new Date().toISOString() },
    });

    apiFetchMock.mockImplementation(async (path: string, init?: any) => {
      if (path === "/experience/preferences" && init?.method === "PATCH") {
        return { status: "ok", mode_preference: "play" };
      }
      return {};
    });

    const user = userEvent.setup();
    renderWithProviders(
      <ExperienceProvider orgId={null} channelId={null}>
        <ShowExperience />
      </ExperienceProvider>,
    );

    await user.click(screen.getByText("set-play"));

    expect(localStorage.getItem("flux_experience_mode_preference")).toBe("play");
    await waitFor(() => {
      expect(apiFetchMock).toHaveBeenCalledWith(
        "/experience/preferences",
        expect.objectContaining({ method: "PATCH" }),
      );
    });
  });
});
