import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { apiFetch } from "../../../api/client";
import type { DiscoverOrgsResponse, JoinPolicy } from "../../../api/types";

export function useDiscoverOrganizations(params: {
  enabled: boolean;
  query: string;
  policy: JoinPolicy | "any";
}) {
  const q = params.query.trim();
  const policy = params.policy === "any" ? "" : params.policy;

  return useQuery({
    enabled: params.enabled,
    queryKey: ["orgs", "discover", q, policy],
    queryFn: () => {
      const qp = new URLSearchParams();
      if (q) qp.set("q", q);
      if (policy) qp.set("policy", policy);
      return apiFetch<DiscoverOrgsResponse>(`/orgs/discover?${qp.toString()}`);
    },
    placeholderData: keepPreviousData,
    staleTime: 10_000,
    retry: false,
  });
}

