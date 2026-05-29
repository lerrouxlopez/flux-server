import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import React, { createContext, useCallback, useEffect, useMemo, useState } from "react";
import { apiFetch } from "../../api/client";
import { useAuthStore } from "../../state/auth";
import { applyBrandingToDom, useBrandingStore, type PublicBranding } from "../../state/branding";
import { DEFAULT_THEME_ID, getThemePreset } from "../../branding/presets";
import { useUserThemeStore } from "../../state/userTheme";

export type RawExperienceMode = "work" | "play";
export type ExperienceModeLabel = "Work Mode" | "Game Mode";
export type ExperienceDensity = "comfortable" | "compact";
export type ExperienceMotion = "full" | "reduced";
export type ExperienceColorScheme = "light" | "dark";

export type ExperienceContextValue = {
  rawMode: RawExperienceMode;
  label: ExperienceModeLabel;
  colorScheme: ExperienceColorScheme;
  density: ExperienceDensity;
  motion: ExperienceMotion;
  source: string;
  notificationProfile: string;
  mediaDefaults: {
    room_kind_preference: "meeting" | "voice";
    join_intent: "video" | "voice_only" | "screen_share" | "stage_viewer" | "stage_speaker";
    auto_publish_audio: boolean;
    auto_publish_video: boolean;
    auto_publish_screen: boolean;
    auto_subscribe: boolean;
  };
  featureFlags: Record<string, boolean>;

  /** Current effective theme id. In org context: org-controlled (admin preview may override). Outside org: user's personal theme. */
  themeId: string;
  /** Whether currently within an org context (orgId was explicitly provided). */
  isOrgContext: boolean;
  /** User's personal theme id (applies outside org context). */
  userThemeId: string;
  /** Update the user's personal theme preference. */
  setUserTheme: (id: string) => void;
  /** Temporarily override the full branding (for admin preview). Pass null to clear. */
  previewBranding: (data: PublicBranding | null) => void;

  isLoading: boolean;
  error: string | null;

  setMode: (mode: RawExperienceMode) => void;
  clearModePreference: () => void;
  refetch: () => void;
};

export const ExperienceContext = createContext<ExperienceContextValue | null>(null);

type ExperienceContextResponse = {
  mode: RawExperienceMode;
  source: string;
  density: ExperienceDensity;
  motion: ExperienceMotion;
  notification_profile: string;
  media_defaults: ExperienceContextValue["mediaDefaults"];
  feature_flags: Record<string, boolean>;
};

const LS_MODE_PREFERENCE = "flux_experience_mode_preference";

function normalizeMode(v: unknown): RawExperienceMode | null {
  const m = typeof v === "string" ? v.trim().toLowerCase() : "";
  if (m === "work" || m === "play") return m;
  return null;
}

function modeLabel(m: RawExperienceMode): ExperienceModeLabel {
  return m === "play" ? "Game Mode" : "Work Mode";
}

function defaultDensity(m: RawExperienceMode): ExperienceDensity {
  return m === "play" ? "compact" : "comfortable";
}

function defaultMotion(m: RawExperienceMode): ExperienceMotion {
  return m === "play" ? "reduced" : "full";
}

function defaultMediaDefaults(m: RawExperienceMode): ExperienceContextValue["mediaDefaults"] {
  if (m === "play") {
    return {
      room_kind_preference: "voice",
      join_intent: "voice_only",
      auto_publish_audio: true,
      auto_publish_video: false,
      auto_publish_screen: false,
      auto_subscribe: true,
    };
  }
  return {
    room_kind_preference: "meeting",
    join_intent: "video",
    auto_publish_audio: true,
    auto_publish_video: true,
    auto_publish_screen: false,
    auto_subscribe: true,
  };
}

