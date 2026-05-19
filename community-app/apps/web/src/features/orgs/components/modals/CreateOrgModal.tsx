import { useMemo, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { apiFetch } from "../../../../api/client";
import type { CreateOrgRequest, Org } from "../../../../api/types";
import { Modal } from "../../../../components/Modal";
import { Input } from "../../../../components/Input";
import { Button } from "../../../../components/Button";

export function CreateOrgModal(props: { open: boolean; onClose: () => void; onCreated?: (org: Org) => void }) {
  const qc = useQueryClient();
  const [name, setName] = useState("");
  const [slug, setSlug] = useState("");
  const [err, setErr] = useState<string | null>(null);

  const canSubmit = useMemo(() => name.trim().length > 0 && slug.trim().length > 0, [name, slug]);

  const create = useMutation({
    mutationFn: async (req: CreateOrgRequest) =>
      apiFetch<Org>("/orgs", { method: "POST", body: JSON.stringify(req) }),
    onSuccess: async (org) => {
      await qc.invalidateQueries({ queryKey: ["orgs"] });
      setErr(null);
      setName("");
      setSlug("");
      props.onClose();
      props.onCreated?.(org);
    },
    onError: (e) => setErr((e as Error).message),
  });

  return (
    <Modal open={props.open} onClose={props.onClose} title="Create organization">
      <form
        className="grid gap-3"
        onSubmit={(e) => {
          e.preventDefault();
          setErr(null);
          create.mutate({ name, slug });
        }}
      >
        <div>
          <label className="mb-1 block text-sm text-slate-300" htmlFor="org-name">
            Name
          </label>
          <Input id="org-name" value={name} onChange={(e) => setName(e.target.value)} placeholder="Acme" />
        </div>
        <div>
          <label className="mb-1 block text-sm text-slate-300" htmlFor="org-slug">
            Slug
          </label>
          <Input id="org-slug" value={slug} onChange={(e) => setSlug(e.target.value)} placeholder="acme" />
        </div>

        {err ? <div className="text-sm text-red-400">{err}</div> : null}

        <div className="flex flex-wrap items-center justify-end gap-2">
          <Button className="bg-slate-800 hover:bg-slate-700" onClick={props.onClose} type="button">
            Cancel
          </Button>
          <Button className="flux-btn-primary" disabled={!canSubmit || create.isPending} type="submit">
            {create.isPending ? "Creatingâ€¦" : "Create"}
          </Button>
        </div>
      </form>
    </Modal>
  );
}

