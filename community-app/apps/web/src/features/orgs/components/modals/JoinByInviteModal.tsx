import { useEffect, useMemo, useState } from "react";
import { Modal } from "../../../../components/Modal";
import { Input } from "../../../../components/Input";
import { Button } from "../../../../components/Button";
import { useJoinByInvite } from "../../hooks/useJoinByInvite";

export function JoinByInviteModal(props: {
  open: boolean;
  initialSlug?: string | null;
  onClose: () => void;
  onJoined: (slug: string) => void;
}) {
  const [slug, setSlug] = useState(props.initialSlug ?? "");
  const [inviteCode, setInviteCode] = useState("");
  const [err, setErr] = useState<string | null>(null);
  const join = useJoinByInvite();

  useEffect(() => {
    if (!props.open) return;
    setSlug(props.initialSlug ?? "");
    setInviteCode("");
    setErr(null);
  }, [props.open, props.initialSlug]);

  const canSubmit = useMemo(() => slug.trim().length > 0 && inviteCode.trim().length > 0, [slug, inviteCode]);

  return (
    <Modal open={props.open} onClose={props.onClose} title="Join by invite code">
      <form
        className="grid gap-3"
        onSubmit={async (e) => {
          e.preventDefault();
          setErr(null);
          join.mutate(
            { slug: slug.trim(), inviteCode: inviteCode.trim() },
            {
              onSuccess: (r) => {
                props.onClose();
                props.onJoined(r.slug);
              },
              onError: (e2) => setErr((e2 as Error).message),
            },
          );
        }}
      >
        <div>
          <label className="mb-1 block text-sm text-slate-300" htmlFor="join-slug">
            Org slug
          </label>
          <Input id="join-slug" value={slug} onChange={(e) => setSlug(e.target.value)} placeholder="acme" />
        </div>
        <div>
          <label className="mb-1 block text-sm text-slate-300" htmlFor="join-code">
            Invite code
          </label>
          <Input
            id="join-code"
            value={inviteCode}
            onChange={(e) => setInviteCode(e.target.value)}
            placeholder="paste code"
            autoComplete="off"
          />
        </div>

        {err ? <div className="text-sm text-red-400">{err}</div> : null}

        <div className="flex flex-wrap items-center justify-end gap-2">
          <Button className="bg-slate-800 hover:bg-slate-700" onClick={props.onClose} type="button">
            Cancel
          </Button>
          <Button className="flux-btn-primary" disabled={!canSubmit || join.isPending} type="submit">
            {join.isPending ? "Joiningâ€¦" : "Join"}
          </Button>
        </div>
      </form>
    </Modal>
  );
}

