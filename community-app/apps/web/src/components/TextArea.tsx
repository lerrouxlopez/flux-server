import type { TextareaHTMLAttributes } from "react";
import { forwardRef } from "react";

export const TextArea = forwardRef<HTMLTextAreaElement, TextareaHTMLAttributes<HTMLTextAreaElement>>(
  function TextArea(props, ref) {
    const { className, ...rest } = props;
    return (
      <textarea
        ref={ref}
        className={
          "w-full resize-none rounded-md border border-slate-800 bg-slate-950/10 px-3 py-2 text-sm text-slate-100 outline-none placeholder:text-slate-500 focus:border-indigo-500 " +
          (className ?? "")
        }
        {...rest}
      />
    );
  },
);

