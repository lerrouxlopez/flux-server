function initials(input: string): string {
  const v = input.trim();
  if (!v) return "?";
  const parts = v.split(/\s+/).filter(Boolean);
  const a = parts[0]?.[0] ?? "?";
  const b = parts.length > 1 ? parts[parts.length - 1]?.[0] ?? "" : v[1] ?? "";
  return (a + b).toUpperCase();
}

export function Avatar(props: {
  name: string;
  src?: string | null;
  size?: number;
  className?: string;
  online?: boolean;
}) {
  const size = props.size ?? 32;
  const letter = initials(props.name);

  return (
    <div className={`relative shrink-0 ${props.className ?? ""}`} style={{ width: size, height: size }}>
      {props.src ? (
        <img
          alt={props.name}
          className="h-full w-full rounded-full object-cover"
          referrerPolicy="no-referrer"
          src={props.src}
        />
      ) : (
        <div className="grid h-full w-full place-items-center rounded-full bg-slate-700 text-xs font-semibold text-slate-100">
          {letter}
        </div>
      )}
      {typeof props.online === "boolean" ? (
        <span
          className={`absolute bottom-0 right-0 h-2.5 w-2.5 rounded-full ring-2 ring-slate-950 ${
            props.online ? "bg-emerald-400" : "bg-slate-500"
          }`}
        />
      ) : null}
    </div>
  );
}

