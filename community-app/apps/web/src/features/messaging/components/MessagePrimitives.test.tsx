import { describe, expect, it, vi } from "vitest";
import { render } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import type { ReactElement } from "react";
import { MessageViewport } from "./MessageViewport";

vi.mock("../../../api/client", () => {
  return {
    apiFetch: vi.fn(async () => ({})),
  };
});

function renderWithProviders(ui: ReactElement) {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <MemoryRouter>{ui}</MemoryRouter>
    </QueryClientProvider>,
  );
}

function fakeEngine(overrides: Partial<any> = {}) {
  const msg = {
    id: "m-1",
    organization_id: "org-1",
    channel_id: "ch-1",
    sender_id: "u-2",
    body: "hello",
    kind: "text",
    created_at: new Date().toISOString(),
    reactions: [{ emoji: "👍", count: 2, reacted_by_me: false }],
    attachments: [
      {
        id: "a-1",
        filename: "file.txt",
        content_type: "text/plain",
        size_bytes: 1,
        download_url: "https://example.com/file.txt",
        created_at: new Date().toISOString(),
      },
    ],
  };

  const base: any = {
    meId: "u-1",
    memberById: new Map([["u-2", { user_id: "u-2", display_name: "Other" }]]),
    presenceByUser: { "u-2": "online" },
    pinnedIds: new Set<string>(),
    reactionPickerFor: null,
    setReactionPickerFor: () => {},
    reactionPickerRef: { current: null },
    QUICK_REACTIONS: ["👍", "❤️"],
    addReaction: { mutate: () => {} },
    removeReaction: { mutate: () => {} },
    pin: { mutate: () => {} },
    unpin: { mutate: () => {} },
    setWorkPane: () => {},
    setActiveThreadId: () => {},
    createThread: { mutate: () => {} },

    messages: { isLoading: false, isError: false },
    visibleMessages: [msg],
    bottomRef: { current: null },
  };

  return { ...base, ...overrides };
}

describe("Messaging primitives", () => {
  it("renders message viewport (snapshot)", () => {
    const e = fakeEngine();
    const { container } = renderWithProviders(
      <MessageViewport e={e} density="comfortable" panelMode="expanded" className="p-2" />,
    );
    expect(container).toMatchSnapshot();
  });
});

