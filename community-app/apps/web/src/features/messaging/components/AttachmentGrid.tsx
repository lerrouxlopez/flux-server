import type { MessageAttachment } from "../../../api/types";

export function AttachmentGrid(props: { attachments: MessageAttachment[]; density: "comfortable" | "compact" }) {
  const atts = props.attachments ?? [];
  if (!atts.length) return null;

  return (
    <div className={`space-y-2 ${props.density === "compact" ? "" : ""}`} aria-label="Attachments">
      {atts.map((a) => {
        const isImage = (a.content_type ?? "").startsWith("image/");
        return (
          <div key={a.id} className="rounded-xl border border-white/10 bg-black/10 p-2">
            {isImage ? (
              <img alt={a.filename} className="max-h-64 w-auto rounded-lg" src={a.download_url} />
            ) : (
              <a className="flux-link text-sm" download={a.filename} href={a.download_url}>
                {a.filename}
              </a>
            )}
          </div>
        );
      })}
    </div>
  );
}

