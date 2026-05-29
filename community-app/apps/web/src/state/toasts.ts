import { create } from "zustand";

export type Toast = {
  id: string;
  title: string;
  message?: string | null;
  createdAt: number;
};

type ToastState = {
  toasts: Toast[];
  push: (t: Omit<Toast, "id" | "createdAt"> & { id?: string; createdAt?: number }) => void;
  remove: (id: string) => void;
  clear: () => void;
};

export const useToastStore = create<ToastState>((set) => ({
  toasts: [],
  push: (t) => {
    const id = t.id ?? `toast-${Date.now()}-${Math.random().toString(16).slice(2)}`;
    const createdAt = t.createdAt ?? Date.now();
    set((s) => ({ toasts: [{ id, createdAt, title: t.title, message: t.message ?? null }, ...s.toasts].slice(0, 4) }));
  },
  remove: (id) => set((s) => ({ toasts: s.toasts.filter((t) => t.id !== id) })),
  clear: () => set({ toasts: [] }),
}));

