import { create } from "zustand";

export type User = {
  id: string;
  email: string;
  display_name: string;
  created_at: string;
};

type AuthState = {
  accessToken: string | null;
  refreshToken: string | null;
  user: User | null;
  setTokens: (access: string, refresh: string) => void;
  clear: () => void;
  logout: () => Promise<void>;
  hydrate: () => void;
  loadMe: () => Promise<void>;
};

const LS_ACCESS = "access_token";
const LS_REFRESH = "refresh_token";

export const useAuthStore = create<AuthState>((set, get) => ({
  accessToken: null,
  refreshToken: null,
  user: null,
  setTokens: (access, refresh) => {
    localStorage.setItem(LS_ACCESS, access);
    localStorage.setItem(LS_REFRESH, refresh);
    set({ accessToken: access, refreshToken: refresh });
  },
  clear: () => {
    localStorage.removeItem(LS_ACCESS);
    localStorage.removeItem(LS_REFRESH);
    set({ accessToken: null, refreshToken: null, user: null });
  },
  logout: async () => {
    const refreshToken = get().refreshToken ?? localStorage.getItem(LS_REFRESH);
    if (refreshToken) {
      await fetch("/auth/logout", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ refresh_token: refreshToken }),
      }).catch(() => {});
    }
    get().clear();
  },
  hydrate: () => {
    set({
      accessToken: localStorage.getItem(LS_ACCESS),
      refreshToken: localStorage.getItem(LS_REFRESH),
    });
  },
  loadMe: async () => {
    const accessToken = get().accessToken ?? localStorage.getItem(LS_ACCESS);
    if (!accessToken) {
      set({ user: null });
      return;
    }
    const res = await fetch("/auth/me", {
      headers: { Authorization: `Bearer ${accessToken}` },
    });
    if (!res.ok) {
      // Token is stale/invalid.
      get().clear();
      return;
    }
    const user = (await res.json()) as User;
    set({ user });
  },
}));
