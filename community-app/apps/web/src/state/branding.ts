import { create } from "zustand";
import { getThemePreset } from "../branding/presets";
import { apiFetch } from "../api/client";

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
  },
  loadOrgBranding: async (orgId) => {
    const url = `${BACKEND_ORIGIN ?? ""}/orgs/${orgId}/branding`;
    const data = await apiFetch<PublicBranding>(url);
    set({ branding: data });
  },
}));

export function applyBrandingToDom(
  data: PublicBranding | null,
  opts?: { themeId?: string },
) {
  const root = document.documentElement;
  root.classList.add("branded");

  // Priority: user preference (themeId) > org suggestion (data.ui_theme) > default
  const preset = getThemePreset(opts?.themeId ?? data?.ui_theme ?? undefined);

  // When a user theme is explicitly requested, use preset palette exclusively.
  // Org's saved custom colors only apply when no user preference is active (org-level branding).
  const c = opts?.themeId ? null : data;

  // uiMode is intentionally NOT set here — ExperienceProvider owns data-ui-mode.
  root.dataset.uiTheme = preset.id;
  root.dataset.colorScheme = preset.colorScheme;

  const brandPrimary = c?.primary_color ?? preset.vars.brandPrimary;
  const brandSecondary = c?.secondary_color ?? preset.vars.brandSecondary;
  root.style.setProperty("--brand-primary", brandPrimary);
  root.style.setProperty("--brand-secondary", brandSecondary);
  root.style.setProperty("--flux-on-accent", preset.vars.onPrimary);

  root.style.setProperty("--app-bg", c?.bg_color ?? preset.vars.appBg);
  root.style.setProperty("--app-surface", c?.surface_color ?? preset.vars.appSurface);
  root.style.setProperty("--app-text", c?.text_color ?? preset.vars.appText);
  root.style.setProperty("--app-muted", c?.muted_color ?? preset.vars.appMuted);
  root.style.setProperty("--app-border", c?.border_color ?? preset.vars.appBorder);

  // Extra UI tokens
  root.style.setProperty("--selection-bg", c?.selection_bg ?? brandPrimary);
  root.style.setProperty("--selection-text", c?.selection_text ?? preset.vars.appText);
  root.style.setProperty("--dropdown-bg", c?.dropdown_bg ?? (c?.surface_color ?? preset.vars.appSurface));
  root.style.setProperty("--dropdown-text", c?.dropdown_text ?? (c?.text_color ?? preset.vars.appText));
  root.style.setProperty("--chat-bubble-me-bg", c?.chat_bubble_me_bg ?? brandPrimary);
  root.style.setProperty("--chat-bubble-me-text", c?.chat_bubble_me_text ?? preset.vars.onPrimary);
  root.style.setProperty("--chat-bubble-other-bg", c?.chat_bubble_other_bg ?? (c?.surface_color ?? preset.vars.appSurface));
  root.style.setProperty("--chat-bubble-other-text", c?.chat_bubble_other_text ?? (c?.text_color ?? preset.vars.appText));
}
