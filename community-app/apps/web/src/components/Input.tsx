import { clsx } from "clsx";
import type { InputHTMLAttributes } from "react";

export function Input(props: InputHTMLAttributes<HTMLInputElement>) {
  const { className, ...rest } = props;
  return (
    <input
      className={clsx(
        "w-full rounded-md border border-slate-800 bg-slate-900 px-3 py-2 text-sm outline-none focus:border-indigo-500",
        className,
      )}
      {...rest}
    />
  );
}

