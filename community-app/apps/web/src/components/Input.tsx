import { clsx } from "clsx";
import type { InputHTMLAttributes } from "react";

export function Input(props: InputHTMLAttributes<HTMLInputElement>) {
  const { className, ...rest } = props;
  return (
    <input
      className={clsx(
        "flux-input w-full rounded-md border px-3 py-2 text-sm outline-none",
        className,
      )}
      {...rest}
    />
  );
}

