import { useAuthStore } from "../state/auth";

export type ApiError = {
  error: { code: string; message: string };
};

const BACKEND_ORIGIN = (import.meta as any).env?.VITE_BACKEND_ORIGIN as string | undefined;

export async function apiFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const accessToken = useAuthStore.getState().accessToken ?? localStorage.getItem("access_token");
  const headers = new Headers(init?.headers);
  headers.set("Content-Type", "application/json");
  if (accessToken) headers.set("Authorization", `Bearer ${accessToken}`);

  const url = path.startsWith("http://") || path.startsWith("https://") ? path : `${BACKEND_ORIGIN ?? ""}${path}`;
  const res = await fetch(url, { ...init, headers });
  if (res.ok) return (await res.json()) as T;
  const err = (await res.json().catch(() => null)) as ApiError | null;
  const message = err?.error?.message ?? "Request failed";
  throw new Error(message);
}
