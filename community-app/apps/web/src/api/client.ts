import { useAuthStore } from "../state/auth";

export type ApiError = {
  error: { code: string; message: string };
};

export async function apiFetch<T>(path: string, init?: RequestInit): Promise<T> {
  const accessToken = useAuthStore.getState().accessToken ?? localStorage.getItem("access_token");
  const headers = new Headers(init?.headers);
  headers.set("Content-Type", "application/json");
  if (accessToken) headers.set("Authorization", `Bearer ${accessToken}`);
  const res = await fetch(path, { ...init, headers });
  if (res.ok) return (await res.json()) as T;
  const err = (await res.json().catch(() => null)) as ApiError | null;
  const message = err?.error?.message ?? "Request failed";
  throw new Error(message);
}

