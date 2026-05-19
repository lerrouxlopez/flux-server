import { useQuery } from "@tanstack/react-query";
import { apiFetch } from "../../../api/client";
import type { OrgsListResponse } from "../../../api/types";

export function useMyOrganizations(enabled: boolean) {
  return useQuery({
    enabled,
    queryKey: ["orgs"],
    queryFn: () => apiFetch<OrgsListResponse>("/orgs"),
    staleTime: 30_000,
    retry: false,
  });
}

