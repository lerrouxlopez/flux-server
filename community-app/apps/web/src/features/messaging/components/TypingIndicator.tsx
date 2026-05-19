export function TypingIndicator(props: { text: string | null | undefined }) {
  if (!props.text) return null;
  return <div className="mt-2 text-xs text-slate-400">{props.text}</div>;
}

