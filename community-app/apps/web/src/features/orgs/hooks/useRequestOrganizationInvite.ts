import { useMutation, useQueryClient } from "@tanstack/react-query";
import { apiFetch } from "../../../api/client";

export function useRequestOrganizationInvite() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: { orgId: string; message: string }) => {
      return apiFetch<{ status: string; request_id?: string }>(`/orgs/${input.orgId}/join-requests`, {
        method: "POST",
        body: JSON.stringify({ message: input.message }),
      });
    },
    onSuccess: async () => {
      await qc.invalidateQueries({ queryKey: ["orgs", "discover"] });
    },
  });
}

