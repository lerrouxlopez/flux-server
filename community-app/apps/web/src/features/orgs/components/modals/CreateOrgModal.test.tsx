import { describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { CreateOrgModal } from "./CreateOrgModal";

const apiFetchMock = vi.fn(async (_path: string, _init?: any) => ({}));
vi.mock("../../../../api/client", () => {
  return {
    apiFetch: (path: string, init?: any) => apiFetchMock(path, init),
  };
});

function renderModal() {
  const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return render(
    <QueryClientProvider client={qc}>
      <CreateOrgModal open={true} onClose={() => {}} />
    </QueryClientProvider>,
  );
}

describe("CreateOrgModal", () => {
  it("auto-fills slug from name and persists join type/details/logo", async () => {
    apiFetchMock.mockImplementation(async (path: string, init?: any) => {
      if (path === "/orgs" && init?.method === "POST") {
        return { id: "org-1", slug: "my-org", name: "My Org", created_at: "2026-01-01T00:00:00Z" };
      }
      if (path === "/orgs/org-1/discovery-settings" && init?.method === "PATCH") {
        return { status: "ok" };
      }
      return {};
    });

    const user = userEvent.setup();
    renderModal();

    await user.type(screen.getByLabelText("Name"), "My Org");

    await waitFor(() => {
      expect((screen.getByLabelText("Slug") as HTMLInputElement).value).toBe("my-org");
    });

    await user.type(screen.getByLabelText("Org details"), "Hello world");
    await user.type(screen.getByLabelText("Org logo URL"), "https://example.com/logo.png");
    await user.selectOptions(screen.getByLabelText("Type"), "open");

    await user.click(screen.getByRole("button", { name: "Create" }));

    await waitFor(() => {
      expect(apiFetchMock).toHaveBeenCalledWith(
        "/orgs/org-1/discovery-settings",
        expect.objectContaining({ method: "PATCH" }),
      );
    });

    const patchCall = apiFetchMock.mock.calls.find((c) => c[0] === "/orgs/org-1/discovery-settings");
    const patchBody = JSON.parse(String(patchCall?.[1]?.body ?? "{}"));
    expect(patchBody).toEqual(
      expect.objectContaining({
        join_policy: "open",
        discoverable: true,
        description: "Hello world",
        avatar_url: "https://example.com/logo.png",
      }),
    );
  });
});
