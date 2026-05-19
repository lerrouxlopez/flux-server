import { useMutation, useQueryClient } from "@tanstack/react-query";
import { apiFetch } from "../../../api/client";

export function useJoinByInvite() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: async (input: { slug: string; inviteCode: string }) => {
      return apiFetch<{ status: string; organization_id: string; slug: string }>("/orgs/join", {
        method: "POST",
        body: JSON.stringify({ slug: input.slug, invite_code: input.inviteCode }),
      });
    },
    onSuccess: async () => {
      await qc.invalidateQueries({ queryKey: ["orgs"] });
      await qc.invalidateQueries({ queryKey: ["orgs", "discover"] });
    },
  });
}

