import type { TextareaHTMLAttributes } from "react";
import { forwardRef } from "react";

export const TextArea = forwardRef<HTMLTextAreaElement, TextareaHTMLAttributes<HTMLTextAreaElement>>(
  function TextArea(props, ref) {
    const { className, ...rest } = props;
    return (
      <textarea
        ref={ref}
        className={
          "flux-input w-full resize-none rounded-md border px-3 py-2 text-sm outline-none " +
          (className ?? "")
        }
        {...rest}
      />
    );
  },
);

