import { create } from "zustand";

export type PublicBranding = {
  organization_id: string;
  app_name: string;
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
  loadBranding: (host: string) => Promise<void>;
};

export const useBrandingStore = create<BrandingState>((set) => ({
  branding: null,
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
  },
}));

