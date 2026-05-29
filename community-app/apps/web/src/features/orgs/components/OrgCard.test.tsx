import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import type { DiscoverOrg } from "../../../api/types";
import { OrgCard } from "./OrgCard";

describe("OrgCard", () => {
  it("shows name, slug, details, logo and type badge", () => {
    const org: DiscoverOrg = {
      id: "org-1",
      slug: "acme",
      name: "Acme Inc",
      description: "Org details here",
      avatar_url: "https://example.com/logo.png",
      banner_url: null,
      join_policy: "invite_only",
      category: null,
      tags: [],
      member_count: 12,
      online_count: 3,
      current_user_status: "not_member",
    };

    render(
      <OrgCard
        org={org}
        density="compact"
        onJoinOpen={vi.fn()}
        onJoinByInvite={vi.fn()}
        onRequestAccess={vi.fn()}
      />,
    );

    expect(screen.getByText("Acme Inc")).toBeInTheDocument();
    expect(screen.getByText("/acme")).toBeInTheDocument();
    expect(screen.getByText("Org details here")).toBeInTheDocument();
    expect(screen.getByAltText("Acme Inc logo")).toBeInTheDocument();
    expect(screen.getByText("Invite-only")).toBeInTheDocument();
  });
});

