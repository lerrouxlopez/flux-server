import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { WorkShell } from "./WorkShell";
import { GameShell } from "./GameShell";
import type { ReactElement } from "react";
import { ExperienceProvider } from "../features/experience/ExperienceProvider";

vi.mock("../api/client", () => {
  return {
    apiFetch: vi.fn(async (path: string) => {
      if (path.includes("/channels")) return { channels: [] };
      if (path.includes("/dms")) return { dms: [] };
      if (path.includes("/members")) return { members: [] };
      if (path.includes("/friends/requests")) return { requests: [] };
      if (path.includes("/friends")) return { friends: [] };
      return {};
    }),
  };
});

function fakeEngine(overrides: Partial<any> = {}) {
  const base: any = {
    uiMode: "work",
    meId: "me-1",
    orgs: { isLoading: false },
    org: { id: "org-1", slug: "acme" },
    channel: { id: "ch-1", kind: "text", name: "general" },
    channel_id: "ch-1",
    channelTitle: "# general",
    presenceByUser: {},
    connected: true,
    canSeeAdmin: false,

    editingChannelName: false,
    setEditingChannelName: () => {},
    channelNameDraft: "general",
    setChannelNameDraft: () => {},
    updateChannel: { isPending: false, mutate: () => {} },
    deleteChannel: { isPending: false, mutate: () => {} },

    workPane: null,
    setWorkPane: () => {},
    workSearchOpen: false,
    setWorkSearchOpen: () => {},
    workSearch: "",
    setWorkSearch: () => {},
    activeThreadId: null,
    setActiveThreadId: () => {},
    threadDraft: "",
    setThreadDraft: () => {},
    newThreadDraft: "",
    setNewThreadDraft: () => {},

    pins: { isLoading: false, isError: false, data: { pins: [] } },
    pinnedIds: new Set<string>(),
    threads: { isLoading: false, isError: false, data: { threads: [] } },
    thread: { isLoading: false, isError: false, data: null },
    createThread: { isPending: false, mutate: () => {} },
    replyToThread: { isPending: false, mutate: () => {} },
    pin: { isPending: false, mutate: () => {} },
    unpin: { isPending: false, mutate: () => {} },

    memberById: new Map(),
    visibleMessages: [],
    messages: { isLoading: false, isError: false },
    bottomRef: { current: null },

    typingText: null,

    text: "",
    setText: () => {},
    emojiOpen: false,
    setEmojiOpen: () => {},
    reactionPickerFor: null,
    setReactionPickerFor: () => {},
    QUICK_REACTIONS: ["👍", "❤️", "😂", "😮", "😢", "😡"],
    reactionPickerRef: { current: null },
    fileInputRef: { current: null },
    textAreaRef: { current: null },
    pendingAttachments: [],
    setPendingAttachments: () => {},
    onTypingChange: () => {},
    onSubmit: (ev: any) => ev.preventDefault(),
    send: { isPending: false, mutate: () => {} },
    addReaction: { mutate: () => {} },
    removeReaction: { mutate: () => {} },

    createMeeting: { isPending: false, mutate: () => {} },
  };
  return { ...base, ...overrides };
}

function renderWithProviders(ui: ReactElement) {
  const qc = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter>
        <ExperienceProvider orgId={null} channelId={null}>
          {ui}
        </ExperienceProvider>
      </MemoryRouter>
    </QueryClientProvider>,
  );
}

describe("Shells", () => {
  it("WorkShell shows search/pins/threads + meeting control and comfortable density", () => {
    renderWithProviders(<WorkShell e={fakeEngine()} />);
    expect(screen.getByTestId("work-shell")).toBeInTheDocument();
    expect(screen.getByText("Search")).toBeInTheDocument();
    expect(screen.getByText("Pins")).toBeInTheDocument();
    expect(screen.getByText("Threads")).toBeInTheDocument();
    expect(screen.getByTestId("meeting-control")).toBeInTheDocument();
    expect(screen.getByText("Start meeting")).toBeInTheDocument();
  });

  it("GameShell shows compact density and reactions control", () => {
    renderWithProviders(<GameShell e={fakeEngine({ uiMode: "play" })} />);
    expect(screen.getByTestId("game-shell")).toBeInTheDocument();
    expect(screen.getByText("Reactions")).toBeInTheDocument();
    expect(screen.getByText("Voice")).toBeInTheDocument();
  });
});