export function ExperienceProvider(props: {
  orgId: string | null;
  channelId: string | null;
  fallbackOrgId?: string | null;
  children: React.ReactNode;
}) {
  const user = useAuthStore((s) => s.user);
  const qc = useQueryClient();
  const branding = useBrandingStore((s) => s.branding);
  const loadOrgBranding = useBrandingStore((s) => s.loadOrgBranding);
  const userThemeId = useUserThemeStore((s) => s.themeId);
  const setUserTheme = useUserThemeStore((s) => s.setThemeId);

  const [localMode, setLocalMode] = useState<RawExperienceMode>(() => {
    return normalizeMode(localStorage.getItem(LS_MODE_PREFERENCE)) ?? "work";
  });

  // Transient full branding override for admin preview. Takes precedence over Zustand store branding.
  const [previewBrandingData, setPreviewBrandingData] = useState<PublicBranding | null>(null);

  const effectiveOrgId = props.orgId ?? props.fallbackOrgId ?? null;

  // Keep org branding in sync with the active org so theming always follows organization context
  // (including on admin routes where pages may not explicitly load branding).
  useEffect(() => {
    if (!user) return;
    if (!effectiveOrgId) return;
    loadOrgBranding(effectiveOrgId).catch(() => {});
  }, [effectiveOrgId, loadOrgBranding, user]);

  const ctx = useQuery({
    enabled: !!user && !!effectiveOrgId,
    queryKey: ["experience", "context", effectiveOrgId, props.channelId],
    queryFn: () => {
      const qp = new URLSearchParams({ org_id: String(effectiveOrgId) });
      if (props.channelId) qp.set("channel_id", String(props.channelId));
      return apiFetch<ExperienceContextResponse>(`/experience/context?${qp.toString()}`);
    },
    placeholderData: keepPreviousData,
    staleTime: 10_000,
    retry: false,
  });

  const patchPref = useMutation({
    mutationFn: async (patch: { mode_preference?: RawExperienceMode | null }) => {
      return apiFetch<{ status: string }>("/experience/preferences", {
        method: "PATCH",
        body: JSON.stringify(patch),
      });
    },
    onSuccess: async () => {
      await qc.invalidateQueries({ queryKey: ["experience", "context"] });
    },
  });

  const setMode = useCallback(
    (mode: RawExperienceMode) => {
      setLocalMode(mode);
      localStorage.setItem(LS_MODE_PREFERENCE, mode);
      if (user) patchPref.mutate({ mode_preference: mode });
    },
    [patchPref, user],
  );

  const clearModePreference = useCallback(() => {
    localStorage.removeItem(LS_MODE_PREFERENCE);
    setLocalMode("work");
    if (user) patchPref.mutate({ mode_preference: null });
  }, [patchPref, user]);

  const previewBranding = useCallback((data: PublicBranding | null) => {
    setPreviewBrandingData(data);
  }, []);

  const resolvedMode: RawExperienceMode = normalizeMode(ctx.data?.mode) ?? localMode;

  // In org context: org admin controls the theme (preview may override).
  // Outside org context: user's personal theme preference applies.
  const resolvedThemeId: string = previewBrandingData?.ui_theme ??
    (props.orgId !== null ? (branding?.ui_theme ?? DEFAULT_THEME_ID) : userThemeId);

  const value: ExperienceContextValue = useMemo(() => {
    const mode = resolvedMode;
    const mediaDefaults = ctx.data?.media_defaults ?? defaultMediaDefaults(mode);
    return {
      rawMode: mode,
      label: modeLabel(mode),
      colorScheme: getThemePreset(resolvedThemeId).colorScheme,
      density: ctx.data?.density ?? defaultDensity(mode),
      motion: ctx.data?.motion ?? defaultMotion(mode),
      source: ctx.data?.source ?? (ctx.isFetching ? "loading" : "local_preference"),
      notificationProfile: ctx.data?.notification_profile ?? (mode === "play" ? "minimal" : "all"),
      mediaDefaults,
      featureFlags: ctx.data?.feature_flags ?? {},

      themeId: resolvedThemeId,
      isOrgContext: props.orgId !== null,
      userThemeId,
      setUserTheme,
      previewBranding,

      isLoading: ctx.isLoading || patchPref.isPending,
      error: ctx.isError ? (ctx.error as Error).message : null,

      setMode,
      clearModePreference,
      refetch: () => ctx.refetch(),
    };
  }, [
    resolvedMode,
    resolvedThemeId,
    props.orgId,
    userThemeId,
    setUserTheme,
    ctx.data,
    ctx.error,
    ctx.isError,
    ctx.isFetching,
    ctx.isLoading,
    ctx.refetch,
    patchPref.isPending,
    setMode,
    previewBranding,
    clearModePreference,
  ]);

  // Apply theme to DOM.
  // Priority: admin preview > org branding (in org context) > user personal theme (global pages).
  useEffect(() => {
    if (previewBrandingData) {
      applyBrandingToDom(previewBrandingData);
    } else if (props.orgId !== null) {
      applyBrandingToDom(branding);
    } else {
      applyBrandingToDom(null, { themeId: userThemeId });
    }
  }, [branding, previewBrandingData, props.orgId, userThemeId]);

  // Sync mode to DOM separately — mode is a feature toggle, not a visual theme.
  useEffect(() => {
    document.documentElement.dataset.uiMode = resolvedMode;
  }, [resolvedMode]);

  return <ExperienceContext.Provider value={value}>{props.children}</ExperienceContext.Provider>;
}
