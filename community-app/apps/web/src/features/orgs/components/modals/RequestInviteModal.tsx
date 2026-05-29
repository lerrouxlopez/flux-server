import { useEffect, useMemo, useState } from "react";
import { Modal } from "../../../../components/Modal";
import { Button } from "../../../../components/Button";
import { TextArea } from "../../../../components/TextArea";
import { useRequestOrganizationInvite } from "../../hooks/useRequestOrganizationInvite";

export function RequestInviteModal(props: {
  open: boolean;
  orgId: string | null;
  orgName: string | null;
  onClose: () => void;
}) {
  const [message, setMessage] = useState("");
  const [err, setErr] = useState<string | null>(null);
  const req = useRequestOrganizationInvite();

  useEffect(() => {
    if (!props.open) return;
    setMessage("");
    setErr(null);
  }, [props.open, props.orgId]);

  const canSubmit = useMemo(() => !!props.orgId && message.trim().length > 0, [props.orgId, message]);

  return (
    <Modal open={props.open} onClose={props.onClose} title={`Request access${props.orgName ? `: ${props.orgName}` : ""}`}>
      <form
        className="grid gap-3"
        onSubmit={(e) => {
          e.preventDefault();
          setErr(null);
          if (!props.orgId) return;
          req.mutate(
            { orgId: props.orgId, message: message.trim() },
            {
              onSuccess: () => props.onClose(),
              onError: (e2) => setErr((e2 as Error).message),
            },
          );
        }}
      >
        <div>
          <label className="mb-1 block text-sm text-slate-300" htmlFor="request-message">
            Message
          </label>
          <TextArea
            id="request-message"
            value={message}
            onChange={(e) => setMessage(e.target.value)}
            placeholder="Tell the admins why you want to join"
            rows={4}
          />
          <div className="mt-1 text-xs text-slate-500">Sent to organization admins for approval.</div>
        </div>

        {err ? <div className="text-sm text-red-400">{err}</div> : null}

        <div className="flex flex-wrap items-center justify-end gap-2">
          <Button className="bg-slate-800 hover:bg-slate-700" onClick={props.onClose} type="button">
            Cancel
          </Button>
          <Button className="flux-btn-primary" disabled={!canSubmit || req.isPending} type="submit">
            {req.isPending ? "Requestingâ€¦" : "Request"}
          </Button>
        </div>
      </form>
    </Modal>
  );
}

