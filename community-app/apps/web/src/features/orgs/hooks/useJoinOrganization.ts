import { useMutation, useQueryClient } from "@tanstack/react-query";
import { apiFetch } from "../../../api/client";

export function useJoinOrganization() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: { orgId: string }) => {
      return apiFetch<{ status: string; slug?: string }>(`/orgs/${input.orgId}/join`, { method: "POST", body: "{}" });
    },
    onSuccess: async () => {
      await qc.invalidateQueries({ queryKey: ["orgs"] });
      await qc.invalidateQueries({ queryKey: ["orgs", "discover"] });
    },
  });
}

