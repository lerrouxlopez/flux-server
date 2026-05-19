import { create } from "zustand";
import { getThemePreset, type UIMode } from "../branding/presets";

const BACKEND_ORIGIN = (import.meta as any).env?.VITE_BACKEND_ORIGIN as string | undefined;

export type PublicBranding = {
  organization_id: string;
  app_name: string;
  theme: "dark" | "light";
  ui_mode?: "work" | "play";
  ui_theme?: string;
  logo_url?: string | null;
  icon_url?: string | null;
  primary_color?: string | null;
  secondary_color?: string | null;
  bg_color?: string | null;
  surface_color?: string | null;
  text_color?: string | null;
  muted_color?: string | null;
  border_color?: string | null;
  selection_bg?: string | null;
  selection_text?: string | null;
  dropdown_bg?: string | null;
  dropdown_text?: string | null;
  chat_bubble_me_bg?: string | null;
  chat_bubble_me_text?: string | null;
  chat_bubble_other_bg?: string | null;
  chat_bubble_other_text?: string | null;
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
    applyBrandingToDom(branding);
  },
  loadBranding: async (host) => {
    const url = `${BACKEND_ORIGIN ?? ""}/public/branding?host=${encodeURIComponent(host)}`;
    const res = await fetch(url);
    if (!res.ok) {
      set({ branding: null });
      return;
    }
    const data = (await res.json()) as PublicBranding;
    set({ branding: data });
    applyBrandingToDom(data);
  },
  loadOrgBranding: async (orgId) => {
    const token = localStorage.getItem("access_token");
    const url = `${BACKEND_ORIGIN ?? ""}/orgs/${orgId}/branding`;
    const res = await fetch(url, {
      headers: token ? { Authorization: `Bearer ${token}` } : undefined,
    });
    if (!res.ok) return;
    const data = (await res.json()) as PublicBranding;
    set({ branding: data });
    applyBrandingToDom(data);
  },
}));

export function applyBrandingToDom(
  data: PublicBranding | null,
  opts?: { uiMode?: UIMode; uiTheme?: string | undefined },
) {
  const root = document.documentElement;
  // Branded variables (used by CSS skin overrides).
  root.classList.add("branded");
  const preset = getThemePreset(opts?.uiMode ?? data?.ui_mode, opts?.uiTheme ?? data?.ui_theme);
  root.dataset.uiMode = preset.mode;
  root.dataset.uiTheme = preset.id;
  root.dataset.colorScheme = preset.colorScheme;

  const brandPrimary = data?.primary_color ?? preset.vars.brandPrimary;
  const brandSecondary = data?.secondary_color ?? preset.vars.brandSecondary;
  root.style.setProperty("--brand-primary", brandPrimary);
  root.style.setProperty("--brand-secondary", brandSecondary);

  root.style.setProperty("--app-bg", data?.bg_color ?? preset.vars.appBg);
  root.style.setProperty("--app-surface", data?.surface_color ?? preset.vars.appSurface);
  root.style.setProperty("--app-text", data?.text_color ?? preset.vars.appText);
  root.style.setProperty("--app-muted", data?.muted_color ?? preset.vars.appMuted);
  root.style.setProperty("--app-border", data?.border_color ?? preset.vars.appBorder);

  // Extra UI tokens
  root.style.setProperty("--selection-bg", data?.selection_bg ?? brandPrimary);
  root.style.setProperty("--selection-text", data?.selection_text ?? preset.vars.appText);
  root.style.setProperty("--dropdown-bg", data?.dropdown_bg ?? (data?.surface_color ?? preset.vars.appSurface));
  root.style.setProperty("--dropdown-text", data?.dropdown_text ?? (data?.text_color ?? preset.vars.appText));
  root.style.setProperty("--chat-bubble-me-bg", data?.chat_bubble_me_bg ?? brandPrimary);
  root.style.setProperty("--chat-bubble-me-text", data?.chat_bubble_me_text ?? "#ffffff");
  root.style.setProperty("--chat-bubble-other-bg", data?.chat_bubble_other_bg ?? (data?.surface_color ?? preset.vars.appSurface));
  root.style.setProperty("--chat-bubble-other-text", data?.chat_bubble_other_text ?? (data?.text_color ?? preset.vars.appText));
}
