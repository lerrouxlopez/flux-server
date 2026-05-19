import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import React, { createContext, useCallback, useEffect, useMemo, useState } from "react";
import { apiFetch } from "../../api/client";
import { useAuthStore } from "../../state/auth";
import { applyBrandingToDom, useBrandingStore } from "../../state/branding";

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

function defaultColorScheme(m: RawExperienceMode): ExperienceColorScheme {
  return m === "play" ? "dark" : "light";
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

  const [localMode, setLocalMode] = useState<RawExperienceMode>(() => {
    return normalizeMode(localStorage.getItem(LS_MODE_PREFERENCE)) ?? "work";
  });

  const effectiveOrgId = props.orgId ?? props.fallbackOrgId ?? null;

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
    mutationFn: async (mode: RawExperienceMode | null) => {
      return apiFetch<{ status: string; mode_preference: RawExperienceMode | null }>("/experience/preferences", {
        method: "PATCH",
        body: JSON.stringify({ mode_preference: mode }),
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
      if (user) patchPref.mutate(mode);
    },
    [patchPref, user],
  );

  const clearModePreference = useCallback(() => {
    localStorage.removeItem(LS_MODE_PREFERENCE);
    setLocalMode("work");
    if (user) patchPref.mutate(null);
  }, [patchPref, user]);

  const resolvedMode: RawExperienceMode = normalizeMode(ctx.data?.mode) ?? localMode;

  const value: ExperienceContextValue = useMemo(() => {
    const mode = resolvedMode;
    const mediaDefaults = ctx.data?.media_defaults ?? defaultMediaDefaults(mode);
    return {
      rawMode: mode,
      label: modeLabel(mode),
      colorScheme: defaultColorScheme(mode),
      density: ctx.data?.density ?? defaultDensity(mode),
      motion: ctx.data?.motion ?? defaultMotion(mode),
      source: ctx.data?.source ?? (ctx.isFetching ? "loading" : "local_preference"),
      notificationProfile: ctx.data?.notification_profile ?? (mode === "play" ? "minimal" : "all"),
      mediaDefaults,
      featureFlags: ctx.data?.feature_flags ?? {},

      isLoading: ctx.isLoading || patchPref.isPending,
      error: ctx.isError ? (ctx.error as Error).message : null,

      setMode,
      clearModePreference,
      refetch: () => ctx.refetch(),
    };
  }, [
    resolvedMode,
    ctx.data,
    ctx.error,
    ctx.isError,
    ctx.isFetching,
    ctx.isLoading,
    ctx.refetch,
    patchPref.isPending,
    setMode,
    clearModePreference,
  ]);

  useEffect(() => {
    applyBrandingToDom(branding, { uiMode: value.rawMode });
  }, [branding, value.rawMode]);

  return <ExperienceContext.Provider value={value}>{props.children}</ExperienceContext.Provider>;
}
