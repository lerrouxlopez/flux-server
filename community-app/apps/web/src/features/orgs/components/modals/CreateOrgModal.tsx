import { useEffect, useMemo, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { apiFetch } from "../../../../api/client";
import type { CreateOrgRequest, Org, PatchDiscoverySettingsRequest } from "../../../../api/types";
import { Modal } from "../../../../components/Modal";
import { Input } from "../../../../components/Input";
import { Button } from "../../../../components/Button";
import { TextArea } from "../../../../components/TextArea";

type OrgJoinType = "invite_only" | "request" | "open";

function slugifyOrgName(name: string): string {
  return name
    .trim()
    .toLowerCase()
    .replace(/['"]/g, "")
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-+/, "")
    .replace(/-+$/, "")
    .slice(0, 48);
}

export function CreateOrgModal(props: { open: boolean; onClose: () => void; onCreated?: (org: Org) => void }) {
  const qc = useQueryClient();
  const [name, setName] = useState("");
  const [slug, setSlug] = useState("");
  const [slugTouched, setSlugTouched] = useState(false);
  const [details, setDetails] = useState("");
  const [logoUrl, setLogoUrl] = useState("");
  const [joinType, setJoinType] = useState<OrgJoinType>("invite_only");
  const [err, setErr] = useState<string | null>(null);

  const canSubmit = useMemo(() => name.trim().length > 0 && slug.trim().length > 0, [name, slug]);

  useEffect(() => {
    if (slugTouched) return;
    const next = slugifyOrgName(name);
    setSlug(next);
  }, [name, slugTouched]);

  const create = useMutation({
    mutationFn: async (req: CreateOrgRequest) =>
      apiFetch<Org>("/orgs", { method: "POST", body: JSON.stringify(req) }),
    onSuccess: async (org) => {
      await qc.invalidateQueries({ queryKey: ["orgs"] });

      const patch: PatchDiscoverySettingsRequest = {
        join_policy: joinType,
        discoverable: joinType !== "invite_only",
      };
      if (details.trim()) patch.description = details.trim();
      if (logoUrl.trim()) patch.avatar_url = logoUrl.trim();

      await apiFetch<{ status: string }>(`/orgs/${org.id}/discovery-settings`, {
        method: "PATCH",
        body: JSON.stringify(patch),
      }).catch(() => {});

      setErr(null);
      setName("");
      setSlug("");
      setSlugTouched(false);
      setDetails("");
      setLogoUrl("");
      setJoinType("invite_only");
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
          <label className="mb-1 block text-sm text-slate-300" htmlFor="org-details">
            Org details
          </label>
          <TextArea
            id="org-details"
            value={details}
            onChange={(e) => setDetails(e.target.value)}
            placeholder="What is this org about?"
          />
        </div>
        <div>
          <label className="mb-1 block text-sm text-slate-300" htmlFor="org-slug">
            Slug
          </label>
          <Input
            id="org-slug"
            value={slug}
            onChange={(e) => {
              setSlugTouched(true);
              setSlug(e.target.value);
            }}
            placeholder="acme"
          />
          <div className="mt-1 text-xs text-slate-500">Lowercase letters, numbers, and dashes only.</div>
        </div>
        <div>
          <label className="mb-1 block text-sm text-slate-300" htmlFor="org-logo">
            Org picture / logo (URL)
          </label>
          <Input
            id="org-logo"
            value={logoUrl}
            onChange={(e) => setLogoUrl(e.target.value)}
            placeholder="https://..."
          />
          {logoUrl.trim() ? (
            <div className="mt-2 flex items-center gap-3">
              <img
                alt="Org logo preview"
                className="h-10 w-10 rounded-md border border-slate-800 bg-slate-900 object-cover"
                src={logoUrl.trim()}
              />
              <div className="text-xs text-slate-500">Preview</div>
            </div>
          ) : null}
        </div>
        <div>
          <label className="mb-1 block text-sm text-slate-300" htmlFor="org-type">
            Type
          </label>
          <select
            id="org-type"
            className="w-full rounded-md border border-slate-800 bg-slate-900 px-3 py-2 text-sm text-slate-200 outline-none focus:border-[color:var(--flux-focus-border)]"
            value={joinType}
            onChange={(e) => setJoinType(e.target.value as OrgJoinType)}
          >
            <option value="invite_only">Invite only</option>
            <option value="request">Private (request access)</option>
            <option value="open">Public (anyone can join)</option>
          </select>
          <div className="mt-1 text-xs text-slate-500">
            Invite-only: admins share invite codes. Private: users request to join. Public: anyone can join.
          </div>
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
