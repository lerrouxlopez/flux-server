import { TextArea } from "../../../components/TextArea";
import type { ChannelEngine } from "../../../engines/useChannelEngine";

export function Composer(props: { e: ChannelEngine; density: "comfortable" | "compact"; className?: string }) {
  const { e } = props;
  return (
    <form className={props.className ?? ""} onSubmit={e.onSubmit} aria-label="Message composer">
      <div className="flux-focus-within flex items-end gap-2 rounded-xl border border-slate-800 bg-slate-950/30 px-3 py-2">
        <TextArea
          ref={e.textAreaRef}
          rows={1}
          value={e.text}
          placeholder="Type a message"
          onChange={(ev) => e.onTypingChange(ev.target.value)}
          onKeyDown={(ev) => {
            if (ev.key === "Enter" && !ev.shiftKey) {
              ev.preventDefault();
              (ev.currentTarget.form as HTMLFormElement | null)?.requestSubmit();
            }
          }}
          onInput={(ev) => {
            const el = ev.currentTarget;
            el.style.height = "";
            el.style.height = Math.min(el.scrollHeight, props.density === "compact" ? 160 : 180) + "px";
          }}
          className={`${props.density === "compact" ? "max-h-[160px]" : "max-h-[180px]"} flex-1 resize-none border-0 bg-transparent px-0 py-0 text-sm leading-6 focus:border-0`}
        />

        <button
          aria-label="Attach file"
          className="grid h-9 w-9 place-items-center rounded-md text-sm text-slate-300 hover:bg-slate-800/60 hover:text-slate-100"
          onClick={() => e.fileInputRef.current?.click()}
          type="button"
          title="Attach"
        >
          {"\u{1F4CE}"}
        </button>
        <div className="mx-1 h-6 w-px bg-slate-800" />
        <button
          aria-label="Send message"
          className={`grid h-9 w-9 place-items-center rounded-md text-sm ${
            e.send.isPending ? "bg-slate-800 text-slate-400" : "flux-btn-primary"
          }`}
          disabled={e.send.isPending}
          type="submit"
          title="Send"
        >
          {"\u{27A4}"}
        </button>
      </div>

      <input
        accept="image/*,application/pdf,text/plain"
        className="hidden"
        multiple
        onChange={(ev) => {
          const files = Array.from(ev.target.files ?? []);
          if (!files.length) return;
          Promise.all(
            files.map(
              (f) =>
                new Promise<{ filename: string; content_type?: string; data_url: string }>((resolve, reject) => {
                  const fr = new FileReader();
                  fr.onerror = () => reject(new Error("read failed"));
                  fr.onload = () =>
                    resolve({
                      filename: f.name,
                      content_type: f.type || undefined,
                      data_url: typeof fr.result === "string" ? fr.result : "",
                    });
                  fr.readAsDataURL(f);
                }),
            ),
          )
            .then((atts) => e.setPendingAttachments((prev) => [...prev, ...atts.filter((a) => a.data_url)]))
            .catch(() => {});
          ev.target.value = "";
        }}
        ref={e.fileInputRef}
        type="file"
      />
    </form>
  );
}

