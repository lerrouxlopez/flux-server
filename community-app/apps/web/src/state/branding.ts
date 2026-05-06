import { create } from "zustand";

export type PublicBranding = {
  organization_id: string;
  app_name: string;
  theme: "dark" | "light";
  logo_url?: string | null;
  icon_url?: string | null;
  primary_color?: string | null;
  secondary_color?: string | null;
  privacy_url?: string | null;
  terms_url?: string | null;
  updated_at: string;
};

type BrandingState = {
  branding: PublicBranding | null;
  setBranding: (b: PublicBranding | null) => void;
  loadBranding: (host: string) => Promise<void>;
  loadOrgBranding: (orgId: string) => Promise<void>;
};

export const useBrandingStore = create<BrandingState>((set) => ({
  branding: null,
  setBranding: (branding) => {
    set({ branding });
    applyTheme(branding);
  },
  loadBranding: async (host) => {
    const res = await fetch(`/public/branding?host=${encodeURIComponent(host)}`);
    if (!res.ok) {
      set({ branding: null });
      return;
    }
    const data = (await res.json()) as PublicBranding;
    set({ branding: data });
    if (data.primary_color) {
      document.documentElement.style.setProperty("--brand-primary", data.primary_color);
    }
    if (data.secondary_color) {
      document.documentElement.style.setProperty("--brand-secondary", data.secondary_color);
    }
    applyTheme(data);
  },
  loadOrgBranding: async (orgId) => {
    const token = localStorage.getItem("access_token");
    const res = await fetch(`/orgs/${orgId}/branding`, {
      headers: token ? { Authorization: `Bearer ${token}` } : undefined,
    });
    if (!res.ok) return;
    const data = (await res.json()) as PublicBranding;
    set({ branding: data });
    if (data.primary_color) {
      document.documentElement.style.setProperty("--brand-primary", data.primary_color);
    }
    if (data.secondary_color) {
      document.documentElement.style.setProperty("--brand-secondary", data.secondary_color);
    }
    applyTheme(data);
  },
}));

function applyTheme(data: PublicBranding | null) {
  const root = document.documentElement;
  if (data?.theme === "light") {
    root.classList.add("theme-light");
  } else {
    root.classList.remove("theme-light");
  }
}
