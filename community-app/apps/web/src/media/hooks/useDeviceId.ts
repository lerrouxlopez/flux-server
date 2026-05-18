import { useMemo } from "react";

export function useDeviceId() {
  return useMemo(() => {
    const key = "device_id";
    const existing = localStorage.getItem(key);
    if (existing) return existing;
    const id =
      typeof crypto !== "undefined" && "randomUUID" in crypto
        ? crypto.randomUUID()
        : `dev-${Math.random().toString(16).slice(2)}`;
    localStorage.setItem(key, id);
    return id;
  }, []);
}

